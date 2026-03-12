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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn session_service_create_list_get_and_delete_roundtrip() {
        let temp = tempdir().expect("tempdir");
        let service = SessionService::new(SessionStore::new(temp.path().join("sessions")));

        let created = service.create().await.expect("create");
        assert!(!created.id.is_empty());

        let listed = service.list().await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, created.id);

        let fetched = service.get(&created.id).await.expect("get");
        assert_eq!(fetched.id, created.id);

        service.delete(&created.id).await.expect("delete");
        let error = service
            .get(&created.id)
            .await
            .expect_err("deleted session should fail");
        assert_eq!(error.code_str(), "not_found");
    }

    #[tokio::test]
    async fn session_service_delete_missing_session_returns_not_found() {
        let temp = tempdir().expect("tempdir");
        let service = SessionService::new(SessionStore::new(temp.path().join("sessions")));

        let error = service
            .delete("missing-session")
            .await
            .expect_err("missing session should fail");
        assert_eq!(error.code_str(), "not_found");
    }
}
