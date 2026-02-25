use chaos_bot_backend::application::agent::{AgentConfig, AgentLoop};
use chaos_bot_backend::infrastructure::model::{LlmProvider, LlmRequest, LlmResponse, LlmStream, LlmStreamEvent};
use chaos_bot_backend::infrastructure::memory::{MemoryBackend, MemoryStore};
use chaos_bot_backend::infrastructure::personality::{PersonalityLoader, PersonalitySource};
use chaos_bot_backend::infrastructure::tooling::ToolRegistry;
use chaos_bot_backend::domain::types::{Role, SessionState};
use futures::stream;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

#[derive(Clone, Default)]
struct CaptureProvider {
    request: Arc<Mutex<Option<LlmRequest>>>,
}

#[async_trait::async_trait]
impl LlmProvider for CaptureProvider {
    fn name(&self) -> &'static str {
        "capture"
    }

    async fn chat(&self, _request: LlmRequest) -> anyhow::Result<LlmResponse> {
        Err(anyhow::anyhow!("unused in this test"))
    }

    async fn chat_stream(&self, request: LlmRequest) -> anyhow::Result<LlmStream> {
        *self.request.lock().expect("lock") = Some(request);
        Ok(Box::pin(stream::iter(vec![
            Ok(LlmStreamEvent {
                delta: "ok".to_string(),
                tool_call: None,
                done: false,
                usage: None,
            }),
            Ok(LlmStreamEvent {
                delta: String::new(),
                tool_call: None,
                done: true,
                usage: None,
            }),
        ])))
    }
}

#[tokio::test]
async fn agent_builds_single_system_message_with_memory_context() {
    let temp = tempdir().expect("tempdir");

    let personality_dir = temp.path().join("personality");
    std::fs::create_dir_all(&personality_dir).expect("personality dir");
    std::fs::write(personality_dir.join("SOUL.md"), "You are helpful").expect("write soul");

    let memory_dir = temp.path().join("memory");
    std::fs::create_dir_all(&memory_dir).expect("memory dir");
    let memory_file = temp.path().join("MEMORY.md");
    std::fs::write(&memory_file, "topic: project context\n").expect("write memory");

    let provider = CaptureProvider::default();
    let request_capture = provider.request.clone();

    let memory_store = MemoryStore::new(memory_dir, memory_file);
    memory_store.ensure_layout().await.expect("ensure memory");
    let memory: Arc<dyn MemoryBackend> = Arc::new(memory_store);
    let personality: Arc<dyn PersonalitySource> = Arc::new(PersonalityLoader::new(personality_dir));

    let agent = AgentLoop::new(
        Arc::new(provider),
        Arc::new(ToolRegistry::new()),
        personality,
        memory,
        AgentConfig {
            model: "mock-model".to_string(),
            temperature: 0.0,
            max_tokens: 128,
            max_iterations: 2,
            token_budget: 4096,
            working_dir: temp.path().to_path_buf(),
        },
    );

    let mut session = SessionState::new("s1");
    let output = agent
        .run_stream(&mut session, "topic".to_string(), |_| {})
        .await
        .expect("run stream");

    assert_eq!(output.assistant_message.content, "ok");

    let request = request_capture
        .lock()
        .expect("lock")
        .clone()
        .expect("captured request");
    let system_count = request
        .messages
        .iter()
        .filter(|message| message.role == Role::System)
        .count();

    assert_eq!(system_count, 1);
}
