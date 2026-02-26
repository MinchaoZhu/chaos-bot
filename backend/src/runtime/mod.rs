pub mod bootstrap;
pub mod config_runtime;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

use crate::application::ChatService;
use crate::application::agent::{AgentConfig, AgentLoop};
use crate::domain::ports::{MemoryPort, ToolExecutorPort};
use crate::interface::api::AppState;
use crate::infrastructure::channels::build_dispatcher;
use crate::infrastructure::channels::telegram::poll_updates_once;
use crate::runtime::bootstrap::bootstrap_runtime_dirs;
use crate::infrastructure::config::{workspace_base_for, AgentFileConfig, AppConfig};
use crate::runtime::config_runtime::{AgentFactory, ConfigRuntime, RestartMode};
use crate::infrastructure::model;
use crate::infrastructure::memory::MemoryStore;
use crate::infrastructure::personality::{PersonalityLoader, PersonalitySource};
use crate::infrastructure::tooling::ToolRegistry;

struct BackendAgentFactory;

#[async_trait::async_trait]
impl AgentFactory for BackendAgentFactory {
    async fn build_agent(&self, config: &AppConfig) -> Result<Arc<AgentLoop>> {
        build_agent_loop(config).await
    }
}

pub async fn build_app(config: &AppConfig) -> Result<AppState> {
    let agent = build_agent_loop(config).await?;
    let channel_dispatcher = build_dispatcher(config).await?;
    let state = AppState::new(
        agent,
        channel_dispatcher,
        config.telegram_webhook_secret.clone(),
        config.telegram_enabled,
        config.telegram_polling,
        config.telegram_api_base_url.clone(),
    );
    maybe_spawn_telegram_poller(state.clone(), config);
    Ok(state)
}

pub async fn build_app_with_config_runtime(
    config: &AppConfig,
    file_config: AgentFileConfig,
    restart_mode: RestartMode,
) -> Result<AppState> {
    let agent = build_agent_loop(config).await?;
    let channel_dispatcher = build_dispatcher(config).await?;
    let agent_slot = Arc::new(RwLock::new(agent));

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let workspace_base = workspace_base_for(&cwd);

    let runtime = Arc::new(ConfigRuntime::new(
        agent_slot.clone(),
        Arc::new(BackendAgentFactory),
        file_config,
        config.clone(),
        workspace_base,
        config.config_path.clone(),
        restart_mode,
    ));

    let state = AppState::with_config_runtime(
        agent_slot,
        runtime,
        channel_dispatcher,
        config.telegram_webhook_secret.clone(),
        config.telegram_enabled,
        config.telegram_polling,
        config.telegram_api_base_url.clone(),
    );
    maybe_spawn_telegram_poller(state.clone(), config);
    Ok(state)
}

pub async fn build_agent_loop(config: &AppConfig) -> Result<Arc<AgentLoop>> {
    bootstrap_runtime_dirs(config).await?;
    tokio::fs::create_dir_all(&config.memory_dir).await?;

    let memory: Arc<dyn MemoryPort> = Arc::new(MemoryStore::new(
        config.memory_dir.clone(),
        config.memory_file.clone(),
    ));
    memory.ensure_layout().await?;

    let personality: Arc<dyn PersonalitySource> =
        Arc::new(PersonalityLoader::new(config.personality_dir.clone()));
    let provider = model::build_provider(config)?;

    let mut registry = ToolRegistry::new();
    registry.register_default_tools();
    let tools: Arc<dyn ToolExecutorPort> = Arc::new(registry);

    Ok(Arc::new(AgentLoop::new(
        provider,
        tools,
        personality,
        memory,
        AgentConfig::from(config),
    )))
}

pub async fn shutdown_signal() {
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

fn maybe_spawn_telegram_poller(state: AppState, config: &AppConfig) {
    if !config.telegram_enabled || !config.telegram_polling {
        return;
    }
    let Some(bot_token) = config.telegram_bot_token.clone() else {
        tracing::warn!(
            "telegram polling enabled but no bot token configured; poller is not started"
        );
        return;
    };

    let api_base_url = config.telegram_api_base_url.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let mut offset: i64 = 0;
        tracing::info!(channel = "telegram", mode = "polling", "telegram poller started");

        loop {
            let updates = match poll_updates_once(&client, &api_base_url, &bot_token, offset, 15).await {
                Ok(items) => items,
                Err(error) => {
                    tracing::warn!(channel = "telegram", mode = "polling", error = %error, "telegram poller request failed");
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            if updates.is_empty() {
                continue;
            }

            let service = ChatService::new(
                state.agent.clone(),
                state.sessions.clone(),
                state.channel_dispatcher.clone(),
            );

            for update in updates {
                if update.update_id + 1 > offset {
                    offset = update.update_id + 1;
                }
                let Some(inbound) = update.into_inbound_message() else {
                    continue;
                };
                if let Err(error) = service.run_channel_message(inbound).await {
                    tracing::warn!(
                        channel = "telegram",
                        mode = "polling",
                        error = %error.message(),
                        "telegram poller failed to process inbound update"
                    );
                }
            }
        }
    });
}
