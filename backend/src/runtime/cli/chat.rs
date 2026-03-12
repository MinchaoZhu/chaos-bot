use crate::domain::chat::{ChatCommand as AgentChatCommand, ChatEvent, ToolEvent};
use clap::Args;

use super::{
    bootstrap_runtime, prompt_text, read_stdin_text, render_chat_stream_done,
    render_chat_stream_event, write_output, ChatResponse, ChatStreamDone, CliError, GlobalOptions,
    OutputMode,
};

#[derive(Debug, Args, Default)]
pub(crate) struct ChatArgs {
    #[arg(long, value_name = "ID")]
    session: Option<String>,

    #[arg(long)]
    stdin: bool,

    #[arg(long)]
    stream: bool,

    #[arg(value_name = "MESSAGE_OR_SESSION")]
    parts: Vec<String>,
}

pub(crate) async fn run(
    global: &GlobalOptions,
    args: ChatArgs,
) -> std::result::Result<(), CliError> {
    let handles = bootstrap_runtime(global)
        .await
        .map_err(CliError::from_anyhow)?;
    let (session_id, message) = resolve_chat_request(&handles, global, &args).await?;

    if args.stream && matches!(global.output, OutputMode::Json) {
        return Err(CliError::invalid_request(
            "chat --stream requires --output text or --output jsonl",
        ));
    }

    let mut tool_events = Vec::new();
    let mut stream_write_error = None::<String>;
    let result = handles
        .context
        .chat_service()
        .run_stream(
            AgentChatCommand {
                session_id,
                message,
            },
            |event| {
                if let ChatEvent::Tool(tool) = &event {
                    tool_events.push(tool.clone());
                }

                if args.stream && stream_write_error.is_none() {
                    if let Err(error) = render_chat_stream_event(global.output, &event) {
                        stream_write_error = Some(error.to_string());
                    }
                }
            },
        )
        .await
        .map_err(CliError::from_app_error)?;

    if let Some(message) = stream_write_error {
        return Err(CliError::execution_failure(message));
    }

    if args.stream {
        let done = ChatStreamDone {
            session_id: result.session_id,
            assistant_message: result.assistant_message,
            finish_reason: result.finish_reason,
            usage: result.usage,
            tool_error_count: tool_events
                .iter()
                .filter(|event| event.result.is_error)
                .count(),
        };
        render_chat_stream_done(global.output, &done)
            .map_err(|error| CliError::execution_failure(error.to_string()))?;
        if let Some(error) = collect_tool_failure(&tool_events) {
            return Err(error);
        }
        return Ok(());
    }

    let tool_failure = collect_tool_failure(&tool_events);
    let response = ChatResponse {
        session_id: result.session_id,
        assistant_message: result.assistant_message,
        finish_reason: result.finish_reason,
        usage: result.usage,
        tool_events,
    };
    write_output(
        global.output,
        &response,
        super::output::render_chat_response_text,
    )?;
    if let Some(error) = tool_failure {
        return Err(error);
    }
    Ok(())
}

async fn resolve_chat_request(
    handles: &super::RuntimeHandles,
    global: &GlobalOptions,
    args: &ChatArgs,
) -> std::result::Result<(Option<String>, String), CliError> {
    if let Some(session_id) = &args.session {
        if args.stdin {
            let message = read_stdin_text("chat message").map_err(CliError::from_anyhow)?;
            return Ok((Some(session_id.clone()), message));
        }
        let message = if args.parts.is_empty() {
            prompt_text("message", global.non_interactive).map_err(CliError::from_anyhow)?
        } else {
            args.parts.join(" ")
        };
        return Ok((Some(session_id.clone()), message));
    }

    if args.stdin {
        let message = read_stdin_text("chat message").map_err(CliError::from_anyhow)?;
        return Ok((None, message));
    }

    if args.parts.is_empty() {
        let message =
            prompt_text("message", global.non_interactive).map_err(CliError::from_anyhow)?;
        return Ok((None, message));
    }

    if args.parts.len() > 1 {
        let candidate = &args.parts[0];
        let existing = handles
            .context
            .sessions
            .get(candidate)
            .await
            .map_err(CliError::from_anyhow)?;
        if existing.is_some() {
            return Ok((Some(candidate.clone()), args.parts[1..].join(" ")));
        }
        if looks_like_session_id(candidate) {
            return Err(CliError::not_found(format!(
                "session '{candidate}' not found; use --session {candidate} to create or continue an explicit session id"
            )));
        }
    }

    Ok((None, args.parts.join(" ")))
}

fn looks_like_session_id(candidate: &str) -> bool {
    const MIN_SESSION_PREFIX_LEN: usize = 8;

    uuid::Uuid::parse_str(candidate).is_ok()
        || (candidate.len() >= MIN_SESSION_PREFIX_LEN
            && candidate
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() || ch == '-')
            && candidate.chars().any(|ch| ch.is_ascii_hexdigit()))
}

fn collect_tool_failure(tool_events: &[ToolEvent]) -> Option<CliError> {
    tool_events.iter().find_map(|event| {
        event.result.is_error.then(|| {
            CliError::tool_failure(format!(
                "tool '{}' failed: {}",
                event.call.name, event.result.output
            ))
        })
    })
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    #[test]
    fn chat_accepts_inline_message() {
        let cli = super::super::Cli::parse_from(["chaos-bot", "chat", "hello", "world"]);
        let super::super::CommandGroup::Chat(args) = cli.command.expect("chat command") else {
            panic!("expected chat command");
        };
        assert_eq!(args.parts, vec!["hello".to_string(), "world".to_string()]);
    }

    #[test]
    fn looks_like_session_id_matches_uuid_and_hex_prefixes() {
        assert!(super::looks_like_session_id(
            "123e4567-e89b-12d3-a456-426614174000"
        ));
        assert!(super::looks_like_session_id("deadbeef"));
        assert!(!super::looks_like_session_id("follow"));
        assert!(!super::looks_like_session_id("session-1"));
    }
}
