use anyhow::Result;

use crate::config::AppConfig;

const DEFAULT_SOUL: &str = include_str!("../../templates/personality/SOUL.md");
const DEFAULT_IDENTITY: &str = include_str!("../../templates/personality/IDENTITY.md");
const DEFAULT_USER: &str = include_str!("../../templates/personality/USER.md");
const DEFAULT_AGENTS: &str = include_str!("../../templates/personality/AGENTS.md");

pub async fn bootstrap_runtime_dirs(config: &AppConfig) -> Result<()> {
    // personality dir: only write defaults if the entire directory does not yet exist
    if !config.personality_dir.exists() {
        tokio::fs::create_dir_all(&config.personality_dir).await?;
        let files = [
            ("SOUL.md", DEFAULT_SOUL),
            ("IDENTITY.md", DEFAULT_IDENTITY),
            ("USER.md", DEFAULT_USER),
            ("AGENTS.md", DEFAULT_AGENTS),
        ];
        for (name, content) in files {
            tokio::fs::write(config.personality_dir.join(name), content).await?;
        }
        tracing::info!("bootstrapped default personality files");
    }

    // data/sessions dir
    tokio::fs::create_dir_all(config.working_dir.join("data/sessions")).await?;

    Ok(())
}
