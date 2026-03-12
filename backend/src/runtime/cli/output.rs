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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{Message, Role, ToolCall, ToolResult};
    use crate::infrastructure::config::{AgentFileConfig, AgentLlmConfig, AgentLoggingConfig};
    use serde_json::json;
    use std::path::PathBuf;

    fn sample_tool_event() -> ToolEvent {
        ToolEvent {
            call: ToolCall {
                id: "tool-1".to_string(),
                name: "bash".to_string(),
                arguments: json!({"command": "pwd"}),
            },
            result: ToolResult {
                tool_call_id: "tool-1".to_string(),
                name: "bash".to_string(),
                output: "/tmp/workspace".to_string(),
                is_error: false,
            },
        }
    }

    fn sample_usage() -> Usage {
        Usage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        }
    }

    fn sample_config() -> AgentFileConfig {
        AgentFileConfig {
            workspace: Some(PathBuf::from("workspace")),
            logging: AgentLoggingConfig {
                level: Some("info".to_string()),
                retention_days: Some(7),
                directory: Some(PathBuf::from("logs")),
            },
            llm: AgentLlmConfig {
                provider: Some("mock".to_string()),
                model: Some("mock-model".to_string()),
                temperature: Some(0.2),
                max_tokens: Some(512),
                max_iterations: Some(6),
                token_budget: Some(2048),
            },
            search: Default::default(),
            secrets: Default::default(),
        }
    }

    fn sample_config_state() -> ConfigStateResponse {
        let config = sample_config();
        ConfigStateResponse {
            config_path: "/tmp/config.json".to_string(),
            backup1_path: "/tmp/config.json.bak1".to_string(),
            backup2_path: "/tmp/config.json.bak2".to_string(),
            config_format: "config.json".to_string(),
            running: config.clone(),
            disk: config,
            raw: "{\n  \"llm\": {}\n}\n".to_string(),
            disk_parse_error: Some("invalid config json".to_string()),
        }
    }

    fn sample_upgrade_status() -> UpgradeStatus {
        UpgradeStatus {
            supported: true,
            current_version: Some("0.1.0".to_string()),
            latest_version: Some("0.1.1".to_string()),
            latest_tag_name: Some("v0.1.1".to_string()),
            upgrade_available: true,
            install_prefix: Some("/opt/chaos-bot".to_string()),
            current_release_root: Some("/opt/chaos-bot/releases/0.1.0".to_string()),
            repository: Some("test/chaos-bot".to_string()),
            latest_release_url: Some("https://example.invalid/latest".to_string()),
            download_url: Some("https://example.invalid/download".to_string()),
            reason: Some("checksum mismatch".to_string()),
        }
    }

    #[test]
    fn write_output_supports_all_modes() {
        let response = ChatResponse {
            session_id: "session-1".to_string(),
            assistant_message: "hello".to_string(),
            finish_reason: Some("stop".to_string()),
            usage: Some(sample_usage()),
            tool_events: vec![sample_tool_event()],
        };

        assert!(write_output(OutputMode::Text, &response, render_chat_response_text).is_ok());
        assert!(write_output(OutputMode::Json, &response, render_chat_response_text).is_ok());
        assert!(write_output(OutputMode::Jsonl, &response, render_chat_response_text).is_ok());
    }

    #[test]
    fn render_chat_stream_event_and_done_support_all_output_modes() {
        let tool = sample_tool_event();
        let done = ChatStreamDone {
            session_id: "session-1".to_string(),
            assistant_message: "final answer".to_string(),
            finish_reason: Some("stop".to_string()),
            usage: Some(sample_usage()),
            tool_error_count: 1,
        };

        for mode in [OutputMode::Text, OutputMode::Json, OutputMode::Jsonl] {
            assert!(render_chat_stream_event(
                mode,
                &ChatEvent::Session {
                    session_id: "session-1".to_string(),
                }
            )
            .is_ok());
            assert!(render_chat_stream_event(mode, &ChatEvent::Delta("chunk".to_string())).is_ok());
            assert!(render_chat_stream_event(mode, &ChatEvent::Tool(tool.clone())).is_ok());
            assert!(render_chat_stream_done(mode, &done).is_ok());
        }
    }

    #[test]
    fn render_chat_response_text_includes_usage_and_tool_count() {
        let rendered = render_chat_response_text(&ChatResponse {
            session_id: "session-1".to_string(),
            assistant_message: "assistant reply".to_string(),
            finish_reason: Some("stop".to_string()),
            usage: Some(sample_usage()),
            tool_events: vec![sample_tool_event()],
        });

        assert!(rendered.contains("session: session-1"));
        assert!(rendered.contains("finish_reason: stop"));
        assert!(rendered.contains("usage: prompt=10 completion=20 total=30"));
        assert!(rendered.contains("tool_events: 1"));
        assert!(rendered.ends_with("assistant reply"));
    }

    #[test]
    fn render_sessions_and_session_detail_text_cover_empty_and_messages() {
        assert_eq!(render_sessions_text(&[]), "no sessions");

        let mut session = SessionState::new("session-1");
        session.push_message(Message::user("hello"));
        session.push_message(Message::assistant("world"));
        let summary = SessionSummary::from(&session);
        let rendered_list = render_sessions_text(&[summary]);
        assert!(rendered_list.contains("session-1"));
        assert!(rendered_list.contains("2 messages"));

        let rendered_detail = render_session_detail_text(&session);
        assert!(rendered_detail.contains("id: session-1"));
        assert!(rendered_detail.contains("message_count: 2"));
        assert!(rendered_detail.contains("- User: hello"));
        assert!(rendered_detail.contains("- Assistant: world"));
    }

    #[test]
    fn render_config_and_delete_text_cover_mutation_contract() {
        let state = sample_config_state();
        let rendered_state = render_config_state_text(&state);
        assert!(rendered_state.contains("config_path: /tmp/config.json"));
        assert!(rendered_state.contains("disk_parse_error: invalid config json"));
        assert!(rendered_state.contains("\"llm\""));

        let rendered_mutation = render_config_mutation_text(&ConfigMutationResponse {
            ok: true,
            action: "restart",
            restart_scheduled: false,
            state,
        });
        assert!(rendered_mutation.contains("ok: true"));
        assert!(rendered_mutation.contains("action: restart"));
        assert!(rendered_mutation.contains("restart_scheduled: false"));
        assert!(rendered_mutation.contains("backup1_path: /tmp/config.json.bak1"));

        assert_eq!(
            render_delete_text(&DeleteResponse {
                ok: true,
                id: "session-1",
            }),
            "deleted: session-1"
        );
    }

    #[test]
    fn render_skills_text_detail_and_install_text_cover_both_paths() {
        let skill = SkillMeta {
            id: "hello-skill".to_string(),
            name: "Hello Skill".to_string(),
            description: "Used for tests".to_string(),
        };

        assert_eq!(render_skills_text(&[]), "no skills installed");
        let rendered_list = render_skills_text(std::slice::from_ref(&skill));
        assert!(rendered_list.contains("hello-skill\tHello Skill\tUsed for tests"));

        let rendered_detail = render_skill_detail_text(&SkillDetail {
            meta: skill.clone(),
            body: "Body text".to_string(),
        });
        assert!(rendered_detail.contains("id: hello-skill"));
        assert!(rendered_detail.contains("body:\nBody text"));

        let rendered_empty_install = render_skill_install_text(&InstallSkillResponse {
            ok: true,
            source: "repo.git",
            installed: &[],
        });
        assert!(rendered_empty_install.contains("installed: none"));

        let rendered_install = render_skill_install_text(&InstallSkillResponse {
            ok: true,
            source: "repo.git",
            installed: &[skill],
        });
        assert!(rendered_install.contains("source: repo.git"));
        assert!(rendered_install.contains("- hello-skill\tHello Skill\tUsed for tests"));
    }

    #[test]
    fn render_upgrade_texts_and_payload_include_optional_fields() {
        let status = sample_upgrade_status();
        let rendered_status = render_upgrade_status_text(&status);
        assert!(rendered_status.contains("supported: true"));
        assert!(rendered_status.contains("upgrade_available: true"));
        assert!(rendered_status.contains("reason: checksum mismatch"));

        let rendered_apply = render_upgrade_apply_text(&UpgradeApplyResult {
            ok: true,
            action: "upgrade",
            current_version: status.current_version.clone(),
            target_version: status.latest_version.clone(),
            launcher_path: Some("/opt/chaos-bot/bin/chaos-bot".to_string()),
            installed_release_root: Some("/opt/chaos-bot/releases/0.1.1".to_string()),
            relaunch_required: true,
            message: "installed new release".to_string(),
            status: status.clone(),
        });
        assert!(rendered_apply.contains("action: upgrade"));
        assert!(rendered_apply.contains("target_version: 0.1.1"));
        assert!(rendered_apply.contains("relaunch_required: true"));

        let rendered_restart = render_upgrade_restart_text(&UpgradeRestartResult {
            ok: true,
            action: "relaunch",
            launcher_path: Some("/opt/chaos-bot/bin/chaos-bot".to_string()),
            target_version: status.latest_version,
            message: "restart scheduled".to_string(),
        });
        assert!(rendered_restart.contains("action: relaunch"));
        assert!(rendered_restart.contains("launcher_path: /opt/chaos-bot/bin/chaos-bot"));

        let payload = chat_tool_payload(&sample_tool_event());
        assert_eq!(payload["name"], "bash");
        assert_eq!(payload["args"]["command"], "pwd");
        assert_eq!(payload["output"], "/tmp/workspace");
    }

    #[test]
    fn session_summary_copies_session_metadata() {
        let mut session = SessionState::new("session-2");
        session.messages = vec![Message {
            role: Role::User,
            content: "hello".to_string(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }];

        let summary = SessionSummary::from(&session);
        assert_eq!(summary.id, "session-2");
        assert_eq!(summary.message_count, 1);
    }
}
