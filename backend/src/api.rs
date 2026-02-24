use crate::agent::{AgentLoop, AgentStreamEvent, ToolEvent};
use crate::sessions::SessionStore;
use crate::types::SessionState;
use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(Clone)]
pub struct AppState {
    pub agent: Arc<AgentLoop>,
    pub sessions: SessionStore,
}

impl AppState {
    pub fn new(agent: Arc<AgentLoop>) -> Self {
        Self {
            agent,
            sessions: SessionStore::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub session_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub now: String,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/app.js", get(app_js))
        .route("/style.css", get(style_css))
        .route("/api/health", get(health))
        .route("/api/chat", post(chat))
        .route("/api/sessions", post(create_session).get(list_sessions))
        .route("/api/sessions/:id", get(get_session).delete(delete_session))
        .with_state(state)
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../../frontend/index.html"))
}

async fn app_js() -> impl IntoResponse {
    (
        [("content-type", "application/javascript; charset=utf-8")],
        include_str!("../../frontend/app.js"),
    )
}

async fn style_css() -> impl IntoResponse {
    (
        [("content-type", "text/css; charset=utf-8")],
        include_str!("../../frontend/style.css"),
    )
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        now: Utc::now().to_rfc3339(),
    })
}

async fn create_session(State(state): State<AppState>) -> Json<SessionState> {
    Json(state.sessions.create().await)
}

async fn list_sessions(State(state): State<AppState>) -> Json<Vec<SessionState>> {
    Json(state.sessions.list().await)
}

async fn get_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SessionState>, axum::http::StatusCode> {
    let session = state
        .sessions
        .get(&id)
        .await
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;
    Ok(Json(session))
}

async fn delete_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    if state.sessions.delete(&id).await {
        Ok(axum::http::StatusCode::NO_CONTENT)
    } else {
        Err(axum::http::StatusCode::NOT_FOUND)
    }
}

async fn chat(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::unbounded_channel::<Result<Event, Infallible>>();

    tokio::spawn(async move {
        let send_event = |event: Event| {
            let _ = tx.send(Ok(event));
        };

        let (session_id, mut session) = match payload.session_id {
            Some(id) => match state.sessions.get(&id).await {
                Some(existing) => (id, existing),
                None => {
                    let created = SessionState::new(id.clone());
                    (id, created)
                }
            },
            None => {
                let created = state.sessions.create().await;
                (created.id.clone(), created)
            }
        };

        send_event(
            Event::default()
                .event("session")
                .data(json!({"session_id": session_id.clone()}).to_string()),
        );

        let result = state
            .agent
            .run_stream(&mut session, payload.message, |event| match event {
                AgentStreamEvent::Delta(chunk) => {
                    send_event(Event::default().event("delta").data(chunk));
                }
                AgentStreamEvent::Tool(tool) => {
                    send_event(tool_event_to_sse(tool));
                }
            })
            .await;

        match result {
            Ok(output) => {
                send_event(
                    Event::default().event("done").data(
                        json!({
                            "session_id": session_id,
                            "usage": output.usage,
                            "finish_reason": output.finish_reason,
                        })
                        .to_string(),
                    ),
                );
            }
            Err(error) => {
                send_event(
                    Event::default()
                        .event("error")
                        .data(json!({"message": error.to_string()}).to_string()),
                );
            }
        }

        state.sessions.upsert(session).await;
    });

    Sse::new(UnboundedReceiverStream::new(rx)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

fn tool_event_to_sse(event: ToolEvent) -> Event {
    Event::default().event("tool_call").data(
        json!({
            "id": event.call.id,
            "name": event.call.name,
            "args": event.call.arguments,
            "output": event.result.output,
            "is_error": event.result.is_error,
        })
        .to_string(),
    )
}
