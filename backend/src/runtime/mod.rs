pub mod bootstrap;
pub mod cli;
pub mod config_runtime;

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::application::agent::{AgentConfig, AgentLoop};
use crate::application::{ChatService, SessionService, UpgradeService};
use crate::domain::ports::{MemoryPort, SkillPort, ToolExecutorPort, UpgradePort};
use crate::infrastructure::config::{
    workspace_base_for, AgentFileConfig, AppConfig, EnvSecrets, LoadedConfig,
};
use crate::infrastructure::memory::MemoryStore;
use crate::infrastructure::model;
use crate::infrastructure::personality::{PersonalityLoader, PersonalitySource};
use crate::infrastructure::session_store::SessionStore;
use crate::infrastructure::skills::SkillStore;
use crate::infrastructure::tooling::ToolRegistry;
use crate::infrastructure::upgrade::GitHubReleaseUpdater;
use crate::runtime::bootstrap::bootstrap_runtime_dirs;
use crate::runtime::config_runtime::{AgentFactory, ConfigRuntime, RestartMode};

struct BackendAgentFactory;

#[async_trait::async_trait]
impl AgentFactory for BackendAgentFactory {
    async fn build_agent(&self, config: &AppConfig) -> Result<Arc<AgentLoop>> {
        let skills: Arc<dyn SkillPort> = Arc::new(SkillStore::new(config.skills_dir.clone()));
        skills.ensure_layout().await?;
        build_agent_loop(config, skills).await
    }
}

#[derive(Clone)]
pub struct AppContext {
    pub config: AppConfig,
    pub agent: Arc<RwLock<Arc<AgentLoop>>>,
    pub sessions: SessionStore,
    pub config_runtime: Arc<ConfigRuntime>,
    pub skills: Arc<dyn SkillPort>,
    pub skills_dir: PathBuf,
    pub upgrades: Arc<dyn UpgradePort>,
}

impl AppContext {
    pub fn chat_service(&self) -> ChatService {
        ChatService::new(self.agent.clone(), self.sessions.clone())
    }

    pub fn session_service(&self) -> SessionService {
        SessionService::new(self.sessions.clone())
    }

    pub fn config_service(&self) -> crate::application::ConfigService {
        crate::application::ConfigService::new(Some(self.config_runtime.clone()))
    }

    pub fn upgrade_service(&self) -> UpgradeService {
        UpgradeService::new(Some(self.upgrades.clone()))
    }
}

pub fn load_runtime_config(
    config_path: Option<&Path>,
    workspace_override: Option<&Path>,
) -> Result<LoadedConfig> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let env_secrets = EnvSecrets::from_env();
    let mut loaded = match config_path {
        Some(path) => {
            let (app, file, raw) =
                AppConfig::from_config_file_path(path, env_secrets, cwd.clone())?;
            LoadedConfig { app, file, raw }
        }
        None => AppConfig::load_with_source()?,
    };

    if let Some(workspace) = workspace_override {
        loaded.file.workspace = Some(workspace.to_path_buf());
        loaded
            .app
            .apply_workspace_override(workspace.to_path_buf(), &workspace_base_for(&cwd));
    }

    Ok(loaded)
}

pub async fn build_app(config: &AppConfig) -> Result<AppContext> {
    build_context(config, AgentFileConfig::default(), RestartMode::Disabled).await
}

pub async fn build_context(
    config: &AppConfig,
    file_config: AgentFileConfig,
    restart_mode: RestartMode,
) -> Result<AppContext> {
    let skills: Arc<dyn SkillPort> = Arc::new(SkillStore::new(config.skills_dir.clone()));
    skills.ensure_layout().await?;

    let upgrades: Arc<dyn UpgradePort> = Arc::new(GitHubReleaseUpdater::new()?);
    let sessions = SessionStore::new(config.sessions_dir.clone());
    sessions.ensure_layout().await?;

    let agent = build_agent_loop(config, skills.clone()).await?;
    let agent_slot = Arc::new(RwLock::new(agent));

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let workspace_base = workspace_base_for(&cwd);
    let env_secrets = EnvSecrets::from_env();
    let config_runtime = Arc::new(ConfigRuntime::new(
        agent_slot.clone(),
        Arc::new(BackendAgentFactory),
        file_config,
        config.clone(),
        workspace_base,
        env_secrets,
        config.config_path.clone(),
        restart_mode,
    ));

    Ok(AppContext {
        config: config.clone(),
        agent: agent_slot,
        sessions,
        config_runtime,
        skills,
        skills_dir: config.skills_dir.clone(),
        upgrades,
    })
}

pub async fn build_agent_loop(
    config: &AppConfig,
    skills: Arc<dyn SkillPort>,
) -> Result<Arc<AgentLoop>> {
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
    registry.register_default_tools_with_config(config);
    let tools: Arc<dyn ToolExecutorPort> = Arc::new(registry);

    Ok(Arc::new(AgentLoop::new(
        provider,
        tools,
        personality,
        memory,
        skills,
        AgentConfig::from(config),
    )))
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
            sessions_dir: temp.path().join("sessions"),
            ..AppConfig::default()
        };

        let context = build_app(&config).await.expect("build_app");
        let sessions = context.sessions.list().await.expect("list sessions");
        assert!(sessions.is_empty());
        assert!(config.personality_dir.exists());
        assert!(config.memory_dir.exists());
        assert!(config.memory_file.exists());
        assert!(config.sessions_dir.exists());
    }

    #[tokio::test]
    async fn build_context_uses_runtime_config() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config = AppConfig {
            provider: "mock".to_string(),
            workspace: temp.path().to_path_buf(),
            config_path: temp.path().join("config.json"),
            log_dir: temp.path().join("logs"),
            working_dir: temp.path().to_path_buf(),
            personality_dir: temp.path().join("personality"),
            memory_dir: temp.path().join("memory"),
            memory_file: temp.path().join("MEMORY.md"),
            sessions_dir: temp.path().join("sessions"),
            skills_dir: temp.path().join("skills"),
            ..AppConfig::default()
        };

        let context = build_context(&config, AgentFileConfig::default(), RestartMode::Disabled)
            .await
            .expect("build_context");

        assert_eq!(context.config_runtime.config_path(), config.config_path);
        assert_eq!(context.config.config_path, config.config_path);
    }
}
