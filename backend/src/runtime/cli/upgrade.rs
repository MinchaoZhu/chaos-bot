use clap::{Args, Subcommand};

use super::{bootstrap_runtime, write_output, CliError, GlobalOptions};

#[derive(Debug, Args)]
pub(crate) struct UpgradeArgs {
    #[command(subcommand)]
    command: Option<UpgradeCommand>,
}

#[derive(Debug, Subcommand)]
enum UpgradeCommand {
    Status,
    Apply,
    Relaunch,
}

pub(crate) async fn run(
    global: &GlobalOptions,
    args: UpgradeArgs,
) -> std::result::Result<(), CliError> {
    let handles = bootstrap_runtime(global)
        .await
        .map_err(CliError::from_anyhow)?;
    let service = handles.context.upgrade_service();

    match args.command.unwrap_or(UpgradeCommand::Status) {
        UpgradeCommand::Status => {
            let status = service.status().await.map_err(CliError::from_app_error)?;
            write_output(
                global.output,
                &status,
                super::output::render_upgrade_status_text,
            )
        }
        UpgradeCommand::Apply => {
            let result = service.apply().await.map_err(CliError::from_app_error)?;
            write_output(
                global.output,
                &result,
                super::output::render_upgrade_apply_text,
            )
        }
        UpgradeCommand::Relaunch => {
            let result = service.relaunch().await.map_err(CliError::from_app_error)?;
            write_output(
                global.output,
                &result,
                super::output::render_upgrade_restart_text,
            )
        }
    }
}
