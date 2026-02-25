mod support;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use chaos_bot_backend::interface::api::router;
use chaos_bot_backend::infrastructure::model::LlmStreamEvent;
use chaos_bot_backend::domain::types::{SessionState, ToolCall};
use serde_json::{json, Value};
use std::sync::Arc;
use support::*;
use tower::util::ServiceExt;

// -------------------------------------------------------------------------
// Health endpoint
// -------------------------------------------------------------------------

#[tokio::test]
async fn health_returns_ok_json() {
    let provider = MockStreamProvider::text("hi");
    let (_temp, state) = build_test_state(Arc::new(provider));
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert!(json["now"].as_str().is_some());
}

// -------------------------------------------------------------------------
// Session CRUD lifecycle
// -------------------------------------------------------------------------

#[tokio::test]
async fn session_crud_lifecycle() {
    let provider = MockStreamProvider::text("hi");
    let (_temp, state) = build_test_state(Arc::new(provider));
    let app = router(state);

    // Create
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let session: SessionState = serde_json::from_slice(&body).unwrap();
    let sid = session.id.clone();

    // Get
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/sessions/{sid}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // List
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let sessions: Vec<SessionState> = serde_json::from_slice(&body).unwrap();
    assert!(sessions.iter().any(|s| s.id == sid));

    // Delete
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/sessions/{sid}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    // Get after delete → 404
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/sessions/{sid}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// -------------------------------------------------------------------------
// Delete non-existent session → 404
// -------------------------------------------------------------------------

#[tokio::test]
async fn delete_nonexistent_session_returns_404() {
    let provider = MockStreamProvider::text("hi");
    let (_temp, state) = build_test_state(Arc::new(provider));
    let app = router(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/sessions/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

// -------------------------------------------------------------------------
// Chat SSE streaming — simple text response
// -------------------------------------------------------------------------

#[tokio::test]
async fn chat_sse_simple_text() {
    let provider = MockStreamProvider::text("Hello world!");
    let (_temp, state) = build_test_state(Arc::new(provider));
    let app = router(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/chat")
                .header("content-type", "application/json")
                .body(Body::from(json!({"message": "hi"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let text = String::from_utf8_lossy(&body);

    // Should contain session, delta, and done events
    assert!(text.contains("event: session"));
    assert!(text.contains("event: delta"));
    assert!(text.contains("event: done"));
    assert!(text.contains("Hello world!"));
}

// -------------------------------------------------------------------------
// Chat SSE streaming — with tool calls
// -------------------------------------------------------------------------

#[tokio::test]
async fn chat_sse_with_tool_calls() {
    let tool_call = ToolCall {
        id: "tc_1".to_string(),
        name: "mock_tool".to_string(),
        arguments: json!({}),
    };
    let provider = MockStreamProvider::tool_then_text(tool_call, "After tool");

    let mut registry = chaos_bot_backend::infrastructure::tooling::ToolRegistry::new();
    registry.register(MockTool::fixed("mock_tool", "tool result"));

    let (_temp, state) = build_test_state_with_registry(Arc::new(provider), registry);
    let app = router(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/chat")
                .header("content-type", "application/json")
                .body(Body::from(json!({"message": "do something"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let text = String::from_utf8_lossy(&body);

    assert!(text.contains("event: session"));
    assert!(text.contains("event: tool_call"));
    assert!(text.contains("event: done"));
    assert!(text.contains("mock_tool"));
}

// -------------------------------------------------------------------------
// Chat with existing session (conversation accumulates)
// -------------------------------------------------------------------------

#[tokio::test]
async fn chat_with_existing_session_accumulates() {
    let provider = MockStreamProvider::new(vec![
        // First chat
        vec![
            LlmStreamEvent {
                delta: "first reply".into(),
                tool_call: None,
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
        // Second chat
        vec![
            LlmStreamEvent {
                delta: "second reply".into(),
                tool_call: None,
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
    ]);

    let (_temp, state) = build_test_state(Arc::new(provider));
    let app = router(state.clone());

    // Create session
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let session: SessionState = serde_json::from_slice(&body).unwrap();

    // First chat
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/chat")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"session_id": session.id, "message": "hello"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    // Wait for body to complete
    let _ = to_bytes(res.into_body(), usize::MAX).await.unwrap();

    // Second chat
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/chat")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"session_id": session.id, "message": "follow up"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("second reply"));

    // Check session has accumulated messages
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/sessions/{}", session.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let stored: SessionState = serde_json::from_slice(&body).unwrap();
    // user1 + assistant1 + user2 + assistant2 = 4 messages
    assert!(stored.messages.len() >= 4);
}

// -------------------------------------------------------------------------
// Chat error handling (mock returns error → SSE error event)
// -------------------------------------------------------------------------

#[tokio::test]
async fn chat_error_returns_sse_error_event() {
    let provider = ErrorProvider::new("test error");
    let (_temp, state) = build_test_state(Arc::new(provider));
    let app = router(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/chat")
                .header("content-type", "application/json")
                .body(Body::from(json!({"message": "hi"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK); // SSE always returns 200

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let text = String::from_utf8_lossy(&body);

    assert!(text.contains("event: error"));
    assert!(text.contains("test error"));
}

// -------------------------------------------------------------------------
// Chat creates session if none provided
// -------------------------------------------------------------------------

#[tokio::test]
async fn chat_creates_session_when_none_provided() {
    let provider = MockStreamProvider::text("auto-session");
    let (_temp, state) = build_test_state(Arc::new(provider));
    let app = router(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/chat")
                .header("content-type", "application/json")
                .body(Body::from(json!({"message": "hi"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("event: session"));
    assert!(text.contains("session_id"));
}

#[tokio::test]
async fn config_api_apply_reset_restart_lifecycle() {
    let (_temp, state, _config_path) = build_test_state_with_config_runtime().await;
    let app = router(state);

    let get_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_res.status(), StatusCode::OK);
    let body = to_bytes(get_res.into_body(), usize::MAX).await.unwrap();
    let mut state_json: Value = serde_json::from_slice(&body).unwrap();

    state_json["running"]["llm"]["model"] = Value::String("mock-next".to_string());

    let apply_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/config/apply")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "config": state_json["running"].clone() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(apply_res.status(), StatusCode::OK);
    let body = to_bytes(apply_res.into_body(), usize::MAX).await.unwrap();
    let apply_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(apply_json["state"]["running"]["llm"]["model"], "mock-next");

    let backup1 = apply_json["state"]["backup1_path"].as_str().unwrap();
    assert!(std::path::Path::new(backup1).exists());

    let reset_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/config/reset")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reset_res.status(), StatusCode::OK);

    let restart_res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/config/restart")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(restart_res.status(), StatusCode::OK);
    let body = to_bytes(restart_res.into_body(), usize::MAX).await.unwrap();
    let restart_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(restart_json["restart_scheduled"], false);
}
