use crate::runtime::config_runtime::ConfigRuntime;
use crate::domain::config::{
    ConfigMutationInput, ConfigMutationResponse, ConfigRestartInput, ConfigStateResponse,
};
use crate::domain::{audit, AppError};
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
