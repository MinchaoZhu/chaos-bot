use crate::agent::{AgentLoop, AgentStreamEvent, ToolEvent};
use crate::config::AgentFileConfig;
use crate::config_runtime::ConfigRuntime;
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
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(Clone)]
pub struct AppState {
    pub agent: Arc<RwLock<Arc<AgentLoop>>>,
    pub sessions: SessionStore,
    pub config_runtime: Option<Arc<ConfigRuntime>>,
}

impl AppState {
    pub fn new(agent: Arc<AgentLoop>) -> Self {
        Self {
            agent: Arc::new(RwLock::new(agent)),
            sessions: SessionStore::new(),
            config_runtime: None,
        }
    }

    pub fn with_config_runtime(
        agent: Arc<RwLock<Arc<AgentLoop>>>,
        config_runtime: Arc<ConfigRuntime>,
    ) -> Self {
        Self {
            agent,
            sessions: SessionStore::new(),
            config_runtime: Some(config_runtime),
        }
    }

    pub async fn current_agent(&self) -> Arc<AgentLoop> {
        self.agent.read().await.clone()
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

#[derive(Debug, Deserialize, Default)]
pub struct ConfigMutationRequest {
    pub raw: Option<String>,
    pub config: Option<AgentFileConfig>,
}

#[derive(Debug, Serialize)]
pub struct ConfigStateResponse {
    pub config_path: String,
    pub backup1_path: String,
    pub backup2_path: String,
    pub config_format: String,
    pub running: AgentFileConfig,
    pub disk: AgentFileConfig,
    pub raw: String,
    pub disk_parse_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConfigMutationResponse {
    pub ok: bool,
    pub action: &'static str,
    pub restart_scheduled: bool,
    pub state: ConfigStateResponse,
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
        .route("/api/config", get(get_config))
        .route("/api/config/reset", post(reset_config))
        .route("/api/config/apply", post(apply_config))
        .route("/api/config/restart", post(restart_config))
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
    let session = state.sessions.create().await;
    tracing::info!(session_id = %session.id, "api create session");
    Json(session)
}

async fn list_sessions(State(state): State<AppState>) -> Json<Vec<SessionState>> {
    let sessions = state.sessions.list().await;
    tracing::debug!(count = sessions.len(), "api list sessions");
    Json(sessions)
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
    tracing::debug!(session_id = %id, "api get session");
    Ok(Json(session))
}

async fn delete_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    if state.sessions.delete(&id).await {
        tracing::info!(session_id = %id, "api delete session");
        Ok(axum::http::StatusCode::NO_CONTENT)
    } else {
        tracing::debug!(session_id = %id, "api delete missing session");
        Err(axum::http::StatusCode::NOT_FOUND)
    }
}

async fn chat(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::info!(
        has_session_id = payload.session_id.is_some(),
        message_chars = payload.message.chars().count(),
        "api chat request"
    );
    let (tx, rx) = mpsc::unbounded_channel::<Result<Event, Infallible>>();

    tokio::spawn(async move {
        let send_event = |event: Event| {
            let _ = tx.send(Ok(event));
        };

        let (session_id, mut session) = match payload.session_id {
            Some(id) => match state.sessions.get(&id).await {
                Some(existing) => {
                    tracing::debug!(session_id = %id, "chat using existing session");
                    (id, existing)
                }
                None => {
                    let created = SessionState::new(id.clone());
                    tracing::info!(session_id = %id, "chat created missing session id");
                    (id, created)
                }
            },
            None => {
                let created = state.sessions.create().await;
                tracing::info!(session_id = %created.id, "chat auto-created session");
                (created.id.clone(), created)
            }
        };

        send_event(
            Event::default()
                .event("session")
                .data(json!({"session_id": session_id.clone()}).to_string()),
        );

        let agent = state.current_agent().await;
        let result = agent
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
                tracing::info!(
                    session_id = %session_id,
                    finish_reason = output.finish_reason.as_deref().unwrap_or("unknown"),
                    usage_total_tokens = output.usage.as_ref().map(|u| u.total_tokens),
                    "chat completed"
                );
                send_event(
                    Event::default().event("done").data(
                        json!({
                            "session_id": session_id.clone(),
                            "usage": output.usage,
                            "finish_reason": output.finish_reason,
                        })
                        .to_string(),
                    ),
                );
            }
            Err(error) => {
                tracing::warn!(
                    session_id = %session_id,
                    error = %error,
                    "chat run_stream failed"
                );
                send_event(
                    Event::default()
                        .event("error")
                        .data(json!({"message": error.to_string()}).to_string()),
                );
            }
        }

        state.sessions.upsert(session).await;
        tracing::debug!(session_id = %session_id, "chat session persisted");
    });

    Sse::new(UnboundedReceiverStream::new(rx)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<ConfigStateResponse>, axum::http::StatusCode> {
    let runtime = require_config_runtime(&state)?;
    let response = build_config_state_response(&runtime).await;
    Ok(Json(response))
}

async fn reset_config(
    State(state): State<AppState>,
) -> Result<Json<ConfigMutationResponse>, axum::http::StatusCode> {
    let runtime = require_config_runtime(&state)?;
    runtime.reset().await.map_err(internal_status)?;
    let state = build_config_state_response(&runtime).await;
    Ok(Json(ConfigMutationResponse {
        ok: true,
        action: "reset",
        restart_scheduled: false,
        state,
    }))
}

async fn apply_config(
    State(state): State<AppState>,
    Json(payload): Json<ConfigMutationRequest>,
) -> Result<Json<ConfigMutationResponse>, axum::http::StatusCode> {
    let runtime = require_config_runtime(&state)?;
    match (payload.raw, payload.config) {
        (Some(raw), None) => runtime.apply_raw(&raw).await.map_err(internal_status)?,
        (None, Some(config)) => runtime
            .apply_structured(config)
            .await
            .map_err(internal_status)?,
        _ => {
            return Err(axum::http::StatusCode::BAD_REQUEST);
        }
    };

    let state = build_config_state_response(&runtime).await;
    Ok(Json(ConfigMutationResponse {
        ok: true,
        action: "apply",
        restart_scheduled: false,
        state,
    }))
}

async fn restart_config(
    State(state): State<AppState>,
    Json(payload): Json<ConfigMutationRequest>,
) -> Result<Json<ConfigMutationResponse>, axum::http::StatusCode> {
    let runtime = require_config_runtime(&state)?;
    let restart_scheduled = match (payload.raw, payload.config) {
        (Some(raw), None) => runtime
            .restart_after_apply_raw(&raw)
            .await
            .map_err(internal_status)?,
        (None, Some(config)) => runtime
            .restart_after_apply_structured(config)
            .await
            .map_err(internal_status)?,
        (None, None) => runtime.request_restart().await.map_err(internal_status)?,
        _ => {
            return Err(axum::http::StatusCode::BAD_REQUEST);
        }
    };

    let state = build_config_state_response(&runtime).await;
    Ok(Json(ConfigMutationResponse {
        ok: true,
        action: "restart",
        restart_scheduled,
        state,
    }))
}

fn require_config_runtime(state: &AppState) -> Result<Arc<ConfigRuntime>, axum::http::StatusCode> {
    state
        .config_runtime
        .clone()
        .ok_or(axum::http::StatusCode::SERVICE_UNAVAILABLE)
}

fn internal_status(error: anyhow::Error) -> axum::http::StatusCode {
    tracing::warn!(error = %error, "config endpoint failed");
    axum::http::StatusCode::INTERNAL_SERVER_ERROR
}

async fn build_config_state_response(runtime: &ConfigRuntime) -> ConfigStateResponse {
    let running = runtime.running_config().await;
    let (disk, raw, disk_parse_error) = match runtime.disk_config().await {
        Ok((disk, raw)) => (disk, raw, None),
        Err(error) => {
            let fallback_raw = serde_json::to_string_pretty(&running)
                .map(|text| format!("{text}\n"))
                .unwrap_or_else(|_| "{}\n".to_string());
            (running.clone(), fallback_raw, Some(error.to_string()))
        }
    };

    let config_path = runtime.config_path().to_path_buf();
    let config_format = config_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "config.json".to_string());

    ConfigStateResponse {
        config_path: config_path.display().to_string(),
        backup1_path: runtime.backup_path(1).display().to_string(),
        backup2_path: runtime.backup_path(2).display().to_string(),
        config_format,
        running,
        disk,
        raw,
        disk_parse_error,
    }
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
