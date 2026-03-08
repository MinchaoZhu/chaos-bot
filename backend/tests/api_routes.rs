use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use chaos_bot_backend::application::agent::{AgentConfig, AgentLoop};
use chaos_bot_backend::domain::ports::UpgradePort;
use chaos_bot_backend::domain::types::SessionState;
use chaos_bot_backend::domain::upgrade::{UpgradeApplyResult, UpgradeStatus};
use chaos_bot_backend::infrastructure::memory::{MemoryBackend, MemoryStore};
use chaos_bot_backend::infrastructure::model::{LlmProvider, LlmRequest, LlmResponse, LlmStream};
use chaos_bot_backend::infrastructure::personality::{PersonalityLoader, PersonalitySource};
use chaos_bot_backend::infrastructure::skills::EmptySkillStore;
use chaos_bot_backend::infrastructure::tooling::ToolRegistry;
use chaos_bot_backend::interface::api::{router, AppState};
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

struct MockUpgradePort;

#[async_trait::async_trait]
impl UpgradePort for MockUpgradePort {
    async fn status(&self) -> anyhow::Result<UpgradeStatus> {
        Ok(UpgradeStatus {
            supported: true,
            current_version: Some("0.1.0-master.1".to_string()),
            latest_version: Some("0.1.0-master.2".to_string()),
            latest_tag_name: Some("v0.1.0-master.2".to_string()),
            upgrade_available: true,
            install_prefix: Some("/tmp/install".to_string()),
            current_release_root: Some(
                "/tmp/install/share/chaos-bot/releases/0.1.0-master.1".to_string(),
            ),
            repository: Some("MinchaoZhu/chaos-bot".to_string()),
            latest_release_url: Some("https://example.invalid/releases/latest".to_string()),
            download_url: Some("https://example.invalid/bundle.tar.gz".to_string()),
            reason: None,
        })
    }

    async fn apply(&self) -> anyhow::Result<UpgradeApplyResult> {
        let status = self.status().await?;
        Ok(UpgradeApplyResult {
            ok: true,
            action: "upgrade",
            current_version: Some("0.1.0-master.1".to_string()),
            target_version: Some("0.1.0-master.2".to_string()),
            launcher_path: Some("/tmp/install/bin/chaos-bot".to_string()),
            installed_release_root: Some(
                "/tmp/install/share/chaos-bot/releases/0.1.0-master.2".to_string(),
            ),
            relaunch_required: true,
            message: "installed 0.1.0-master.2. relaunch the launcher to start the new release"
                .to_string(),
            status,
        })
    }
}

fn build_state(
    frontend_dist: Option<std::path::PathBuf>,
    upgrade_port: Option<Arc<dyn UpgradePort>>,
) -> (tempfile::TempDir, AppState) {
    let temp = tempdir().expect("tempdir");
    let personality_dir = temp.path().join("personality");
    std::fs::create_dir_all(&personality_dir).expect("create personality dir");
    std::fs::write(personality_dir.join("SOUL.md"), "# soul").expect("write soul");

    let memory_dir = temp.path().join("memory");
    let memory_file = temp.path().join("MEMORY.md");
    let memory: Arc<dyn MemoryBackend> = Arc::new(MemoryStore::new(memory_dir, memory_file));
    let personality: Arc<dyn PersonalitySource> = Arc::new(PersonalityLoader::new(personality_dir));

    let agent = AgentLoop::new(
        Arc::new(MockProvider),
        Arc::new(ToolRegistry::new()),
        personality,
        memory,
        Arc::new(EmptySkillStore),
        AgentConfig {
            model: "mock-model".to_string(),
            temperature: 0.0,
            max_tokens: 128,
            max_iterations: 1,
            token_budget: 1024,
            working_dir: temp.path().to_path_buf(),
        },
    );

    let mut state = AppState::new(
        Arc::new(agent),
        None,
        None,
        false,
        false,
        "https://api.telegram.org".to_string(),
        frontend_dist,
    );
    if let Some(upgrades) = upgrade_port {
        state = state.with_upgrade(upgrades);
    }

    (temp, state)
}

#[tokio::test]
async fn sessions_get_and_delete_routes_work() {
    let (_temp, state) = build_state(None, None);
    let app = router(state);

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

#[tokio::test]
async fn frontend_routes_serve_dist_assets_and_spa_index() {
    let temp = tempdir().expect("tempdir");
    let frontend_dist = temp.path().join("frontend-dist");
    let assets_dir = frontend_dist.join("assets");
    std::fs::create_dir_all(&assets_dir).expect("create assets dir");
    std::fs::write(
        frontend_dist.join("index.html"),
        "<html><body>chaos-bot</body></html>",
    )
    .expect("write index");
    std::fs::write(assets_dir.join("app.js"), "console.log('bundle');").expect("write js");

    let (_runtime_temp, state) = build_state(Some(frontend_dist), None);
    let app = router(state);

    let root_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(root_response.status(), StatusCode::OK);
    assert_eq!(
        root_response.headers().get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );

    let spa_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/sessions/123")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let spa_body = to_bytes(spa_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    assert!(String::from_utf8_lossy(&spa_body).contains("chaos-bot"));

    let asset_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/assets/app.js")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(asset_response.status(), StatusCode::OK);
    assert_eq!(
        asset_response.headers().get("content-type").unwrap(),
        "application/javascript; charset=utf-8"
    );

    let api_response = app
        .oneshot(
            Request::builder()
                .uri("/api/missing")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(api_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn upgrade_routes_return_status_and_apply_result() {
    let (_temp, state) = build_state(None, Some(Arc::new(MockUpgradePort)));
    let app = router(state);

    let status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/upgrade")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(status_response.status(), StatusCode::OK);
    let status_body = to_bytes(status_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let status_json: serde_json::Value = serde_json::from_slice(&status_body).expect("status json");
    assert_eq!(status_json["upgrade_available"], true);
    assert_eq!(status_json["latest_version"], "0.1.0-master.2");

    let apply_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/upgrade/apply")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(apply_response.status(), StatusCode::OK);
    let apply_body = to_bytes(apply_response.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let apply_json: serde_json::Value = serde_json::from_slice(&apply_body).expect("apply json");
    assert_eq!(apply_json["action"], "upgrade");
    assert_eq!(apply_json["relaunch_required"], true);
}
