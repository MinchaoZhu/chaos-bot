use crate::domain::types::SessionState;
use crate::domain::AppError;
use crate::infrastructure::session_store::SessionStore;

#[derive(Clone)]
pub struct SessionService {
    sessions: SessionStore,
}

impl SessionService {
    pub fn new(sessions: SessionStore) -> Self {
        Self { sessions }
    }

    pub async fn create(&self) -> Result<SessionState, AppError> {
        self.sessions
            .create()
            .await
            .map_err(|error| AppError::internal(format!("failed to create session: {error}")))
    }

    pub async fn list(&self) -> Result<Vec<SessionState>, AppError> {
        self.sessions
            .list()
            .await
            .map_err(|error| AppError::internal(format!("failed to list sessions: {error}")))
    }

    pub async fn get(&self, id: &str) -> Result<SessionState, AppError> {
        self.sessions
            .get(id)
            .await
            .map_err(|error| AppError::internal(format!("failed to get session: {error}")))?
            .ok_or_else(|| AppError::not_found("session not found"))
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        if self
            .sessions
            .delete(id)
            .await
            .map_err(|error| AppError::internal(format!("failed to delete session: {error}")))?
        {
            Ok(())
        } else {
            Err(AppError::not_found("session not found"))
        }
    }
}
