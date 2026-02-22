use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use chaos_bot_backend::agent::AgentLoop;
use chaos_bot_backend::api::{router, AppState};
use chaos_bot_backend::llm::{LlmProvider, LlmRequest, LlmResponse, LlmStream};
use chaos_bot_backend::memory::MemoryStore;
use chaos_bot_backend::personality::PersonalityLoader;
use chaos_bot_backend::sessions::SessionStore;
use chaos_bot_backend::tools::ToolRegistry;
use chaos_bot_backend::types::SessionState;
use std::sync::Arc;
use tempfile::tempdir;
use tower::util::ServiceExt;

struct MockProvider;

#[async_trait::async_trait]
impl LlmProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn chat(&self, _request: LlmRequest) -> anyhow::Result<LlmResponse> {
        Err(anyhow::anyhow!("unused in this test"))
    }

    async fn chat_stream(&self, _request: LlmRequest) -> anyhow::Result<LlmStream> {
        Err(anyhow::anyhow!("unused in this test"))
    }
}

fn build_state() -> AppState {
    let temp = tempdir().expect("tempdir");
    let personality_dir = temp.path().join("personality");
    std::fs::create_dir_all(&personality_dir).expect("create personality dir");
    std::fs::write(personality_dir.join("SOUL.md"), "# soul").expect("write soul");

    let memory_dir = temp.path().join("memory");
    let memory_file = temp.path().join("MEMORY.md");
    let memory = MemoryStore::new(memory_dir, memory_file);

    let agent = AgentLoop::new(
        Arc::new(MockProvider),
        Arc::new(ToolRegistry::new()),
        PersonalityLoader::new(personality_dir),
        memory,
        "mock-model".to_string(),
        0.0,
        128,
        1,
        1024,
        temp.path().to_path_buf(),
    );

    AppState {
        agent: Arc::new(agent),
        sessions: SessionStore::new(),
    }
}

#[tokio::test]
async fn sessions_get_and_delete_routes_work() {
    let app = router(build_state());

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(create_response.status(), StatusCode::OK);

    let body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let session: SessionState = serde_json::from_slice(&body).expect("session json");

    let get_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/sessions/{}", session.id))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(get_response.status(), StatusCode::OK);

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/sessions/{}", session.id))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    let get_missing_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/sessions/{}", session.id))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(get_missing_response.status(), StatusCode::NOT_FOUND);
}
