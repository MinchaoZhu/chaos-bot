use anyhow::Result;

use crate::infrastructure::config::AppConfig;
use crate::infrastructure::runtime_assets::{
    DEFAULT_AGENTS_MD, DEFAULT_IDENTITY_MD, DEFAULT_SOUL_MD, DEFAULT_USER_MD,
};

pub async fn bootstrap_runtime_dirs(config: &AppConfig) -> Result<()> {
    tokio::fs::create_dir_all(&config.personality_dir).await?;
    tracing::debug!(
        personality_dir = %config.personality_dir.display(),
        "ensured personality directory"
    );
    let files = [
        ("SOUL.md", DEFAULT_SOUL_MD),
        ("IDENTITY.md", DEFAULT_IDENTITY_MD),
        ("USER.md", DEFAULT_USER_MD),
        ("AGENTS.md", DEFAULT_AGENTS_MD),
    ];
    let mut created = 0usize;
    for (name, content) in files {
        let path = config.personality_dir.join(name);
        if !path.exists() {
            tokio::fs::write(path, content).await?;
            created += 1;
        }
    }
    if created > 0 {
        tracing::info!(
            created_files = created,
            personality_dir = %config.personality_dir.display(),
            "bootstrapped default personality files"
        );
    }

    // data/sessions dir
    let sessions_dir = config.working_dir.join("data/sessions");
    tokio::fs::create_dir_all(&sessions_dir).await?;
    tracing::debug!(sessions_dir = %sessions_dir.display(), "ensured sessions directory");

    Ok(())
}
