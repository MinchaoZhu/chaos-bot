use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chaos_bot_backend::application::{AgentConfig, AgentLoop};
use chaos_bot_backend::domain::ports::{
    MemoryHit, MemoryPort, ModelPort, ModelRequest, ModelResponse, ModelStream, ModelStreamEvent,
    SkillPort, ToolExecutionContext,
};
use chaos_bot_backend::domain::types::{ToolCall, ToolExecution, ToolSpec, Usage};
use chaos_bot_backend::infrastructure::personality::PersonalitySource;
use chaos_bot_backend::infrastructure::skills::EmptySkillStore;
use chaos_bot_backend::infrastructure::tooling::{Tool, ToolRegistry};
use futures::stream;
use serde_json::Value;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

#[derive(Clone)]
pub struct MockStreamProvider {
    responses: Arc<Mutex<VecDeque<Vec<ModelStreamEvent>>>>,
}

impl MockStreamProvider {
    pub fn new(responses: Vec<Vec<ModelStreamEvent>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
        }
    }

    pub fn text(text: &str) -> Self {
        Self::new(vec![vec![
            ModelStreamEvent {
                delta: text.to_string(),
                tool_call: None,
                done: false,
                usage: None,
            },
            ModelStreamEvent {
                delta: String::new(),
                tool_call: None,
                done: true,
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                }),
            },
        ]])
    }

    pub fn tool_then_text(tool_call: ToolCall, text: &str) -> Self {
        Self::new(vec![
            vec![
                ModelStreamEvent {
                    delta: String::new(),
                    tool_call: Some(tool_call),
                    done: false,
                    usage: None,
                },
                ModelStreamEvent {
                    delta: String::new(),
                    tool_call: None,
                    done: true,
                    usage: Some(Usage {
                        prompt_tokens: 10,
                        completion_tokens: 5,
                        total_tokens: 15,
                    }),
                },
            ],
            vec![
                ModelStreamEvent {
                    delta: text.to_string(),
                    tool_call: None,
                    done: false,
                    usage: None,
                },
                ModelStreamEvent {
                    delta: String::new(),
                    tool_call: None,
                    done: true,
                    usage: Some(Usage {
                        prompt_tokens: 12,
                        completion_tokens: 8,
                        total_tokens: 20,
                    }),
                },
            ],
        ])
    }
}

#[async_trait]
impl ModelPort for MockStreamProvider {
    fn name(&self) -> &'static str {
        "mock-stream"
    }

    async fn chat(&self, _request: ModelRequest) -> Result<ModelResponse> {
        Err(anyhow!("chat() not used in these tests"))
    }

    async fn chat_stream(&self, _request: ModelRequest) -> Result<ModelStream> {
        let events = self
            .responses
            .lock()
            .expect("responses lock")
            .pop_front()
            .unwrap_or_default()
            .into_iter()
            .map(Ok)
            .collect::<Vec<_>>();
        Ok(Box::pin(stream::iter(events)))
    }
}

pub struct ErrorProvider {
    message: String,
}

impl ErrorProvider {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

#[async_trait]
impl ModelPort for ErrorProvider {
    fn name(&self) -> &'static str {
        "error-provider"
    }

    async fn chat(&self, _request: ModelRequest) -> Result<ModelResponse> {
        Err(anyhow!(self.message.clone()))
    }

    async fn chat_stream(&self, _request: ModelRequest) -> Result<ModelStream> {
        Err(anyhow!(self.message.clone()))
    }
}

pub struct MockTool {
    spec: ToolSpec,
    output: String,
}

impl MockTool {
    pub fn fixed(name: &str, output: &str) -> Self {
        Self {
            spec: ToolSpec {
                name: name.to_string(),
                description: "mock tool".to_string(),
                parameters_schema: serde_json::json!({"type":"object"}),
            },
            output: output.to_string(),
        }
    }
}

#[async_trait]
impl Tool for MockTool {
    fn name(&self) -> &'static str {
        "mock_tool"
    }

    fn description(&self) -> &'static str {
        "mock tool"
    }

    fn parameters_schema(&self) -> Value {
        self.spec.parameters_schema.clone()
    }

    async fn execute(
        &self,
        _args: Value,
        _context: &ToolExecutionContext,
    ) -> Result<ToolExecution> {
        Ok(ToolExecution {
            name: self.spec.name.clone(),
            output: self.output.clone(),
            is_error: false,
        })
    }
}

#[derive(Default)]
struct TestMemory;

#[async_trait]
impl MemoryPort for TestMemory {
    async fn search(&self, _keyword: &str) -> Result<Vec<MemoryHit>> {
        Ok(Vec::new())
    }

    async fn append_daily_log(&self, _summary: &str) -> Result<PathBuf> {
        Ok(PathBuf::from("memory.log"))
    }

    async fn get_file(
        &self,
        _relative_path: &str,
        _start_line: Option<usize>,
        _end_line: Option<usize>,
    ) -> Result<String> {
        Ok(String::new())
    }

    async fn read_curated(&self) -> Result<String> {
        Ok(String::new())
    }

    async fn write_curated(&self, _content: &str) -> Result<()> {
        Ok(())
    }

    async fn ensure_layout(&self) -> Result<()> {
        Ok(())
    }
}

struct TestPersonality;

#[async_trait]
impl PersonalitySource for TestPersonality {
    async fn system_prompt(&self) -> Result<String> {
        Ok("You are helpful.".to_string())
    }
}

pub fn build_test_agent(provider: Arc<dyn ModelPort>) -> (TempDir, AgentLoop) {
    build_test_agent_with_registry(provider, ToolRegistry::new())
}

pub fn build_test_agent_with_registry(
    provider: Arc<dyn ModelPort>,
    registry: ToolRegistry,
) -> (TempDir, AgentLoop) {
    let temp = tempfile::tempdir().expect("tempdir");
    let memory: Arc<dyn MemoryPort> = Arc::new(TestMemory);
    let personality: Arc<dyn PersonalitySource> = Arc::new(TestPersonality);
    let tools = Arc::new(registry);
    let skills: Arc<dyn SkillPort> = Arc::new(EmptySkillStore);

    let agent = AgentLoop::new(
        provider,
        tools,
        personality,
        memory,
        skills,
        AgentConfig {
            model: "mock-model".to_string(),
            temperature: 0.0,
            max_tokens: 256,
            max_iterations: 6,
            token_budget: 2048,
            working_dir: temp.path().to_path_buf(),
        },
    );

    (temp, agent)
}
