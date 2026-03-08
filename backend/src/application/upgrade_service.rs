use crate::domain::ports::UpgradePort;
use crate::domain::upgrade::{UpgradeApplyResult, UpgradeStatus};
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
