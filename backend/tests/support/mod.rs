//! Shared test fixtures and mocks for integration/unit tests.
#![allow(dead_code)]

use anyhow::Result;
use async_trait::async_trait;
use chaos_bot_backend::application::agent::{AgentConfig, AgentLoop};
use chaos_bot_backend::interface::api::AppState;
use chaos_bot_backend::infrastructure::config::{
    write_config_file, AgentFileConfig, AgentLlmConfig, AgentLoggingConfig, AgentSecretsConfig,
    AgentServerConfig, AppConfig, EnvSecrets,
};
use chaos_bot_backend::runtime::config_runtime::{AgentFactory, ConfigRuntime, RestartMode};
use chaos_bot_backend::infrastructure::model::{LlmProvider, LlmRequest, LlmResponse, LlmStream, LlmStreamEvent};
use chaos_bot_backend::infrastructure::memory::{MemoryBackend, MemoryStore};
use chaos_bot_backend::infrastructure::personality::{PersonalityLoader, PersonalitySource};
use chaos_bot_backend::infrastructure::tooling::{Tool, ToolContext, ToolRegistry};
use chaos_bot_backend::domain::types::{Message, ToolCall, ToolExecution, ToolSpec, Usage};
use futures::stream;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// MockStreamProvider
// ---------------------------------------------------------------------------

/// Configurable LlmProvider that returns pre-programmed sequences of
/// LlmStreamEvent per iteration (one Vec per agent loop iteration).
pub struct MockStreamProvider {
    /// Each outer vec is one call to chat_stream; inner vec is the event sequence.
    responses: Mutex<Vec<Vec<LlmStreamEvent>>>,
    /// Captured requests for assertions.
    pub captured: Mutex<Vec<LlmRequest>>,
}

impl MockStreamProvider {
    /// Create a provider that will return the given event sequences in order.
    /// Each call to `chat_stream` pops the first entry.
    pub fn new(responses: Vec<Vec<LlmStreamEvent>>) -> Self {
        Self {
            responses: Mutex::new(responses),
            captured: Mutex::new(Vec::new()),
        }
    }

    /// Convenience: single iteration with a text reply and done.
    pub fn text(reply: &str) -> Self {
        Self::new(vec![vec![
            LlmStreamEvent {
                delta: reply.to_string(),
                tool_call: None,
                done: false,
                usage: None,
            },
            LlmStreamEvent {
                delta: String::new(),
                tool_call: None,
                done: true,
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                }),
            },
        ]])
    }

    /// Convenience: first iteration returns a tool call, second returns text.
    pub fn tool_then_text(tool_call: ToolCall, reply: &str) -> Self {
        Self::new(vec![
            vec![
                LlmStreamEvent {
                    delta: String::new(),
                    tool_call: Some(tool_call),
                    done: false,
                    usage: None,
                },
                LlmStreamEvent {
                    delta: String::new(),
                    tool_call: None,
                    done: true,
                    usage: None,
                },
            ],
            vec![
                LlmStreamEvent {
                    delta: reply.to_string(),
                    tool_call: None,
                    done: false,
                    usage: None,
                },
                LlmStreamEvent {
                    delta: String::new(),
                    tool_call: None,
                    done: true,
                    usage: Some(Usage {
                        prompt_tokens: 20,
                        completion_tokens: 10,
                        total_tokens: 30,
                    }),
                },
            ],
        ])
    }

    /// Provider that always returns an error.
    pub fn error(_msg: &str) -> Self {
        // We use an empty responses vec and override chat_stream below won't
        // actually be reached — we need a custom impl. Instead, use a different
        // approach: store a special sentinel.
        Self {
            responses: Mutex::new(Vec::new()),
            captured: Mutex::new(vec![]),
        }
    }
}

#[async_trait]
impl LlmProvider for MockStreamProvider {
    fn name(&self) -> &'static str {
        "mock-stream"
    }

    async fn chat(&self, _request: LlmRequest) -> Result<LlmResponse> {
        anyhow::bail!("MockStreamProvider does not support non-streaming chat")
    }

    async fn chat_stream(&self, request: LlmRequest) -> Result<LlmStream> {
        self.captured.lock().unwrap().push(request);
        let events = {
            let mut responses = self.responses.lock().unwrap();
            if responses.is_empty() {
                return Err(anyhow::anyhow!("mock provider exhausted or error"));
            }
            responses.remove(0)
        };
        let items: Vec<Result<LlmStreamEvent>> = events.into_iter().map(Ok).collect();
        Ok(Box::pin(stream::iter(items)))
    }
}

// ---------------------------------------------------------------------------
// ErrorProvider — always errors on chat_stream
// ---------------------------------------------------------------------------

pub struct ErrorProvider {
    pub message: String,
}

impl ErrorProvider {
    pub fn new(msg: &str) -> Self {
        Self {
            message: msg.to_string(),
        }
    }
}

#[async_trait]
impl LlmProvider for ErrorProvider {
    fn name(&self) -> &'static str {
        "error"
    }

    async fn chat(&self, _request: LlmRequest) -> Result<LlmResponse> {
        Err(anyhow::anyhow!("{}", self.message))
    }

    async fn chat_stream(&self, _request: LlmRequest) -> Result<LlmStream> {
        Err(anyhow::anyhow!("{}", self.message))
    }
}

// ---------------------------------------------------------------------------
// MockTool
// ---------------------------------------------------------------------------

/// A configurable Tool implementation backed by a closure.
pub struct MockTool {
    tool_name: &'static str,
    tool_description: &'static str,
    schema: Value,
    handler: Box<dyn Fn(Value) -> Result<ToolExecution> + Send + Sync>,
}

impl MockTool {
    pub fn new(
        name: &'static str,
        handler: impl Fn(Value) -> Result<ToolExecution> + Send + Sync + 'static,
    ) -> Self {
        Self {
            tool_name: name,
            tool_description: "mock tool",
            schema: json!({"type": "object", "properties": {}}),
            handler: Box::new(handler),
        }
    }

    pub fn fixed(name: &'static str, output: &str) -> Self {
        let output = output.to_string();
        Self::new(name, move |_| {
            Ok(ToolExecution {
                name: name.to_string(),
                output: output.clone(),
                is_error: false,
            })
        })
    }
}

#[async_trait]
impl Tool for MockTool {
    fn name(&self) -> &'static str {
        self.tool_name
    }

    fn description(&self) -> &'static str {
        self.tool_description
    }

    fn parameters_schema(&self) -> Value {
        self.schema.clone()
    }

    async fn execute(&self, args: Value, _context: &ToolContext) -> Result<ToolExecution> {
        (self.handler)(args)
    }
}

pub struct MockPersonality {
    prompt: String,
}

impl MockPersonality {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
        }
    }
}

#[async_trait]
impl PersonalitySource for MockPersonality {
    async fn system_prompt(&self) -> Result<String> {
        Ok(self.prompt.clone())
    }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn default_agent_config(working_dir: PathBuf) -> AgentConfig {
    AgentConfig {
        model: "mock-model".to_string(),
        temperature: 0.0,
        max_tokens: 128,
        max_iterations: 6,
        token_budget: 4096,
        working_dir,
    }
}

/// Create a tempdir-backed test environment returning (TempDir, AppState).
/// The TempDir must be held alive for the duration of the test.
pub fn build_test_state(provider: Arc<dyn LlmProvider>) -> (TempDir, AppState) {
    build_test_state_with_registry(provider, ToolRegistry::new())
}

pub fn build_test_state_with_registry(
    provider: Arc<dyn LlmProvider>,
    registry: ToolRegistry,
) -> (TempDir, AppState) {
    let temp = tempfile::tempdir().expect("tempdir");

    let memory_dir = temp.path().join("memory");
    let memory_file = temp.path().join("MEMORY.md");
    let memory: Arc<dyn MemoryBackend> = Arc::new(MemoryStore::new(&memory_dir, &memory_file));
    let personality: Arc<dyn PersonalitySource> =
        Arc::new(MockPersonality::new("## SOUL.md\nYou are a test bot."));

    let agent = AgentLoop::new(
        provider,
        Arc::new(registry),
        personality,
        memory,
        default_agent_config(temp.path().to_path_buf()),
    );

    let state = AppState::new(Arc::new(agent));

    (temp, state)
}

pub fn build_test_agent(provider: Arc<dyn LlmProvider>) -> (TempDir, AgentLoop) {
    build_test_agent_with_registry(provider, ToolRegistry::new())
}

pub fn build_test_agent_with_registry(
    provider: Arc<dyn LlmProvider>,
    registry: ToolRegistry,
) -> (TempDir, AgentLoop) {
    let temp = tempfile::tempdir().expect("tempdir");

    let memory_dir = temp.path().join("memory");
    let memory_file = temp.path().join("MEMORY.md");
    let memory: Arc<dyn MemoryBackend> = Arc::new(MemoryStore::new(&memory_dir, &memory_file));
    let personality: Arc<dyn PersonalitySource> =
        Arc::new(MockPersonality::new("## SOUL.md\nYou are a test bot."));

    let agent = AgentLoop::new(
        provider,
        Arc::new(registry),
        personality,
        memory,
        default_agent_config(temp.path().to_path_buf()),
    );

    (temp, agent)
}

struct RuntimeTestAgentFactory;

#[async_trait]
impl AgentFactory for RuntimeTestAgentFactory {
    async fn build_agent(&self, config: &AppConfig) -> Result<Arc<AgentLoop>> {
        let memory: Arc<dyn MemoryBackend> = Arc::new(MemoryStore::new(
            config.memory_dir.clone(),
            config.memory_file.clone(),
        ));
        memory.ensure_layout().await?;

        std::fs::create_dir_all(&config.personality_dir)?;
        if !config.personality_dir.join("SOUL.md").exists() {
            std::fs::write(
                config.personality_dir.join("SOUL.md"),
                "# Soul\nTest soul content.",
            )?;
        }
        let personality: Arc<dyn PersonalitySource> =
            Arc::new(PersonalityLoader::new(config.personality_dir.clone()));

        let agent = AgentLoop::new(
            Arc::new(MockStreamProvider::text("runtime factory")),
            Arc::new(ToolRegistry::new()),
            personality,
            memory,
            AgentConfig {
                model: config.model.clone(),
                temperature: config.temperature,
                max_tokens: config.max_tokens,
                max_iterations: config.max_iterations,
                token_budget: config.token_budget,
                working_dir: config.working_dir.clone(),
            },
        );

        Ok(Arc::new(agent))
    }
}

pub async fn build_test_state_with_config_runtime() -> (TempDir, AppState, PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_base = temp.path().join("home");
    std::fs::create_dir_all(&workspace_base).expect("create home");

    let config_file = AgentFileConfig {
        workspace: Some(PathBuf::from("./runtime")),
        logging: AgentLoggingConfig {
            level: Some("info".to_string()),
            retention_days: Some(7),
            directory: Some(PathBuf::from("logs")),
        },
        server: AgentServerConfig {
            host: Some("127.0.0.1".to_string()),
            port: Some(3010),
        },
        llm: AgentLlmConfig {
            provider: Some("mock".to_string()),
            model: Some("mock-model".to_string()),
            temperature: Some(0.2),
            max_tokens: Some(1024),
            max_iterations: Some(6),
            token_budget: Some(12000),
        },
        secrets: AgentSecretsConfig::default(),
    };

    let mut app_config = AppConfig::from_inputs(
        config_file.clone(),
        EnvSecrets::default(),
        workspace_base.clone(),
    );
    let config_path = workspace_base.join(".chaos-bot/config.json");
    app_config.config_path = config_path.clone();
    write_config_file(&config_path, &config_file).expect("write config");

    let factory: Arc<dyn AgentFactory> = Arc::new(RuntimeTestAgentFactory);
    let initial_agent = factory
        .build_agent(&app_config)
        .await
        .expect("build initial agent");
    let agent_slot = Arc::new(RwLock::new(initial_agent));

    let runtime = Arc::new(ConfigRuntime::new(
        agent_slot.clone(),
        factory,
        config_file,
        app_config,
        workspace_base,
        config_path.clone(),
        RestartMode::Disabled,
    ));

    let state = AppState::with_config_runtime(agent_slot, runtime);
    (temp, state, config_path)
}

pub fn temp_personality_dir() -> (TempDir, PersonalityLoader) {
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path().join("personality");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SOUL.md"), "# Soul\nTest soul content.").unwrap();
    std::fs::write(dir.join("IDENTITY.md"), "# Identity\nTest identity.").unwrap();
    (temp, PersonalityLoader::new(dir))
}

pub fn temp_memory_store() -> (TempDir, MemoryStore) {
    let temp = tempfile::tempdir().expect("tempdir");
    let memory_dir = temp.path().join("memory");
    let memory_file = temp.path().join("MEMORY.md");
    (temp, MemoryStore::new(memory_dir, memory_file))
}

pub fn sample_messages() -> Vec<Message> {
    vec![
        Message::system("You are a test assistant."),
        Message::user("Hello"),
        Message::assistant("Hi there!"),
    ]
}

pub fn sample_tool_call() -> ToolCall {
    ToolCall {
        id: "tc_1".to_string(),
        name: "read".to_string(),
        arguments: json!({"path": "test.txt"}),
    }
}

pub fn sample_tool_spec() -> ToolSpec {
    ToolSpec {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
        parameters_schema: json!({"type": "object", "properties": {"input": {"type": "string"}}}),
    }
}
