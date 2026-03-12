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
    use crate::infrastructure::config::{
        AgentLlmConfig, AgentLoggingConfig, AgentSearchConfig, AgentSecretsConfig,
    };
    use tempfile::tempdir;

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

    #[test]
    fn config_mutation_accepts_raw_payload() {
        let input = config_mutation_input(ConfigPayloadArgs {
            raw: Some("{\"llm\":{\"provider\":\"mock\"}}".to_string()),
            file: None,
            stdin: false,
        })
        .expect("raw config mutation input");

        match input {
            ConfigMutationInput::Raw(raw) => assert!(raw.contains("\"provider\":\"mock\"")),
            other => panic!("expected raw payload, got {other:?}"),
        }
    }

    #[test]
    fn config_mutation_reads_structured_payload_from_file() {
        let temp = tempdir().expect("tempdir");
        let payload_path = temp.path().join("config.json");
        std::fs::write(
            &payload_path,
            r#"{
  "llm": {
    "provider": "mock",
    "model": "mock-model"
  },
  "logging": {
    "level": "error"
  }
}
"#,
        )
        .expect("write payload");

        let input = config_mutation_input(ConfigPayloadArgs {
            raw: None,
            file: Some(payload_path),
            stdin: false,
        })
        .expect("file config mutation input");

        match input {
            ConfigMutationInput::Structured(config) => {
                assert_eq!(config.llm.provider.as_deref(), Some("mock"));
                assert_eq!(config.llm.model.as_deref(), Some("mock-model"));
                assert_eq!(config.logging.level.as_deref(), Some("error"));
            }
            other => panic!("expected structured payload, got {other:?}"),
        }
    }

    #[test]
    fn config_mutation_rejects_missing_input() {
        let error = config_mutation_input(ConfigPayloadArgs {
            raw: None,
            file: None,
            stdin: false,
        })
        .expect_err("missing mutation input should fail");

        assert!(error
            .message
            .contains("exactly one of --raw, --file, or --stdin"));
    }

    #[test]
    fn config_restart_accepts_raw_and_structured_inputs() {
        let raw = config_restart_input(ConfigPayloadArgs {
            raw: Some("{\"logging\":{\"level\":\"warn\"}}".to_string()),
            file: None,
            stdin: false,
        })
        .expect("raw restart input");
        assert!(matches!(raw, ConfigRestartInput::Raw(_)));

        let temp = tempdir().expect("tempdir");
        let payload_path = temp.path().join("config.json");
        let structured = crate::infrastructure::config::AgentFileConfig {
            workspace: None,
            logging: AgentLoggingConfig {
                level: Some("debug".to_string()),
                retention_days: Some(3),
                directory: None,
            },
            llm: AgentLlmConfig {
                provider: Some("mock".to_string()),
                model: Some("mock-model".to_string()),
                temperature: Some(0.1),
                max_tokens: Some(256),
                max_iterations: Some(3),
                token_budget: Some(1024),
            },
            search: AgentSearchConfig::default(),
            secrets: AgentSecretsConfig::default(),
        };
        std::fs::write(
            &payload_path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&structured).expect("json")
            ),
        )
        .expect("write payload");

        let file = config_restart_input(ConfigPayloadArgs {
            raw: None,
            file: Some(payload_path),
            stdin: false,
        })
        .expect("file restart input");

        match file {
            ConfigRestartInput::Structured(config) => {
                assert_eq!(config.logging.level.as_deref(), Some("debug"));
                assert_eq!(config.llm.max_iterations, Some(3));
            }
            other => panic!("expected structured restart input, got {other:?}"),
        }
    }

    #[test]
    fn config_restart_rejects_multiple_sources() {
        let error = config_restart_input(ConfigPayloadArgs {
            raw: Some("{}".to_string()),
            file: Some(std::path::PathBuf::from("config.json")),
            stdin: false,
        })
        .expect_err("multiple restart sources should fail");

        assert!(error
            .message
            .contains("at most one of --raw, --file, or --stdin"));
    }
}
