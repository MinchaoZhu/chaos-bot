use crate::domain::AppError;
use crate::infrastructure::session_store::SessionStore;
use crate::domain::types::SessionState;

#[derive(Clone)]
pub struct SessionService {
    sessions: SessionStore,
}

impl SessionService {
    pub fn new(sessions: SessionStore) -> Self {
        Self { sessions }
    }

    pub async fn create(&self) -> SessionState {
        self.sessions.create().await
    }

    pub async fn list(&self) -> Vec<SessionState> {
        self.sessions.list().await
    }

    pub async fn get(&self, id: &str) -> Result<SessionState, AppError> {
        self.sessions
            .get(id)
            .await
            .ok_or_else(|| AppError::not_found("session not found"))
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        if self.sessions.delete(id).await {
            Ok(())
        } else {
            Err(AppError::not_found("session not found"))
        }
    }
}
