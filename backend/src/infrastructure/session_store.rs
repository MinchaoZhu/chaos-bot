use crate::domain::types::SessionState;
use anyhow::{bail, Context, Result};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct SessionStore {
    root: Arc<PathBuf>,
}

impl SessionStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root: Arc::new(root),
        }
    }

    pub async fn ensure_layout(&self) -> Result<()> {
        tokio::fs::create_dir_all(&*self.root)
            .await
            .with_context(|| format!("failed to create sessions dir: {}", self.root.display()))
    }

    pub async fn create(&self) -> Result<SessionState> {
        let session = SessionState::new(Uuid::new_v4().to_string());
        self.upsert(session.clone()).await?;
        tracing::debug!(session_id = %session.id, "created session");
        Ok(session)
    }

    pub async fn get(&self, id: &str) -> Result<Option<SessionState>> {
        let Some(path) = self.session_path(id)? else {
            return Ok(None);
        };
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tracing::debug!(session_id = %id, found = false, "fetched session");
            return Ok(None);
        }

        let raw = tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("failed to read session file: {}", path.display()))?;
        let session = serde_json::from_str::<SessionState>(&raw)
            .with_context(|| format!("invalid session json: {}", path.display()))?;
        tracing::debug!(session_id = %id, found = true, "fetched session");
        Ok(Some(session))
    }

    pub async fn upsert(&self, session: SessionState) -> Result<()> {
        let session_id = session.id.clone();
        let path = self
            .session_path(&session_id)?
            .context("invalid session id")?;
        self.ensure_layout().await?;
        let raw = format!("{}\n", serde_json::to_string_pretty(&session)?);
        tokio::fs::write(&path, raw)
            .await
            .with_context(|| format!("failed to write session file: {}", path.display()))?;
        tracing::debug!(session_id = %session_id, "upserted session");
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<SessionState>> {
        self.ensure_layout().await?;
        let mut dir = tokio::fs::read_dir(&*self.root)
            .await
            .with_context(|| format!("failed to read sessions dir: {}", self.root.display()))?;
        let mut sessions = Vec::new();

        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension() != Some(OsStr::new("json")) {
                continue;
            }
            let raw = tokio::fs::read_to_string(&path)
                .await
                .with_context(|| format!("failed to read session file: {}", path.display()))?;
            let session = serde_json::from_str::<SessionState>(&raw)
                .with_context(|| format!("invalid session json: {}", path.display()))?;
            sessions.push(session);
        }

        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        tracing::debug!(count = sessions.len(), "listed sessions");
        Ok(sessions)
    }

    pub async fn delete(&self, id: &str) -> Result<bool> {
        let Some(path) = self.session_path(id)? else {
            return Ok(false);
        };
        let deleted = if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tokio::fs::remove_file(&path)
                .await
                .with_context(|| format!("failed to delete session file: {}", path.display()))?;
            true
        } else {
            false
        };
        tracing::debug!(session_id = %id, deleted, "deleted session");
        Ok(deleted)
    }

    pub fn root(&self) -> &Path {
        self.root.as_ref()
    }

    fn session_path(&self, id: &str) -> Result<Option<PathBuf>> {
        if id.is_empty() {
            bail!("session id is empty");
        }
        if id == "." || id == ".." || id.contains('/') || id.contains('\\') {
            return Ok(None);
        }
        Ok(Some(self.root.join(format!("{id}.json"))))
    }
}
