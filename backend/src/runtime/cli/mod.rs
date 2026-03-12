mod chat;
mod config;
mod output;
mod sessions;
mod skills;
mod upgrade;

use anyhow::{bail, Context, Result};
use clap::{error::ErrorKind, CommandFactory, Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::ffi::OsString;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use crate::domain::{AppError, ErrorCode};
use crate::infrastructure::logging::{init_logging, LoggingRuntime};
use crate::runtime::config_runtime::RestartMode;
use crate::runtime::{build_context, load_runtime_config, AppContext};

pub(crate) use output::{
    render_chat_stream_done, render_chat_stream_event, write_output, ChatResponse, ChatStreamDone,
    DeleteResponse, InstallSkillResponse, SessionSummary,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputMode {
    Text,
    Json,
    Jsonl,
}

#[derive(Debug, Parser)]
#[command(name = "chaos-bot", about = "CLI-first runtime for chaos-bot")]
pub struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    #[arg(long, global = true, value_name = "PATH")]
    workspace: Option<PathBuf>,

    #[arg(long, global = true, value_enum, default_value_t = OutputMode::Text)]
    output: OutputMode,

    #[arg(long, global = true)]
    non_interactive: bool,

    #[command(subcommand)]
    command: Option<CommandGroup>,
}

#[derive(Debug, Subcommand)]
enum CommandGroup {
    Chat(chat::ChatArgs),
    Sessions(sessions::SessionsArgs),
    Config(config::ConfigArgs),
    Skills(skills::SkillsArgs),
    Upgrade(upgrade::UpgradeArgs),
}

struct RuntimeHandles {
    context: AppContext,
    _logging: LoggingRuntime,
}

#[derive(Clone, Debug)]
struct GlobalOptions {
    config: Option<PathBuf>,
    workspace: Option<PathBuf>,
    output: OutputMode,
    non_interactive: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CliErrorKind {
    InvalidRequest,
    NotFound,
    ServiceUnavailable,
    ConfigValidation,
    NetworkFailure,
    ProviderFailure,
    ToolFailure,
    ExecutionFailure,
}

impl CliErrorKind {
    fn code(self) -> u8 {
        match self {
            Self::InvalidRequest => 2,
            Self::NotFound => 3,
            Self::ServiceUnavailable => 4,
            Self::ConfigValidation => 5,
            Self::NetworkFailure => 10,
            Self::ProviderFailure => 11,
            Self::ToolFailure => 12,
            Self::ExecutionFailure => 20,
        }
    }
}

#[derive(Debug)]
pub(crate) struct CliError {
    kind: CliErrorKind,
    message: String,
}

impl CliError {
    pub(crate) fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::InvalidRequest, message)
    }

    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::NotFound, message)
    }

    pub(crate) fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::ServiceUnavailable, message)
    }

    pub(crate) fn config_validation(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::ConfigValidation, message)
    }

    pub(crate) fn network_failure(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::NetworkFailure, message)
    }

    pub(crate) fn provider_failure(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::ProviderFailure, message)
    }

    pub(crate) fn tool_failure(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::ToolFailure, message)
    }

    pub(crate) fn execution_failure(message: impl Into<String>) -> Self {
        Self::new(CliErrorKind::ExecutionFailure, message)
    }

    fn new(kind: CliErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub(crate) fn from_app_error(error: AppError) -> Self {
        match error.code() {
            ErrorCode::InvalidRequest => Self::invalid_request(error.message()),
            ErrorCode::NotFound => Self::not_found(error.message()),
            ErrorCode::ServiceUnavailable => Self::service_unavailable(error.message()),
            ErrorCode::Internal => classify_error_message(error.message()),
        }
    }

    pub(crate) fn from_anyhow(error: anyhow::Error) -> Self {
        classify_error_message(&error.to_string())
    }

    fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.kind.code())
    }
}

pub async fn run(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => return render_parse_error(error),
    };

    let output = cli.output;
    match run_parsed(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            render_cli_error(output, &error);
            error.exit_code()
        }
    }
}

async fn run_parsed(cli: Cli) -> std::result::Result<(), CliError> {
    let global = GlobalOptions {
        config: cli.config.clone(),
        workspace: cli.workspace.clone(),
        output: cli.output,
        non_interactive: cli.non_interactive,
    };

    match cli.command {
        Some(CommandGroup::Chat(args)) => chat::run(&global, args).await,
        Some(CommandGroup::Sessions(args)) => sessions::run(&global, args).await,
        Some(CommandGroup::Config(args)) => config::run(&global, args).await,
        Some(CommandGroup::Skills(args)) => skills::run(&global, args).await,
        Some(CommandGroup::Upgrade(args)) => upgrade::run(&global, args).await,
        None => print_help(),
    }
}

fn print_help() -> std::result::Result<(), CliError> {
    let mut command = Cli::command();
    command
        .print_help()
        .map_err(|error| CliError::execution_failure(error.to_string()))?;
    println!();
    Ok(())
}

async fn bootstrap_runtime(global: &GlobalOptions) -> Result<RuntimeHandles> {
    let loaded = load_runtime_config(global.config.as_deref(), global.workspace.as_deref())?;
    let config = loaded.app;

    let logging_runtime = init_logging(&config)?;
    tracing::info!(
        workspace = %config.workspace.display(),
        config_file = %config.config_path.display(),
        log_dir = %config.log_dir.display(),
        log_file = %logging_runtime.log_file.display(),
        log_level = %config.log_level,
        retention_days = config.log_retention_days,
        "chaos-bot logging initialized"
    );

    let context = build_context(&config, loaded.file, RestartMode::Disabled).await?;
    Ok(RuntimeHandles {
        context,
        _logging: logging_runtime,
    })
}

pub(crate) fn read_stdin_text(label: &str) -> Result<String> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .with_context(|| format!("failed to read stdin for {label}"))?;
    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        bail!("{label} is empty");
    }
    Ok(trimmed.to_string())
}

pub(crate) fn prompt_text(label: &str, non_interactive: bool) -> Result<String> {
    if !io::stdin().is_terminal() {
        return read_stdin_text(label);
    }
    if non_interactive {
        bail!("{label} is required in non-interactive mode");
    }

    eprint!("{label}: ");
    io::stderr().flush().ok();
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .with_context(|| format!("failed to read {label}"))?;
    let trimmed = line.trim();
    if trimmed.is_empty() {
        bail!("{label} is empty");
    }
    Ok(trimmed.to_string())
}

fn render_parse_error(error: clap::Error) -> ExitCode {
    match error.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            print!("{error}");
            ExitCode::SUCCESS
        }
        _ => {
            eprint!("{error}");
            ExitCode::from(CliErrorKind::InvalidRequest.code())
        }
    }
}

fn render_cli_error(mode: OutputMode, error: &CliError) {
    let envelope = serde_json::json!({
        "error": {
            "code": error.kind,
            "message": error.message,
        },
        "exit_code": error.kind.code(),
    });

    match mode {
        OutputMode::Text => {
            let _ = writeln!(
                io::stderr().lock(),
                "error[{:?}]: {}",
                error.kind,
                error.message
            );
        }
        OutputMode::Json => {
            let mut stderr = io::stderr().lock();
            let _ = serde_json::to_writer_pretty(&mut stderr, &envelope);
            let _ = stderr.write_all(b"\n");
        }
        OutputMode::Jsonl => {
            let mut stderr = io::stderr().lock();
            let _ = serde_json::to_writer(&mut stderr, &envelope);
            let _ = stderr.write_all(b"\n");
        }
    }
}

pub(crate) fn classify_error_message(message: &str) -> CliError {
    let normalized = message.to_ascii_lowercase();

    if normalized.contains("invalid config")
        || normalized.contains("config payload")
        || normalized.contains("config json")
    {
        return CliError::config_validation(message);
    }

    if normalized.contains("tool error")
        || normalized.contains("tool call")
        || normalized.contains("tool '")
    {
        return CliError::tool_failure(message);
    }

    if normalized.contains("dns")
        || normalized.contains("timed out")
        || normalized.contains("connection refused")
        || normalized.contains("connection reset")
        || normalized.contains("network")
        || normalized.contains("reqwest")
        || normalized.contains("git clone failed")
    {
        return CliError::network_failure(message);
    }

    if normalized.contains("provider")
        || normalized.contains("openai")
        || normalized.contains("anthropic")
        || normalized.contains("gemini")
        || normalized.contains("no api key")
        || normalized.contains("model")
    {
        return CliError::provider_failure(message);
    }

    CliError::execution_failure(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_help_includes_primary_command_groups() {
        let rendered = Cli::command().render_help().to_string();
        assert!(rendered.contains("chat"));
        assert!(rendered.contains("sessions"));
        assert!(rendered.contains("config"));
        assert!(rendered.contains("skills"));
        assert!(rendered.contains("upgrade"));
        assert!(!rendered.contains("serve"));
        assert!(!rendered.contains("channels"));
    }

    #[test]
    fn classify_provider_errors() {
        let error = CliError::from_app_error(AppError::internal("openai provider failed"));
        assert_eq!(error.kind, CliErrorKind::ProviderFailure);
    }
}
