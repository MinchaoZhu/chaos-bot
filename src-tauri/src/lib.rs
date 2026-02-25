use futures_util::StreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::Emitter;

const CHAT_STREAM_EVENT: &str = "chaos://chat-event";

#[derive(Debug, Clone, Serialize)]
struct RuntimeError {
    code: &'static str,
    message: String,
}

impl RuntimeError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ChatRequest {
    session_id: Option<String>,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    now: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMessage {
    role: String,
    content: Option<String>,
    tool_name: Option<String>,
    tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionState {
    id: String,
    messages: Vec<SessionMessage>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigStateResponse {
    config_path: String,
    backup1_path: String,
    backup2_path: String,
    config_format: String,
    running: Value,
    disk: Value,
    raw: String,
    disk_parse_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigMutationResponse {
    ok: bool,
    action: String,
    restart_scheduled: bool,
    state: ConfigStateResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ConfigMutationRequest {
    raw: Option<String>,
    config: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
struct ChatStreamEnvelope {
    stream_id: String,
    event: String,
    data: Value,
}

fn normalize_base_url(base_url: &str) -> String {
    base_url.trim_end_matches('/').to_string()
}

fn map_status_error(status: StatusCode) -> RuntimeError {
    let code = match status.as_u16() {
        400 => "HTTP_BAD_REQUEST",
        401 | 403 => "HTTP_UNAUTHORIZED",
        404 => "HTTP_NOT_FOUND",
        500..=599 => "HTTP_SERVER_ERROR",
        _ => "UNKNOWN",
    };
    RuntimeError::new(code, format!("HTTP {}", status.as_u16()))
}

async fn get_json<T: for<'de> Deserialize<'de>>(url: String) -> Result<T, RuntimeError> {
    let response = reqwest::get(url)
        .await
        .map_err(|error| RuntimeError::new("NETWORK_UNAVAILABLE", error.to_string()))?;

    if !response.status().is_success() {
        return Err(map_status_error(response.status()));
    }

    response
        .json::<T>()
        .await
        .map_err(|error| RuntimeError::new("UNKNOWN", error.to_string()))
}

async fn post_json<T: for<'de> Deserialize<'de>, B: Serialize>(
    url: String,
    body: &B,
) -> Result<T, RuntimeError> {
    let response = reqwest::Client::new()
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|error| RuntimeError::new("NETWORK_UNAVAILABLE", error.to_string()))?;

    if !response.status().is_success() {
        return Err(map_status_error(response.status()));
    }

    response
        .json::<T>()
        .await
        .map_err(|error| RuntimeError::new("UNKNOWN", error.to_string()))
}

#[tauri::command]
async fn health(base_url: String) -> Result<HealthResponse, RuntimeError> {
    get_json(format!("{}/api/health", normalize_base_url(&base_url))).await
}

#[tauri::command]
async fn list_sessions(base_url: String) -> Result<Vec<SessionState>, RuntimeError> {
    get_json(format!("{}/api/sessions", normalize_base_url(&base_url))).await
}

#[tauri::command]
async fn create_session(base_url: String) -> Result<SessionState, RuntimeError> {
    let url = format!("{}/api/sessions", normalize_base_url(&base_url));
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .send()
        .await
        .map_err(|error| RuntimeError::new("NETWORK_UNAVAILABLE", error.to_string()))?;

    if !response.status().is_success() {
        return Err(map_status_error(response.status()));
    }

    response
        .json::<SessionState>()
        .await
        .map_err(|error| RuntimeError::new("UNKNOWN", error.to_string()))
}

#[tauri::command]
async fn get_session(base_url: String, session_id: String) -> Result<SessionState, RuntimeError> {
    get_json(format!(
        "{}/api/sessions/{}",
        normalize_base_url(&base_url),
        session_id
    ))
    .await
}

#[tauri::command]
async fn delete_session(base_url: String, session_id: String) -> Result<(), RuntimeError> {
    let url = format!("{}/api/sessions/{}", normalize_base_url(&base_url), session_id);
    let response = reqwest::Client::new()
        .delete(url)
        .send()
        .await
        .map_err(|error| RuntimeError::new("NETWORK_UNAVAILABLE", error.to_string()))?;

    if !response.status().is_success() {
        return Err(map_status_error(response.status()));
    }

    Ok(())
}

#[tauri::command]
async fn get_config(base_url: String) -> Result<ConfigStateResponse, RuntimeError> {
    get_json(format!("{}/api/config", normalize_base_url(&base_url))).await
}

#[tauri::command]
async fn apply_config(
    base_url: String,
    request: ConfigMutationRequest,
) -> Result<ConfigMutationResponse, RuntimeError> {
    post_json(
        format!("{}/api/config/apply", normalize_base_url(&base_url)),
        &request,
    )
    .await
}

#[tauri::command]
async fn reset_config(base_url: String) -> Result<ConfigMutationResponse, RuntimeError> {
    post_json(
        format!("{}/api/config/reset", normalize_base_url(&base_url)),
        &json!({}),
    )
    .await
}

#[tauri::command]
async fn restart_config(
    base_url: String,
    request: Option<ConfigMutationRequest>,
) -> Result<ConfigMutationResponse, RuntimeError> {
    let payload = request.unwrap_or_default();
    post_json(
        format!("{}/api/config/restart", normalize_base_url(&base_url)),
        &payload,
    )
    .await
}

fn parse_sse_block(block: &str) -> Option<(String, Value)> {
    let mut event = String::from("delta");
    let mut data = String::new();

    for line in block.lines() {
        if let Some(name) = line.strip_prefix("event:") {
            event = name.trim().to_string();
        }
        if let Some(payload) = line.strip_prefix("data:") {
            data.push_str(payload.trim());
        }
    }

    if data.is_empty() {
        return None;
    }

    if ["session", "tool_call", "done", "error"].contains(&event.as_str()) {
        let json = serde_json::from_str::<Value>(&data).ok()?;
        return Some((event, json));
    }

    Some((event, Value::String(data)))
}

#[tauri::command]
async fn chat_stream(
    window: tauri::Window,
    base_url: String,
    request: ChatRequest,
    stream_id: String,
) -> Result<(), RuntimeError> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/chat", normalize_base_url(&base_url)))
        .json(&request)
        .send()
        .await
        .map_err(|error| RuntimeError::new("NETWORK_UNAVAILABLE", error.to_string()))?;

    if !response.status().is_success() {
        return Err(map_status_error(response.status()));
    }

    let mut bytes_stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = bytes_stream.next().await {
        let chunk = chunk.map_err(|error| RuntimeError::new("NETWORK_UNAVAILABLE", error.to_string()))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        let mut blocks: Vec<String> = buffer.split("\n\n").map(str::to_owned).collect();
        let remain = blocks.pop().unwrap_or_default();
        buffer = remain;

        for block in blocks {
            if block.trim().is_empty() || block.contains("keepalive") {
                continue;
            }

            match parse_sse_block(&block) {
                Some((event, data)) => {
                    let payload = ChatStreamEnvelope {
                        stream_id: stream_id.clone(),
                        event,
                        data,
                    };
                    if let Err(error) = window.emit(CHAT_STREAM_EVENT, payload) {
                        return Err(RuntimeError::new("UNKNOWN", error.to_string()));
                    }
                }
                None => {
                    return Err(RuntimeError::new(
                        "SSE_PROTOCOL_ERROR",
                        "Invalid SSE frame received",
                    ));
                }
            }
        }
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            health,
            list_sessions,
            create_session,
            get_session,
            delete_session,
            get_config,
            apply_config,
            reset_config,
            restart_config,
            chat_stream
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
