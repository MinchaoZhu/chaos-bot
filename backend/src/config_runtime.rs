use anyhow::{Context, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::agent::AgentLoop;
use crate::config::{read_config_file, write_config_file, AgentFileConfig, AppConfig, EnvSecrets};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartMode {
    Disabled,
    ExitProcess,
}

#[async_trait]
pub trait AgentFactory: Send + Sync {
    async fn build_agent(&self, config: &AppConfig) -> Result<Arc<AgentLoop>>;
}

#[derive(Clone)]
pub struct ConfigRuntime {
    agent_slot: Arc<RwLock<Arc<AgentLoop>>>,
    agent_factory: Arc<dyn AgentFactory>,
    state: Arc<RwLock<RuntimeSnapshot>>,
    workspace_base: PathBuf,
    config_path: PathBuf,
    restart_mode: RestartMode,
}

#[derive(Clone)]
struct RuntimeSnapshot {
    running_file: AgentFileConfig,
    running_app: AppConfig,
}

impl ConfigRuntime {
    pub fn new(
        agent_slot: Arc<RwLock<Arc<AgentLoop>>>,
        agent_factory: Arc<dyn AgentFactory>,
        running_file: AgentFileConfig,
        running_app: AppConfig,
        workspace_base: PathBuf,
        config_path: PathBuf,
        restart_mode: RestartMode,
    ) -> Self {
        Self {
            agent_slot,
            agent_factory,
            state: Arc::new(RwLock::new(RuntimeSnapshot {
                running_file,
                running_app,
            })),
            workspace_base,
            config_path,
            restart_mode,
        }
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn backup_path(&self, level: u8) -> PathBuf {
        backup_path(&self.config_path, level)
    }

    pub async fn running_config(&self) -> AgentFileConfig {
        self.state.read().await.running_file.clone()
    }

    pub async fn running_app_config(&self) -> AppConfig {
        self.state.read().await.running_app.clone()
    }

    pub async fn disk_config(&self) -> Result<(AgentFileConfig, String)> {
        read_config_file(&self.config_path)
    }

    pub async fn reset(&self) -> Result<AgentFileConfig> {
        let running = self.running_config().await;
        write_config_with_backups(&self.config_path, &running)?;
        tracing::info!(
            config_path = %self.config_path.display(),
            "config reset to running snapshot"
        );
        Ok(running)
    }

    pub async fn apply_raw(&self, raw: &str) -> Result<AgentFileConfig> {
        let parsed = serde_json::from_str::<AgentFileConfig>(raw)
            .context("invalid config json in raw payload")?;
        self.apply_structured(parsed).await
    }

    pub async fn apply_structured(&self, next: AgentFileConfig) -> Result<AgentFileConfig> {
        let mut next_app = AppConfig::from_inputs(
            next.clone(),
            EnvSecrets::default(),
            self.workspace_base.clone(),
        );
        next_app.config_path = self.config_path.clone();

        let next_agent = self.agent_factory.build_agent(&next_app).await?;
        write_config_with_backups(&self.config_path, &next)?;

        {
            let mut slot = self.agent_slot.write().await;
            *slot = next_agent;
        }

        {
            let mut state = self.state.write().await;
            state.running_file = next.clone();
            state.running_app = next_app;
        }

        tracing::info!(
            config_path = %self.config_path.display(),
            "config applied to runtime"
        );
        Ok(next)
    }

    pub async fn restart_after_apply_raw(&self, raw: &str) -> Result<bool> {
        self.apply_raw(raw).await?;
        self.request_restart().await
    }

    pub async fn restart_after_apply_structured(&self, next: AgentFileConfig) -> Result<bool> {
        self.apply_structured(next).await?;
        self.request_restart().await
    }

    pub async fn request_restart(&self) -> Result<bool> {
        if self.restart_mode == RestartMode::Disabled {
            tracing::info!("restart requested but disabled by runtime mode");
            return Ok(false);
        }

        tracing::warn!("process restart requested; exiting current process");
        tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(250)).await;
            std::process::exit(0);
        });
        Ok(true)
    }
}

pub fn write_config_with_backups(path: &Path, config: &AgentFileConfig) -> Result<String> {
    rotate_backups(path)?;
    write_config_file(path, config)
}

fn rotate_backups(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let bak1 = backup_path(path, 1);
    let bak2 = backup_path(path, 2);

    if bak2.exists() {
        std::fs::remove_file(&bak2)
            .with_context(|| format!("failed to remove old backup file: {}", bak2.display()))?;
    }

    if bak1.exists() {
        std::fs::rename(&bak1, &bak2).with_context(|| {
            format!(
                "failed to rotate config backup from {} to {}",
                bak1.display(),
                bak2.display()
            )
        })?;
    }

    std::fs::copy(path, &bak1).with_context(|| {
        format!(
            "failed to create latest config backup from {} to {}",
            path.display(),
            bak1.display()
        )
    })?;

    Ok(())
}

fn backup_path(path: &Path, level: u8) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "config.json".to_string());
    path.with_file_name(format!("{file_name}.bak{level}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentFileConfig;
    use tempfile::tempdir;

    #[test]
    fn rotate_backups_keeps_two_generations() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("config.json");

        std::fs::write(&config_path, "{\"v\":1}\n").expect("write v1");
        write_config_with_backups(&config_path, &AgentFileConfig::default()).expect("write v2");
        std::fs::write(&config_path, "{\"v\":3}\n").expect("write v3");
        write_config_with_backups(&config_path, &AgentFileConfig::default()).expect("write v4");

        let bak1 = backup_path(&config_path, 1);
        let bak2 = backup_path(&config_path, 2);

        assert!(bak1.exists());
        assert!(bak2.exists());
    }
}
