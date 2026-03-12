use crate::domain::chat::{ChatEvent, ToolEvent};
use crate::domain::config::{ConfigMutationResponse, ConfigStateResponse};
use crate::domain::skills::{SkillDetail, SkillMeta};
use crate::domain::types::{SessionState, Usage};
use crate::domain::upgrade::{UpgradeApplyResult, UpgradeRestartResult, UpgradeStatus};
use serde::Serialize;
use serde_json::json;
use std::io::{self, Write};

use super::{CliError, OutputMode};

#[derive(Serialize)]
pub(crate) struct ChatResponse {
    pub session_id: String,
    pub assistant_message: String,
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
    pub tool_events: Vec<ToolEvent>,
}

#[derive(Serialize)]
pub(crate) struct DeleteResponse<'a> {
    pub ok: bool,
    pub id: &'a str,
}

#[derive(Serialize)]
pub(crate) struct InstallSkillResponse<'a> {
    pub ok: bool,
    pub source: &'a str,
    pub installed: &'a [SkillMeta],
}

#[derive(Clone, Serialize)]
pub(crate) struct SessionSummary {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub message_count: usize,
}

impl From<&SessionState> for SessionSummary {
    fn from(value: &SessionState) -> Self {
        Self {
            id: value.id.clone(),
            created_at: value.created_at,
            updated_at: value.updated_at,
            message_count: value.messages.len(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct ChatStreamDone {
    pub session_id: String,
    pub assistant_message: String,
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
    pub tool_error_count: usize,
}

#[derive(Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum ChatStreamJsonEvent {
    Session {
        session_id: String,
    },
    Delta {
        chunk: String,
    },
    ToolCall {
        id: String,
        name: String,
        args: serde_json::Value,
        output: String,
        is_error: bool,
    },
    Done {
        session_id: String,
        assistant_message: String,
        finish_reason: Option<String>,
        usage: Option<Usage>,
        tool_error_count: usize,
    },
}

pub(crate) fn write_output<T, F>(
    mode: OutputMode,
    value: &T,
    render_text: F,
) -> std::result::Result<(), CliError>
where
    T: Serialize,
    F: FnOnce(&T) -> String,
{
    match mode {
        OutputMode::Text => write_text(render_text(value)),
        OutputMode::Json => write_json_pretty(value),
        OutputMode::Jsonl => write_json_line(value),
    }
}

pub(crate) fn render_chat_stream_event(mode: OutputMode, event: &ChatEvent) -> io::Result<()> {
    match mode {
        OutputMode::Text => {
            let mut stdout = io::stdout().lock();
            match event {
                ChatEvent::Session { session_id } => writeln!(stdout, "session: {session_id}")?,
                ChatEvent::Delta(chunk) => {
                    writeln!(stdout, "delta: {}", serde_json::to_string(chunk).unwrap())?
                }
                ChatEvent::Tool(tool) => {
                    writeln!(stdout, "tool_call: {}", chat_tool_payload(tool))?
                }
            }
            stdout.flush()
        }
        OutputMode::Jsonl => {
            let payload = match event {
                ChatEvent::Session { session_id } => ChatStreamJsonEvent::Session {
                    session_id: session_id.clone(),
                },
                ChatEvent::Delta(chunk) => ChatStreamJsonEvent::Delta {
                    chunk: chunk.clone(),
                },
                ChatEvent::Tool(tool) => ChatStreamJsonEvent::ToolCall {
                    id: tool.call.id.clone(),
                    name: tool.call.name.clone(),
                    args: tool.call.arguments.clone(),
                    output: tool.result.output.clone(),
                    is_error: tool.result.is_error,
                },
            };
            let mut stdout = io::stdout().lock();
            serde_json::to_writer(&mut stdout, &payload)?;
            stdout.write_all(b"\n")?;
            stdout.flush()
        }
        OutputMode::Json => Ok(()),
    }
}

pub(crate) fn render_chat_stream_done(mode: OutputMode, done: &ChatStreamDone) -> io::Result<()> {
    match mode {
        OutputMode::Text => {
            let mut stdout = io::stdout().lock();
            writeln!(
                stdout,
                "done: {}",
                json!({
                    "session_id": done.session_id,
                    "assistant_message": done.assistant_message,
                    "finish_reason": done.finish_reason,
                    "usage": done.usage,
                    "tool_error_count": done.tool_error_count,
                })
            )?;
            stdout.flush()
        }
        OutputMode::Jsonl => {
            let payload = ChatStreamJsonEvent::Done {
                session_id: done.session_id.clone(),
                assistant_message: done.assistant_message.clone(),
                finish_reason: done.finish_reason.clone(),
                usage: done.usage.clone(),
                tool_error_count: done.tool_error_count,
            };
            let mut stdout = io::stdout().lock();
            serde_json::to_writer(&mut stdout, &payload)?;
            stdout.write_all(b"\n")?;
            stdout.flush()
        }
        OutputMode::Json => Ok(()),
    }
}

pub(crate) fn render_chat_response_text(response: &ChatResponse) -> String {
    let mut out = String::new();
    out.push_str(&format!("session: {}\n", response.session_id));
    out.push_str(&format!(
        "finish_reason: {}\n",
        response.finish_reason.as_deref().unwrap_or("unknown")
    ));
    if let Some(usage) = &response.usage {
        out.push_str(&format!(
            "usage: prompt={} completion={} total={}\n",
            usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
        ));
    }
    if !response.tool_events.is_empty() {
        out.push_str(&format!("tool_events: {}\n", response.tool_events.len()));
    }
    out.push('\n');
    out.push_str(&response.assistant_message);
    out
}

pub(crate) fn render_sessions_text(summaries: &[SessionSummary]) -> String {
    if summaries.is_empty() {
        return "no sessions".to_string();
    }
    summaries
        .iter()
        .map(|session| {
            format!(
                "{}\t{}\t{}\t{} messages",
                session.id,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                session.message_count
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_session_detail_text(session: &SessionState) -> String {
    let mut out = String::new();
    out.push_str(&format!("id: {}\n", session.id));
    out.push_str(&format!(
        "created_at: {}\n",
        session.created_at.to_rfc3339()
    ));
    out.push_str(&format!(
        "updated_at: {}\n",
        session.updated_at.to_rfc3339()
    ));
    out.push_str(&format!("message_count: {}\n", session.messages.len()));
    if !session.messages.is_empty() {
        out.push_str("\nmessages:\n");
        for message in &session.messages {
            out.push_str(&format!("- {:?}: {}\n", message.role, message.content));
        }
    }
    out.trim_end().to_string()
}

pub(crate) fn render_delete_text(response: &DeleteResponse<'_>) -> String {
    format!("deleted: {}", response.id)
}

pub(crate) fn render_config_state_text(response: &ConfigStateResponse) -> String {
    let mut out = String::new();
    out.push_str(&format!("config_path: {}\n", response.config_path));
    out.push_str(&format!("backup1_path: {}\n", response.backup1_path));
    out.push_str(&format!("backup2_path: {}\n", response.backup2_path));
    out.push_str(&format!("config_format: {}\n", response.config_format));
    if let Some(error) = &response.disk_parse_error {
        out.push_str(&format!("disk_parse_error: {error}\n"));
    }
    out.push_str("\nraw:\n");
    out.push_str(&response.raw);
    out.trim_end().to_string()
}

pub(crate) fn render_config_mutation_text(response: &ConfigMutationResponse) -> String {
    let mut out = String::new();
    out.push_str(&format!("ok: {}\n", response.ok));
    out.push_str(&format!("action: {}\n", response.action));
    out.push_str(&format!(
        "restart_scheduled: {}\n\n",
        response.restart_scheduled
    ));
    out.push_str(&render_config_state_text(&response.state));
    out
}

pub(crate) fn render_skills_text(skills: &[SkillMeta]) -> String {
    if skills.is_empty() {
        return "no skills installed".to_string();
    }
    skills
        .iter()
        .map(|skill| format!("{}\t{}\t{}", skill.id, skill.name, skill.description))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn render_skill_detail_text(skill: &SkillDetail) -> String {
    let mut out = String::new();
    out.push_str(&format!("id: {}\n", skill.meta.id));
    out.push_str(&format!("name: {}\n", skill.meta.name));
    out.push_str(&format!("description: {}\n", skill.meta.description));
    out.push_str("\nbody:\n");
    out.push_str(&skill.body);
    out.trim_end().to_string()
}

pub(crate) fn render_skill_install_text(response: &InstallSkillResponse<'_>) -> String {
    let mut out = String::new();
    out.push_str(&format!("ok: {}\n", response.ok));
    out.push_str(&format!("source: {}\n", response.source));
    if response.installed.is_empty() {
        out.push_str("installed: none");
    } else {
        out.push_str("installed:\n");
        for skill in response.installed {
            out.push_str(&format!(
                "- {}\t{}\t{}\n",
                skill.id, skill.name, skill.description
            ));
        }
    }
    out.trim_end().to_string()
}

pub(crate) fn render_upgrade_status_text(status: &UpgradeStatus) -> String {
    let mut out = String::new();
    out.push_str(&format!("supported: {}\n", status.supported));
    out.push_str(&format!(
        "upgrade_available: {}\n",
        status.upgrade_available
    ));
    out.push_str(&format!(
        "current_version: {}\n",
        status.current_version.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "latest_version: {}\n",
        status.latest_version.as_deref().unwrap_or("unknown")
    ));
    if let Some(reason) = &status.reason {
        out.push_str(&format!("reason: {reason}\n"));
    }
    out.trim_end().to_string()
}

pub(crate) fn render_upgrade_apply_text(result: &UpgradeApplyResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("ok: {}\n", result.ok));
    out.push_str(&format!("action: {}\n", result.action));
    out.push_str(&format!("message: {}\n", result.message));
    out.push_str(&format!(
        "target_version: {}\n",
        result.target_version.as_deref().unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "relaunch_required: {}\n",
        result.relaunch_required
    ));
    out.trim_end().to_string()
}

pub(crate) fn render_upgrade_restart_text(result: &UpgradeRestartResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("ok: {}\n", result.ok));
    out.push_str(&format!("action: {}\n", result.action));
    out.push_str(&format!("message: {}\n", result.message));
    if let Some(path) = &result.launcher_path {
        out.push_str(&format!("launcher_path: {path}\n"));
    }
    out.trim_end().to_string()
}

fn write_text(text: String) -> std::result::Result<(), CliError> {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(text.as_bytes())
        .map_err(|error| CliError::execution_failure(error.to_string()))?;
    if !text.ends_with('\n') {
        stdout
            .write_all(b"\n")
            .map_err(|error| CliError::execution_failure(error.to_string()))?;
    }
    stdout
        .flush()
        .map_err(|error| CliError::execution_failure(error.to_string()))
}

fn write_json_pretty<T: Serialize>(value: &T) -> std::result::Result<(), CliError> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, value)
        .map_err(|error| CliError::execution_failure(error.to_string()))?;
    stdout
        .write_all(b"\n")
        .map_err(|error| CliError::execution_failure(error.to_string()))?;
    stdout
        .flush()
        .map_err(|error| CliError::execution_failure(error.to_string()))
}

fn write_json_line<T: Serialize>(value: &T) -> std::result::Result<(), CliError> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)
        .map_err(|error| CliError::execution_failure(error.to_string()))?;
    stdout
        .write_all(b"\n")
        .map_err(|error| CliError::execution_failure(error.to_string()))?;
    stdout
        .flush()
        .map_err(|error| CliError::execution_failure(error.to_string()))
}

fn chat_tool_payload(tool: &ToolEvent) -> serde_json::Value {
    json!({
        "id": tool.call.id,
        "name": tool.call.name,
        "args": tool.call.arguments,
        "output": tool.result.output,
        "is_error": tool.result.is_error,
    })
}
