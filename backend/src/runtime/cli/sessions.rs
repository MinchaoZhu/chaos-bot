use clap::{Args, Subcommand};

use super::{
    bootstrap_runtime, write_output, CliError, DeleteResponse, GlobalOptions, SessionSummary,
};

#[derive(Debug, Args)]
pub(crate) struct SessionsArgs {
    #[command(subcommand)]
    command: Option<SessionsCommand>,
}

#[derive(Debug, Subcommand)]
enum SessionsCommand {
    List,
    Get(IdArgs),
    Delete(IdArgs),
}

#[derive(Debug, Args)]
struct IdArgs {
    id: String,
}

pub(crate) async fn run(
    global: &GlobalOptions,
    args: SessionsArgs,
) -> std::result::Result<(), CliError> {
    let handles = bootstrap_runtime(global)
        .await
        .map_err(CliError::from_anyhow)?;
    let service = handles.context.session_service();

    match args.command.unwrap_or(SessionsCommand::List) {
        SessionsCommand::List => {
            let sessions = service.list().await.map_err(CliError::from_app_error)?;
            let summaries = sessions
                .iter()
                .map(SessionSummary::from)
                .collect::<Vec<_>>();
            write_output(global.output, &summaries, |value| {
                super::output::render_sessions_text(value)
            })
        }
        SessionsCommand::Get(args) => {
            let session = service
                .get(&args.id)
                .await
                .map_err(CliError::from_app_error)?;
            write_output(
                global.output,
                &session,
                super::output::render_session_detail_text,
            )
        }
        SessionsCommand::Delete(args) => {
            service
                .delete(&args.id)
                .await
                .map_err(CliError::from_app_error)?;
            let response = DeleteResponse {
                ok: true,
                id: &args.id,
            };
            write_output(global.output, &response, super::output::render_delete_text)
        }
    }
}
