use crate::application::agent::AgentLoop;
use crate::application::{ChatService, ConfigService, SessionService, UpgradeService};
use crate::domain::chat::{ChatCommand, ChatEvent, ToolEvent};
use crate::domain::config::{
    ConfigMutationInput, ConfigMutationResponse, ConfigRestartInput, ConfigStateResponse,
};
use crate::domain::ports::ChannelDispatcherPort;
use crate::domain::ports::SkillPort;
use crate::domain::ports::UpgradePort;
use crate::domain::skills::{SkillDetail, SkillMeta};
use crate::domain::types::SessionState;
use crate::domain::upgrade::{UpgradeApplyResult, UpgradeRestartResult, UpgradeStatus};
use crate::domain::AppError;
use crate::infrastructure::channels::telegram::TelegramWebhookUpdate;
use crate::infrastructure::config::AgentFileConfig;
use crate::infrastructure::session_store::SessionStore;
use crate::infrastructure::skills::{install_skills_from_git_source, EmptySkillStore};
use crate::runtime::config_runtime::ConfigRuntime;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::http::{header, Response, StatusCode, Uri};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::Infallible;
use std::path::{Component, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(Clone)]
pub struct AppState {
    pub agent: Arc<RwLock<Arc<AgentLoop>>>,
    pub sessions: SessionStore,
    pub config_runtime: Option<Arc<ConfigRuntime>>,
    pub channel_dispatcher: Option<Arc<dyn ChannelDispatcherPort>>,
    pub telegram_webhook_secret: Option<String>,
    pub telegram_enabled: bool,
    pub telegram_polling: bool,
    pub telegram_api_base_url: String,
    pub skills: Arc<dyn SkillPort>,
    pub skills_dir: PathBuf,
    pub frontend_dist: Option<PathBuf>,
    pub upgrades: Option<Arc<dyn UpgradePort>>,
}

impl AppState {
    pub fn new(
        agent: Arc<AgentLoop>,
        channel_dispatcher: Option<Arc<dyn ChannelDispatcherPort>>,
        telegram_webhook_secret: Option<String>,
        telegram_enabled: bool,
        telegram_polling: bool,
        telegram_api_base_url: String,
        frontend_dist: Option<PathBuf>,
    ) -> Self {
        Self {
            agent: Arc::new(RwLock::new(agent)),
            sessions: SessionStore::new(),
            config_runtime: None,
            channel_dispatcher,
            telegram_webhook_secret,
            telegram_enabled,
            telegram_polling,
            telegram_api_base_url,
            skills: Arc::new(EmptySkillStore),
            skills_dir: PathBuf::from("."),
            frontend_dist,
            upgrades: None,
        }
    }

    pub fn with_config_runtime(
        agent: Arc<RwLock<Arc<AgentLoop>>>,
        config_runtime: Arc<ConfigRuntime>,
        channel_dispatcher: Option<Arc<dyn ChannelDispatcherPort>>,
        telegram_webhook_secret: Option<String>,
        telegram_enabled: bool,
        telegram_polling: bool,
        telegram_api_base_url: String,
        frontend_dist: Option<PathBuf>,
    ) -> Self {
        Self {
            agent,
            sessions: SessionStore::new(),
            config_runtime: Some(config_runtime),
            channel_dispatcher,
            telegram_webhook_secret,
            telegram_enabled,
            telegram_polling,
            telegram_api_base_url,
            skills: Arc::new(EmptySkillStore),
            skills_dir: PathBuf::from("."),
            frontend_dist,
            upgrades: None,
        }
    }

    pub fn with_skills(mut self, skills: Arc<dyn SkillPort>) -> Self {
        self.skills = skills;
        self
    }

    pub fn with_skills_dir(mut self, skills_dir: PathBuf) -> Self {
        self.skills_dir = skills_dir;
        self
    }

    pub fn with_upgrade(mut self, upgrades: Arc<dyn UpgradePort>) -> Self {
        self.upgrades = Some(upgrades);
        self
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
pub struct TelegramWebhookResponse {
    pub ok: bool,
    pub ignored: bool,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChannelStatusResponse {
    pub enabled_channels: Vec<String>,
    pub connectors: Vec<crate::domain::chat::ChannelHealth>,
    pub telegram: TelegramChannelStatus,
}

#[derive(Debug, Deserialize)]
pub struct InstallSkillRequest {
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct InstallSkillResponse {
    pub ok: bool,
    pub source: String,
    pub installed: Vec<SkillMeta>,
}

#[derive(Debug, Serialize)]
pub struct TelegramChannelStatus {
    pub enabled: bool,
    pub polling: bool,
    pub api_base_url: String,
    pub webhook_secret_configured: bool,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/channels/status", get(channel_status))
        .route("/api/chat", post(chat))
        .route("/api/channels/telegram/webhook", post(telegram_webhook))
        .route("/api/sessions", post(create_session).get(list_sessions))
        .route("/api/sessions/:id", get(get_session).delete(delete_session))
        .route("/api/config", get(get_config))
        .route("/api/config/reset", post(reset_config))
        .route("/api/config/apply", post(apply_config))
        .route("/api/config/restart", post(restart_config))
        .route("/api/skills", get(list_skills))
        .route("/api/skills/install", post(install_skill))
        .route("/api/skills/:id", get(get_skill))
        .route("/api/upgrade", get(get_upgrade_status))
        .route("/api/upgrade/apply", post(apply_upgrade))
        .route("/api/upgrade/relaunch", post(relaunch_upgrade))
        .fallback(get(frontend))
        .with_state(state)
}

async fn frontend(State(state): State<AppState>, uri: Uri) -> Result<Response<Body>, StatusCode> {
    let Some(frontend_dist) = state.frontend_dist.as_ref() else {
        return Err(StatusCode::NOT_FOUND);
    };

    let request_path = uri.path().trim_start_matches('/');
    if request_path == "api" || request_path.starts_with("api/") {
        return Err(StatusCode::NOT_FOUND);
    }

    let file_path = resolve_frontend_asset_path(frontend_dist, request_path)?;
    let resolved_path = match tokio::fs::metadata(&file_path).await {
        Ok(metadata) if metadata.is_file() => file_path,
        _ => frontend_dist.join("index.html"),
    };
    let bytes = tokio::fs::read(&resolved_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type_for(&resolved_path))
        .body(Body::from(bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn resolve_frontend_asset_path(
    frontend_dist: &std::path::Path,
    request_path: &str,
) -> Result<PathBuf, StatusCode> {
    let relative = if request_path.is_empty() {
        PathBuf::from("index.html")
    } else {
        let candidate = PathBuf::from(request_path);
        if candidate.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(StatusCode::BAD_REQUEST);
        }
        if request_path.ends_with('/') {
            candidate.join("index.html")
        } else {
            candidate
        }
    };

    Ok(frontend_dist.join(relative))
}

fn content_type_for(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    }
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        now: Utc::now().to_rfc3339(),
    })
}

async fn channel_status(
    State(state): State<AppState>,
) -> Result<Json<ChannelStatusResponse>, AppError> {
    let (enabled_channels, connectors) = if let Some(dispatcher) = &state.channel_dispatcher {
        let channels = dispatcher.enabled_channels();
        let health = dispatcher.health_summary().await.map_err(|error| {
            AppError::internal(format!("failed to load channel health: {error}"))
        })?;
        (channels, health)
    } else {
        (Vec::new(), Vec::new())
    };

    Ok(Json(ChannelStatusResponse {
        enabled_channels,
        connectors,
        telegram: TelegramChannelStatus {
            enabled: state.telegram_enabled,
            polling: state.telegram_polling,
            api_base_url: state.telegram_api_base_url.clone(),
            webhook_secret_configured: state.telegram_webhook_secret.is_some(),
        },
    }))
}

async fn create_session(State(state): State<AppState>) -> Json<SessionState> {
    let service = SessionService::new(state.sessions.clone());
    let session = service.create().await;
    tracing::info!(session_id = %session.id, "api create session");
    Json(session)
}

async fn list_sessions(State(state): State<AppState>) -> Json<Vec<SessionState>> {
    let service = SessionService::new(state.sessions.clone());
    let sessions = service.list().await;
    tracing::debug!(count = sessions.len(), "api list sessions");
    Json(sessions)
}

async fn get_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SessionState>, AppError> {
    let service = SessionService::new(state.sessions.clone());
    let session = service.get(&id).await?;
    tracing::debug!(session_id = %id, "api get session");
    Ok(Json(session))
}

async fn delete_session(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<axum::http::StatusCode, AppError> {
    let service = SessionService::new(state.sessions.clone());
    service.delete(&id).await?;
    tracing::info!(session_id = %id, "api delete session");
    Ok(axum::http::StatusCode::NO_CONTENT)
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

        let service = ChatService::new(
            state.agent.clone(),
            state.sessions.clone(),
            state.channel_dispatcher.clone(),
        );
        let result = service
            .run_stream(
                ChatCommand {
                    session_id: payload.session_id,
                    message: payload.message,
                    channel: None,
                },
                |event| send_event(chat_event_to_sse(event)),
            )
            .await;

        match result {
            Ok(output) => {
                send_event(
                    Event::default().event("done").data(
                        json!({
                            "session_id": output.session_id,
                            "usage": output.usage,
                            "finish_reason": output.finish_reason,
                        })
                        .to_string(),
                    ),
                );
            }
            Err(error) => {
                send_event(
                    Event::default().event("error").data(
                        json!({
                            "code": error.code_str(),
                            "message": error.message(),
                        })
                        .to_string(),
                    ),
                );
            }
        }
    });

    Sse::new(UnboundedReceiverStream::new(rx)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

async fn telegram_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(update): Json<TelegramWebhookUpdate>,
) -> Result<Json<TelegramWebhookResponse>, AppError> {
    if state.channel_dispatcher.is_none() {
        return Err(AppError::service_unavailable(
            "channel dispatcher unavailable",
        ));
    }

    if let Some(expected) = &state.telegram_webhook_secret {
        let provided = headers
            .get("x-telegram-bot-api-secret-token")
            .and_then(|value| value.to_str().ok());
        if provided != Some(expected.as_str()) {
            return Err(AppError::bad_request("invalid telegram webhook secret"));
        }
    }

    let Some(inbound) = update.into_inbound_message() else {
        return Ok(Json(TelegramWebhookResponse {
            ok: true,
            ignored: true,
            session_id: None,
        }));
    };

    let service = ChatService::new(
        state.agent.clone(),
        state.sessions.clone(),
        state.channel_dispatcher.clone(),
    );
    let result = service.run_channel_message(inbound).await?;

    Ok(Json(TelegramWebhookResponse {
        ok: true,
        ignored: false,
        session_id: Some(result.session_id),
    }))
}

async fn get_config(State(state): State<AppState>) -> Result<Json<ConfigStateResponse>, AppError> {
    let service = ConfigService::new(state.config_runtime.clone());
    Ok(Json(service.get().await?))
}

async fn reset_config(
    State(state): State<AppState>,
) -> Result<Json<ConfigMutationResponse>, AppError> {
    let service = ConfigService::new(state.config_runtime.clone());
    Ok(Json(service.reset().await?))
}

async fn apply_config(
    State(state): State<AppState>,
    Json(payload): Json<ConfigMutationRequest>,
) -> Result<Json<ConfigMutationResponse>, AppError> {
    let service = ConfigService::new(state.config_runtime.clone());
    let input = match (payload.raw, payload.config) {
        (Some(raw), None) => ConfigMutationInput::Raw(raw),
        (None, Some(config)) => ConfigMutationInput::Structured(config),
        _ => {
            return Err(AppError::bad_request(
                "exactly one of raw/config must be set",
            ))
        }
    };

    Ok(Json(service.apply(input).await?))
}

async fn restart_config(
    State(state): State<AppState>,
    Json(payload): Json<ConfigMutationRequest>,
) -> Result<Json<ConfigMutationResponse>, AppError> {
    let service = ConfigService::new(state.config_runtime.clone());
    let input = match (payload.raw, payload.config) {
        (Some(raw), None) => ConfigRestartInput::Raw(raw),
        (None, Some(config)) => ConfigRestartInput::Structured(config),
        (None, None) => ConfigRestartInput::Noop,
        _ => return Err(AppError::bad_request("raw/config payload shape is invalid")),
    };

    Ok(Json(service.restart(input).await?))
}

async fn list_skills(State(state): State<AppState>) -> Result<Json<Vec<SkillMeta>>, AppError> {
    let skills = state
        .skills
        .list()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    Ok(Json(skills))
}

async fn get_skill(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SkillDetail>, AppError> {
    let skill = state
        .skills
        .get(&id)
        .await
        .map_err(|_| AppError::not_found("skill not found"))?;
    Ok(Json(skill))
}

async fn get_upgrade_status(
    State(state): State<AppState>,
) -> Result<Json<UpgradeStatus>, AppError> {
    let service = UpgradeService::new(state.upgrades.clone());
    Ok(Json(service.status().await?))
}

async fn apply_upgrade(
    State(state): State<AppState>,
) -> Result<Json<UpgradeApplyResult>, AppError> {
    let service = UpgradeService::new(state.upgrades.clone());
    Ok(Json(service.apply().await?))
}

async fn relaunch_upgrade(
    State(state): State<AppState>,
) -> Result<Json<UpgradeRestartResult>, AppError> {
    let service = UpgradeService::new(state.upgrades.clone());
    Ok(Json(service.relaunch().await?))
}

async fn install_skill(
    State(state): State<AppState>,
    Json(payload): Json<InstallSkillRequest>,
) -> Result<Json<InstallSkillResponse>, AppError> {
    let source = payload.source.trim();
    if source.is_empty() {
        return Err(AppError::bad_request("source is required"));
    }

    let installed = install_skills_from_git_source(&state.skills_dir, source)
        .await
        .map_err(|error| AppError::bad_request(format!("install skill failed: {error}")))?;

    Ok(Json(InstallSkillResponse {
        ok: true,
        source: source.to_string(),
        installed,
    }))
}

fn chat_event_to_sse(event: ChatEvent) -> Event {
    match event {
        ChatEvent::Session { session_id } => Event::default()
            .event("session")
            .data(json!({"session_id": session_id}).to_string()),
        ChatEvent::Delta(chunk) => Event::default().event("delta").data(chunk),
        ChatEvent::Tool(tool) => tool_event_to_sse(tool),
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
