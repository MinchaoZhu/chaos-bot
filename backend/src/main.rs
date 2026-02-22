use anyhow::Result;
use chaos_bot_backend::agent::AgentLoop;
use chaos_bot_backend::api::{router, AppState};
use chaos_bot_backend::bootstrap::bootstrap_runtime_dirs;
use chaos_bot_backend::config::AppConfig;
use chaos_bot_backend::llm;
use chaos_bot_backend::memory::MemoryStore;
use chaos_bot_backend::personality::PersonalityLoader;
use chaos_bot_backend::tools::ToolRegistry;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = AppConfig::from_env()?;

    bootstrap_runtime_dirs(&config).await?;
    tokio::fs::create_dir_all(&config.memory_dir).await?;

    let memory = MemoryStore::new(config.memory_dir.clone(), config.memory_file.clone());
    memory.ensure_layout().await?;

    let personality = PersonalityLoader::new(config.personality_dir.clone());
    let provider = llm::build_provider(&config)?;

    let mut registry = ToolRegistry::new();
    registry.register_default_tools();

    let agent = Arc::new(AgentLoop::new(
        provider,
        Arc::new(registry),
        personality,
        memory,
        config.model.clone(),
        config.temperature,
        config.max_tokens,
        config.max_iterations,
        config.token_budget,
        config.working_dir.clone(),
    ));

    let state = AppState::new(agent);
    let app = router(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;
    info!(address = %addr, "chaos-bot server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(filter).init();
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
