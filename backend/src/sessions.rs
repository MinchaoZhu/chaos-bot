use crate::types::SessionState;
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
        session
    }

    pub async fn get(&self, id: &str) -> Option<SessionState> {
        self.inner.read().await.get(id).cloned()
    }

    pub async fn upsert(&self, session: SessionState) {
        self.inner.write().await.insert(session.id.clone(), session);
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
        sessions
    }

    pub async fn delete(&self, id: &str) -> bool {
        self.inner.write().await.remove(id).is_some()
    }
}
