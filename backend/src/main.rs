use anyhow::Result;
use chaos_bot_backend::agent::{AgentConfig, AgentLoop};
use chaos_bot_backend::api::{router, AppState};
use chaos_bot_backend::bootstrap::bootstrap_runtime_dirs;
use chaos_bot_backend::config::AppConfig;
use chaos_bot_backend::llm;
use chaos_bot_backend::logging::init_logging;
use chaos_bot_backend::memory::{MemoryBackend, MemoryStore};
use chaos_bot_backend::personality::{PersonalityLoader, PersonalitySource};
use chaos_bot_backend::tools::ToolRegistry;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    let logging_runtime = init_logging(&config)?;
    info!(
        workspace = %config.workspace.display(),
        log_dir = %config.log_dir.display(),
        log_file = %logging_runtime.log_file.display(),
        log_level = %config.log_level,
        retention_days = config.log_retention_days,
        "chaos-bot logging initialized"
    );

    let state = build_app(&config).await?;
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

pub async fn build_app(config: &AppConfig) -> Result<AppState> {
    bootstrap_runtime_dirs(config).await?;
    tokio::fs::create_dir_all(&config.memory_dir).await?;

    let memory: Arc<dyn MemoryBackend> = Arc::new(MemoryStore::new(
        config.memory_dir.clone(),
        config.memory_file.clone(),
    ));
    memory.ensure_layout().await?;

    let personality: Arc<dyn PersonalitySource> =
        Arc::new(PersonalityLoader::new(config.personality_dir.clone()));
    let provider = llm::build_provider(config)?;

    let mut registry = ToolRegistry::new();
    registry.register_default_tools();

    let agent = Arc::new(AgentLoop::new(
        provider,
        Arc::new(registry),
        personality,
        memory,
        AgentConfig::from(config),
    ));

    Ok(AppState::new(agent))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
