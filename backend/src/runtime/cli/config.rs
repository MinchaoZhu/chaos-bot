use clap::{Args, Subcommand};

use crate::domain::config::{ConfigMutationInput, ConfigRestartInput};
use crate::infrastructure::config::read_config_file;

use super::{bootstrap_runtime, read_stdin_text, CliError, GlobalOptions};

#[derive(Debug, Args)]
pub(crate) struct ConfigArgs {
    #[command(subcommand)]
    command: Option<ConfigCommand>,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    #[command(alias = "show")]
    Get,
    Reset,
    Apply(ConfigPayloadArgs),
    Restart(ConfigPayloadArgs),
}

#[derive(Debug, Args, Clone)]
struct ConfigPayloadArgs {
    #[arg(long, conflicts_with_all = ["file", "stdin"])]
    raw: Option<String>,

    #[arg(long, value_name = "PATH", conflicts_with_all = ["raw", "stdin"])]
    file: Option<std::path::PathBuf>,

    #[arg(long, conflicts_with_all = ["raw", "file"])]
    stdin: bool,
}

pub(crate) async fn run(
    global: &GlobalOptions,
    args: ConfigArgs,
) -> std::result::Result<(), CliError> {
    let handles = bootstrap_runtime(global)
        .await
        .map_err(CliError::from_anyhow)?;
    let service = handles.context.config_service();

    match args.command.unwrap_or(ConfigCommand::Get) {
        ConfigCommand::Get => {
            let response = service.get().await.map_err(CliError::from_app_error)?;
            super::write_output(
                global.output,
                &response,
                super::output::render_config_state_text,
            )
        }
        ConfigCommand::Reset => {
            let response = service.reset().await.map_err(CliError::from_app_error)?;
            super::write_output(
                global.output,
                &response,
                super::output::render_config_mutation_text,
            )
        }
        ConfigCommand::Apply(args) => {
            let response = service
                .apply(config_mutation_input(args)?)
                .await
                .map_err(CliError::from_app_error)?;
            super::write_output(
                global.output,
                &response,
                super::output::render_config_mutation_text,
            )
        }
        ConfigCommand::Restart(args) => {
            let response = service
                .restart(config_restart_input(args)?)
                .await
                .map_err(CliError::from_app_error)?;
            super::write_output(
                global.output,
                &response,
                super::output::render_config_mutation_text,
            )
        }
    }
}

fn config_mutation_input(
    args: ConfigPayloadArgs,
) -> std::result::Result<ConfigMutationInput, CliError> {
    match (args.raw, args.file, args.stdin) {
        (Some(raw), None, false) => Ok(ConfigMutationInput::Raw(raw)),
        (None, Some(path), false) => {
            let (config, _) = read_config_file(&path).map_err(|error| {
                CliError::config_validation(format!(
                    "failed to read config payload from {}: {error}",
                    path.display()
                ))
            })?;
            Ok(ConfigMutationInput::Structured(config))
        }
        (None, None, true) => {
            let raw = read_stdin_text("config payload").map_err(CliError::from_anyhow)?;
            Ok(ConfigMutationInput::Raw(raw))
        }
        _ => Err(CliError::invalid_request(
            "exactly one of --raw, --file, or --stdin must be set",
        )),
    }
}

fn config_restart_input(
    args: ConfigPayloadArgs,
) -> std::result::Result<ConfigRestartInput, CliError> {
    match (args.raw, args.file, args.stdin) {
        (Some(raw), None, false) => Ok(ConfigRestartInput::Raw(raw)),
        (None, Some(path), false) => {
            let (config, _) = read_config_file(&path).map_err(|error| {
                CliError::config_validation(format!(
                    "failed to read config payload from {}: {error}",
                    path.display()
                ))
            })?;
            Ok(ConfigRestartInput::Structured(config))
        }
        (None, None, true) => {
            let raw = read_stdin_text("config payload").map_err(CliError::from_anyhow)?;
            Ok(ConfigRestartInput::Raw(raw))
        }
        (None, None, false) => Ok(ConfigRestartInput::Noop),
        _ => Err(CliError::invalid_request(
            "at most one of --raw, --file, or --stdin may be set",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_restart_defaults_to_noop() {
        let input = config_restart_input(ConfigPayloadArgs {
            raw: None,
            file: None,
            stdin: false,
        })
        .expect("config restart input");

        assert!(matches!(input, ConfigRestartInput::Noop));
    }
}
