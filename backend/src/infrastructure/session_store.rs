use crate::domain::types::SessionState;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Default)]
struct SessionStoreState {
    sessions: HashMap<String, SessionState>,
    channel_bindings: HashMap<String, String>,
}

#[derive(Clone, Default)]
pub struct SessionStore {
    inner: Arc<RwLock<SessionStoreState>>,
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
            .sessions
            .insert(session.id.clone(), session.clone());
        tracing::debug!(session_id = %session.id, "created session");
        session
    }

    pub async fn get(&self, id: &str) -> Option<SessionState> {
        let found = self.inner.read().await.sessions.get(id).cloned();
        tracing::debug!(session_id = %id, found = found.is_some(), "fetched session");
        found
    }

    pub async fn upsert(&self, session: SessionState) {
        let session_id = session.id.clone();
        self.inner
            .write()
            .await
            .sessions
            .insert(session_id.clone(), session);
        tracing::debug!(session_id = %session_id, "upserted session");
    }

    pub async fn list(&self) -> Vec<SessionState> {
        let mut sessions = self
            .inner
            .read()
            .await
            .sessions
            .values()
            .cloned()
            .collect::<Vec<_>>();
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        tracing::debug!(count = sessions.len(), "listed sessions");
        sessions
    }

    pub async fn delete(&self, id: &str) -> bool {
        let mut state = self.inner.write().await;
        let deleted = state.sessions.remove(id).is_some();
        if deleted {
            state
                .channel_bindings
                .retain(|_, session_id| session_id != id);
        }
        tracing::debug!(session_id = %id, deleted, "deleted session");
        deleted
    }

    pub async fn bind_channel_session(&self, channel_key: &str, session_id: &str) {
        self.inner
            .write()
            .await
            .channel_bindings
            .insert(channel_key.to_string(), session_id.to_string());
        tracing::debug!(channel_key = %channel_key, session_id = %session_id, "bound channel key to session");
    }

    pub async fn session_for_channel_key(&self, channel_key: &str) -> Option<SessionState> {
        let state = self.inner.read().await;
        let session_id = state.channel_bindings.get(channel_key)?;
        state.sessions.get(session_id).cloned()
    }
}
