use crate::domain::config::{
    ConfigMutationInput, ConfigMutationResponse, ConfigRestartInput, ConfigStateResponse,
};
use crate::domain::{audit, AppError};
use crate::runtime::config_runtime::ConfigRuntime;
use std::sync::Arc;

#[derive(Clone)]
pub struct ConfigService {
    runtime: Option<Arc<ConfigRuntime>>,
}

impl ConfigService {
    pub fn new(runtime: Option<Arc<ConfigRuntime>>) -> Self {
        Self { runtime }
    }

    pub async fn get(&self) -> Result<ConfigStateResponse, AppError> {
        let runtime = self.require_runtime()?;
        Ok(build_config_state_response(&runtime).await)
    }

    pub async fn reset(&self) -> Result<ConfigMutationResponse, AppError> {
        let runtime = self.require_runtime()?;
        runtime
            .reset()
            .await
            .map_err(|error| map_internal(error, "reset"))?;
        let state = build_config_state_response(&runtime).await;
        Ok(ConfigMutationResponse {
            ok: true,
            action: "reset",
            restart_scheduled: false,
            state,
        })
    }

    pub async fn apply(
        &self,
        input: ConfigMutationInput,
    ) -> Result<ConfigMutationResponse, AppError> {
        let runtime = self.require_runtime()?;
        match input {
            ConfigMutationInput::Raw(raw) => {
                tracing::info!(
                    action = "apply",
                    payload = %audit::redact_raw_json(&raw),
                    "config mutation audit"
                );
                runtime
                    .apply_raw(&raw)
                    .await
                    .map_err(|error| map_internal(error, "apply_raw"))?;
            }
            ConfigMutationInput::Structured(config) => {
                let value = serde_json::to_value(&config).unwrap_or_default();
                tracing::info!(
                    action = "apply",
                    payload = %audit::redact_json(&value),
                    "config mutation audit"
                );
                runtime
                    .apply_structured(config)
                    .await
                    .map_err(|error| map_internal(error, "apply_structured"))?;
            }
        }

        let state = build_config_state_response(&runtime).await;
        Ok(ConfigMutationResponse {
            ok: true,
            action: "apply",
            restart_scheduled: false,
            state,
        })
    }

    pub async fn restart(
        &self,
        input: ConfigRestartInput,
    ) -> Result<ConfigMutationResponse, AppError> {
        let runtime = self.require_runtime()?;
        let restart_scheduled = match input {
            ConfigRestartInput::Noop => runtime
                .request_restart()
                .await
                .map_err(|error| map_internal(error, "request_restart"))?,
            ConfigRestartInput::Raw(raw) => {
                tracing::info!(
                    action = "restart",
                    payload = %audit::redact_raw_json(&raw),
                    "config mutation audit"
                );
                runtime
                    .restart_after_apply_raw(&raw)
                    .await
                    .map_err(|error| map_internal(error, "restart_after_apply_raw"))?
            }
            ConfigRestartInput::Structured(config) => {
                let value = serde_json::to_value(&config).unwrap_or_default();
                tracing::info!(
                    action = "restart",
                    payload = %audit::redact_json(&value),
                    "config mutation audit"
                );
                runtime
                    .restart_after_apply_structured(config)
                    .await
                    .map_err(|error| map_internal(error, "restart_after_apply_structured"))?
            }
        };

        let state = build_config_state_response(&runtime).await;
        Ok(ConfigMutationResponse {
            ok: true,
            action: "restart",
            restart_scheduled,
            state,
        })
    }

    fn require_runtime(&self) -> Result<Arc<ConfigRuntime>, AppError> {
        self.runtime
            .clone()
            .ok_or_else(|| AppError::service_unavailable("config runtime unavailable"))
    }
}

fn map_internal(error: anyhow::Error, action: &str) -> AppError {
    tracing::warn!(action, error = %error, "config endpoint failed");
    AppError::internal(format!("config {action} failed"))
}

async fn build_config_state_response(runtime: &ConfigRuntime) -> ConfigStateResponse {
    let running = runtime.running_config().await;
    let (disk, raw, disk_parse_error) = match runtime.disk_config().await {
        Ok((disk, raw)) => (disk, raw, None),
        Err(error) => {
            let fallback_raw = serde_json::to_string_pretty(&running)
                .map(|text| format!("{text}\n"))
                .unwrap_or_else(|_| "{}\n".to_string());
            (running.clone(), fallback_raw, Some(error.to_string()))
        }
    };

    let config_path = runtime.config_path().to_path_buf();
    let config_format = config_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "config.json".to_string());

    ConfigStateResponse {
        config_path: config_path.display().to_string(),
        backup1_path: runtime.backup_path(1).display().to_string(),
        backup2_path: runtime.backup_path(2).display().to_string(),
        config_format,
        running,
        disk,
        raw,
        disk_parse_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{AgentConfig, AgentLoop};
    use crate::domain::config::{ConfigMutationInput, ConfigRestartInput};
    use crate::domain::ports::{MemoryPort, ModelPort, SkillPort};
    use crate::infrastructure::config::{
        write_config_file, AgentFileConfig, AgentLlmConfig, AgentLoggingConfig, AppConfig,
        EnvSecrets,
    };
    use crate::infrastructure::memory::MemoryStore;
    use crate::infrastructure::model::MockProvider;
    use crate::infrastructure::personality::PersonalitySource;
    use crate::infrastructure::skills::EmptySkillStore;
    use crate::infrastructure::tooling::ToolRegistry;
    use crate::runtime::config_runtime::{AgentFactory, RestartMode};
    use anyhow::Result;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::sync::RwLock;

    struct TestPersonality;

    #[async_trait]
    impl PersonalitySource for TestPersonality {
        async fn system_prompt(&self) -> Result<String> {
            Ok("You are helpful.".to_string())
        }
    }

    struct StaticAgentFactory {
        agent: Arc<AgentLoop>,
    }

    #[async_trait]
    impl AgentFactory for StaticAgentFactory {
        async fn build_agent(&self, _config: &AppConfig) -> Result<Arc<AgentLoop>> {
            Ok(self.agent.clone())
        }
    }

    fn test_agent(temp: &tempfile::TempDir) -> Arc<AgentLoop> {
        let memory: Arc<dyn MemoryPort> = Arc::new(MemoryStore::new(
            temp.path().join("memory"),
            temp.path().join("MEMORY.md"),
        ));
        let personality: Arc<dyn PersonalitySource> = Arc::new(TestPersonality);
        let skills: Arc<dyn SkillPort> = Arc::new(EmptySkillStore);
        Arc::new(AgentLoop::new(
            Arc::new(MockProvider) as Arc<dyn ModelPort>,
            Arc::new(ToolRegistry::new()),
            personality,
            memory,
            skills,
            AgentConfig {
                model: "mock-model".to_string(),
                temperature: 0.0,
                max_tokens: 256,
                max_iterations: 6,
                token_budget: 2048,
                working_dir: temp.path().to_path_buf(),
            },
        ))
    }

    fn runtime_and_service(
        initial: AgentFileConfig,
    ) -> (tempfile::TempDir, Arc<ConfigRuntime>, ConfigService) {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("config.json");
        write_config_file(&config_path, &initial).expect("write config");
        let agent = test_agent(&temp);
        let runtime = Arc::new(ConfigRuntime::new(
            Arc::new(RwLock::new(agent.clone())),
            Arc::new(StaticAgentFactory { agent }),
            initial.clone(),
            AppConfig::from_inputs(initial, EnvSecrets::default(), temp.path().to_path_buf()),
            temp.path().to_path_buf(),
            EnvSecrets::default(),
            config_path,
            RestartMode::Disabled,
        ));
        let service = ConfigService::new(Some(runtime.clone()));
        (temp, runtime, service)
    }

    #[tokio::test]
    async fn config_service_requires_runtime() {
        let service = ConfigService::new(None);
        let error = service
            .get()
            .await
            .expect_err("missing runtime should fail");
        assert_eq!(error.code_str(), "service_unavailable");
    }

    #[tokio::test]
    async fn config_service_get_reset_apply_and_restart_cover_core_paths() {
        let initial = AgentFileConfig {
            llm: AgentLlmConfig {
                provider: Some("mock".to_string()),
                model: Some("mock-model".to_string()),
                ..Default::default()
            },
            logging: AgentLoggingConfig {
                level: Some("error".to_string()),
                ..Default::default()
            },
            ..AgentFileConfig::default()
        };
        let (_temp, runtime, service) = runtime_and_service(initial);

        let state = service.get().await.expect("get");
        assert_eq!(state.running.llm.model.as_deref(), Some("mock-model"));
        assert_eq!(state.config_format, "config.json");

        let applied = service
            .apply(ConfigMutationInput::Raw(
                r#"{"llm":{"provider":"mock","model":"mock-model-v2"},"logging":{"level":"debug"}}"#
                    .to_string(),
            ))
            .await
            .expect("apply raw");
        assert_eq!(applied.action, "apply");
        assert_eq!(
            applied.state.running.llm.model.as_deref(),
            Some("mock-model-v2")
        );
        assert_eq!(
            applied.state.running.logging.level.as_deref(),
            Some("debug")
        );

        let restarted = service
            .restart(ConfigRestartInput::Structured(AgentFileConfig {
                llm: AgentLlmConfig {
                    provider: Some("mock".to_string()),
                    model: Some("mock-model-v3".to_string()),
                    ..Default::default()
                },
                logging: AgentLoggingConfig {
                    level: Some("warn".to_string()),
                    ..Default::default()
                },
                ..AgentFileConfig::default()
            }))
            .await
            .expect("restart");
        assert_eq!(restarted.action, "restart");
        assert!(!restarted.restart_scheduled);
        assert_eq!(
            restarted.state.running.llm.model.as_deref(),
            Some("mock-model-v3")
        );

        std::fs::write(runtime.config_path(), "{\"corrupted\":").expect("corrupt config");
        let reset = service.reset().await.expect("reset");
        assert_eq!(reset.action, "reset");
        assert!(reset.state.disk_parse_error.is_none());
        let disk = runtime
            .disk_config()
            .await
            .expect("disk config after reset")
            .0;
        assert_eq!(disk.llm.model.as_deref(), Some("mock-model-v3"));
    }

    #[tokio::test]
    async fn build_config_state_response_falls_back_when_disk_config_is_invalid() {
        let initial = AgentFileConfig {
            llm: AgentLlmConfig {
                provider: Some("mock".to_string()),
                model: Some("mock-model".to_string()),
                ..Default::default()
            },
            ..AgentFileConfig::default()
        };
        let (_temp, runtime, _service) = runtime_and_service(initial);
        std::fs::write(runtime.config_path(), "{not-json").expect("write invalid config");

        let state = build_config_state_response(&runtime).await;
        assert!(state.disk_parse_error.is_some());
        assert_eq!(state.disk.llm.model.as_deref(), Some("mock-model"));
        assert!(state.raw.contains("mock-model"));
    }
}
