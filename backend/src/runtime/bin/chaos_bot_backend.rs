use anyhow::Result;
use chaos_bot_backend::infrastructure::config::AppConfig;
use chaos_bot_backend::infrastructure::logging::init_logging;
use chaos_bot_backend::interface::api::router;
use chaos_bot_backend::runtime::{build_app_with_config_runtime, shutdown_signal};
use chaos_bot_backend::runtime::config_runtime::RestartMode;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let loaded = AppConfig::load_with_source()?;
    let config = loaded.app.clone();

    let logging_runtime = init_logging(&config)?;
    info!(
        workspace = %config.workspace.display(),
        config_file = %config.config_path.display(),
        log_dir = %config.log_dir.display(),
        log_file = %logging_runtime.log_file.display(),
        log_level = %config.log_level,
        retention_days = config.log_retention_days,
        "chaos-bot logging initialized"
    );

    let restart_mode = if std::env::var("CHAOS_BOT_DISABLE_SELF_RESTART")
        .ok()
        .as_deref()
        == Some("1")
    {
        RestartMode::Disabled
    } else {
        RestartMode::ExitProcess
    };

    let state = build_app_with_config_runtime(&config, loaded.file, restart_mode).await?;
    let app = router(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;
    info!(address = %addr, "chaos-bot server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    drop(logging_runtime);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chaos_bot_backend::runtime::build_app;

    #[tokio::test]
    async fn build_app_works_with_mock_provider() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config = AppConfig {
            provider: "mock".to_string(),
            workspace: temp.path().to_path_buf(),
            log_dir: temp.path().join("logs"),
            working_dir: temp.path().to_path_buf(),
            personality_dir: temp.path().join("personality"),
            memory_dir: temp.path().join("memory"),
            memory_file: temp.path().join("MEMORY.md"),
            ..AppConfig::default()
        };

        let state = build_app(&config).await.expect("build_app");
        let sessions = state.sessions.list().await;
        assert!(sessions.is_empty());
        assert!(config.personality_dir.exists());
        assert!(config.memory_dir.exists());
        assert!(config.memory_file.exists());
    }

    #[tokio::test]
    async fn restart_mode_derives_from_env_switch() {
        let previous = std::env::var("CHAOS_BOT_DISABLE_SELF_RESTART").ok();
        std::env::set_var("CHAOS_BOT_DISABLE_SELF_RESTART", "1");

        let mode = if std::env::var("CHAOS_BOT_DISABLE_SELF_RESTART")
            .ok()
            .as_deref()
            == Some("1")
        {
            RestartMode::Disabled
        } else {
            RestartMode::ExitProcess
        };

        assert!(matches!(mode, RestartMode::Disabled));

        match previous {
            Some(value) => std::env::set_var("CHAOS_BOT_DISABLE_SELF_RESTART", value),
            None => std::env::remove_var("CHAOS_BOT_DISABLE_SELF_RESTART"),
        }
    }
}
