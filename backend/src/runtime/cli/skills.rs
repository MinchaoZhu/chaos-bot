use clap::{Args, Subcommand};

use crate::infrastructure::skills::install_skills_from_git_source;

use super::{bootstrap_runtime, write_output, CliError, GlobalOptions, InstallSkillResponse};

#[derive(Debug, Args)]
pub(crate) struct SkillsArgs {
    #[command(subcommand)]
    command: Option<SkillsCommand>,
}

#[derive(Debug, Subcommand)]
enum SkillsCommand {
    List,
    Get(IdArgs),
    Install(InstallSkillArgs),
}

#[derive(Debug, Args)]
struct IdArgs {
    id: String,
}

#[derive(Debug, Args)]
struct InstallSkillArgs {
    source: String,
}

pub(crate) async fn run(
    global: &GlobalOptions,
    args: SkillsArgs,
) -> std::result::Result<(), CliError> {
    let handles = bootstrap_runtime(global)
        .await
        .map_err(CliError::from_anyhow)?;

    match args.command.unwrap_or(SkillsCommand::List) {
        SkillsCommand::List => {
            let skills = handles
                .context
                .skills
                .list()
                .await
                .map_err(CliError::from_anyhow)?;
            write_output(global.output, &skills, |value| {
                super::output::render_skills_text(value)
            })
        }
        SkillsCommand::Get(args) => {
            let skill = handles
                .context
                .skills
                .get(&args.id)
                .await
                .map_err(CliError::from_anyhow)?;
            write_output(
                global.output,
                &skill,
                super::output::render_skill_detail_text,
            )
        }
        SkillsCommand::Install(args) => {
            let installed =
                install_skills_from_git_source(&handles.context.skills_dir, &args.source)
                    .await
                    .map_err(CliError::from_anyhow)?;
            let response = InstallSkillResponse {
                ok: true,
                source: &args.source,
                installed: &installed,
            };
            write_output(
                global.output,
                &response,
                super::output::render_skill_install_text,
            )
        }
    }
}
