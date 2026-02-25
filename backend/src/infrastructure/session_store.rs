use crate::domain::types::SessionState;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct SessionStore {
    inner: Arc<RwLock<HashMap<String, SessionState>>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn create(&self) -> SessionState {
        let session = SessionState::new(Uuid::new_v4().to_string());
        self.inner
            .write()
            .await
            .insert(session.id.clone(), session.clone());
        tracing::debug!(session_id = %session.id, "created session");
        session
    }

    pub async fn get(&self, id: &str) -> Option<SessionState> {
        let found = self.inner.read().await.get(id).cloned();
        tracing::debug!(session_id = %id, found = found.is_some(), "fetched session");
        found
    }

    pub async fn upsert(&self, session: SessionState) {
        let session_id = session.id.clone();
        self.inner.write().await.insert(session_id.clone(), session);
        tracing::debug!(session_id = %session_id, "upserted session");
    }

    pub async fn list(&self) -> Vec<SessionState> {
        let mut sessions = self
            .inner
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        tracing::debug!(count = sessions.len(), "listed sessions");
        sessions
    }

    pub async fn delete(&self, id: &str) -> bool {
        let deleted = self.inner.write().await.remove(id).is_some();
        tracing::debug!(session_id = %id, deleted, "deleted session");
        deleted
    }
}
