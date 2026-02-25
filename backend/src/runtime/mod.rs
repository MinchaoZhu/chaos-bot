pub mod bootstrap;
pub mod config_runtime;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::application::agent::{AgentConfig, AgentLoop};
use crate::domain::ports::{MemoryPort, SkillPort, ToolExecutorPort};
use crate::interface::api::AppState;
use crate::runtime::bootstrap::bootstrap_runtime_dirs;
use crate::infrastructure::config::{workspace_base_for, AgentFileConfig, AppConfig};
use crate::runtime::config_runtime::{AgentFactory, ConfigRuntime, RestartMode};
use crate::infrastructure::model;
use crate::infrastructure::memory::MemoryStore;
use crate::infrastructure::personality::{PersonalityLoader, PersonalitySource};
use crate::infrastructure::skills::SkillStore;
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
    let skills: Arc<dyn SkillPort> = Arc::new(SkillStore::new(config.skills_dir.clone()));
    Ok(AppState::new(agent).with_skills(skills))
}

pub async fn build_app_with_config_runtime(
    config: &AppConfig,
    file_config: AgentFileConfig,
    restart_mode: RestartMode,
) -> Result<AppState> {
    let agent = build_agent_loop(config).await?;
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

    let skills: Arc<dyn SkillPort> = Arc::new(SkillStore::new(config.skills_dir.clone()));
    Ok(AppState::with_config_runtime(agent_slot, runtime).with_skills(skills))
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

    let skills: Arc<dyn SkillPort> = Arc::new(SkillStore::new(config.skills_dir.clone()));
    skills.ensure_layout().await?;

    Ok(Arc::new(AgentLoop::new(
        provider,
        tools,
        personality,
        memory,
        skills,
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
