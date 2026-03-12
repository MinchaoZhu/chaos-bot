use crate::domain::ports::UpgradePort;
use crate::domain::upgrade::{UpgradeApplyResult, UpgradeRestartResult, UpgradeStatus};
use crate::domain::AppError;
use std::sync::Arc;

#[derive(Clone)]
pub struct UpgradeService {
    runtime: Option<Arc<dyn UpgradePort>>,
}

impl UpgradeService {
    pub fn new(runtime: Option<Arc<dyn UpgradePort>>) -> Self {
        Self { runtime }
    }

    pub async fn status(&self) -> Result<UpgradeStatus, AppError> {
        let runtime = self.require_runtime()?;
        runtime
            .status()
            .await
            .map_err(|error| map_internal(error, "status"))
    }

    pub async fn apply(&self) -> Result<UpgradeApplyResult, AppError> {
        let runtime = self.require_runtime()?;
        runtime
            .apply()
            .await
            .map_err(|error| map_internal(error, "apply"))
    }

    pub async fn relaunch(&self) -> Result<UpgradeRestartResult, AppError> {
        let runtime = self.require_runtime()?;
        runtime
            .relaunch()
            .await
            .map_err(|error| map_internal(error, "relaunch"))
    }

    fn require_runtime(&self) -> Result<Arc<dyn UpgradePort>, AppError> {
        self.runtime
            .clone()
            .ok_or_else(|| AppError::service_unavailable("upgrade runtime unavailable"))
    }
}

fn map_internal(error: anyhow::Error, action: &str) -> AppError {
    tracing::warn!(action, error = %error, "upgrade endpoint failed");
    AppError::internal(format!("upgrade {action} failed"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::UpgradePort;
    use anyhow::{anyhow, Result};
    use async_trait::async_trait;

    #[derive(Clone)]
    struct TestUpgradePort {
        fail_action: Option<&'static str>,
    }

    #[async_trait]
    impl UpgradePort for TestUpgradePort {
        async fn status(&self) -> Result<UpgradeStatus> {
            if self.fail_action == Some("status") {
                return Err(anyhow!("status failed"));
            }
            Ok(UpgradeStatus {
                supported: true,
                current_version: Some("0.1.0".to_string()),
                latest_version: Some("0.1.1".to_string()),
                latest_tag_name: Some("v0.1.1".to_string()),
                upgrade_available: true,
                install_prefix: Some("/opt/chaos-bot".to_string()),
                current_release_root: Some("/opt/chaos-bot/releases/0.1.0".to_string()),
                repository: Some("test/chaos-bot".to_string()),
                latest_release_url: Some("https://example.invalid/latest".to_string()),
                download_url: Some("https://example.invalid/download".to_string()),
                reason: None,
            })
        }

        async fn apply(&self) -> Result<UpgradeApplyResult> {
            if self.fail_action == Some("apply") {
                return Err(anyhow!("apply failed"));
            }
            Ok(UpgradeApplyResult {
                ok: true,
                action: "upgrade",
                current_version: Some("0.1.0".to_string()),
                target_version: Some("0.1.1".to_string()),
                launcher_path: Some("/opt/chaos-bot/bin/chaos-bot".to_string()),
                installed_release_root: Some("/opt/chaos-bot/releases/0.1.1".to_string()),
                relaunch_required: true,
                message: "installed new release".to_string(),
                status: self.status().await?,
            })
        }

        async fn relaunch(&self) -> Result<UpgradeRestartResult> {
            if self.fail_action == Some("relaunch") {
                return Err(anyhow!("relaunch failed"));
            }
            Ok(UpgradeRestartResult {
                ok: true,
                action: "relaunch",
                launcher_path: Some("/opt/chaos-bot/bin/chaos-bot".to_string()),
                target_version: Some("0.1.1".to_string()),
                message: "restart scheduled".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn upgrade_service_requires_runtime() {
        let service = UpgradeService::new(None);
        let error = service
            .status()
            .await
            .expect_err("missing runtime should fail");
        assert_eq!(error.code_str(), "service_unavailable");
    }

    #[tokio::test]
    async fn upgrade_service_status_apply_and_relaunch_delegate_to_runtime() {
        let service = UpgradeService::new(Some(Arc::new(TestUpgradePort { fail_action: None })));

        let status = service.status().await.expect("status");
        assert!(status.supported);
        assert!(status.upgrade_available);

        let apply = service.apply().await.expect("apply");
        assert_eq!(apply.action, "upgrade");
        assert!(apply.relaunch_required);

        let relaunch = service.relaunch().await.expect("relaunch");
        assert_eq!(relaunch.action, "relaunch");
        assert!(relaunch.message.contains("restart scheduled"));
    }

    #[tokio::test]
    async fn upgrade_service_maps_internal_failures() {
        let service = UpgradeService::new(Some(Arc::new(TestUpgradePort {
            fail_action: Some("apply"),
        })));
        let error = service.apply().await.expect_err("apply should fail");
        assert_eq!(error.code_str(), "internal_error");
        assert_eq!(error.message(), "upgrade apply failed");
    }
}
