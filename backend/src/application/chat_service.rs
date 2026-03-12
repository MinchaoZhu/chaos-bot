use crate::application::agent::{AgentLoop, AgentStreamEvent};
use crate::domain::chat::{ChatCommand, ChatEvent, ChatResult};
use crate::domain::types::SessionState;
use crate::domain::{audit, AppError};
use crate::infrastructure::session_store::SessionStore;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ChatService {
    agent: Arc<RwLock<Arc<AgentLoop>>>,
    sessions: SessionStore,
}

impl ChatService {
    pub fn new(agent: Arc<RwLock<Arc<AgentLoop>>>, sessions: SessionStore) -> Self {
        Self { agent, sessions }
    }

    pub async fn run_stream<F>(
        &self,
        command: ChatCommand,
        mut on_event: F,
    ) -> Result<ChatResult, AppError>
    where
        F: FnMut(ChatEvent),
    {
        tracing::info!(
            has_session_id = command.session_id.is_some(),
            message_chars = command.message.chars().count(),
            "cli chat request"
        );

        let (session_id, mut session) = self.resolve_session(command.session_id).await?;
        on_event(ChatEvent::Session {
            session_id: session_id.clone(),
        });

        let agent = self.agent.read().await.clone();
        let result = agent
            .run_stream(&mut session, command.message, |event| match event {
                AgentStreamEvent::Delta(chunk) => on_event(ChatEvent::Delta(chunk)),
                AgentStreamEvent::Tool(tool) => {
                    let redacted = audit::redact_json(&tool.call.arguments);
                    tracing::info!(
                        tool_call_id = %tool.call.id,
                        tool_name = %tool.call.name,
                        tool_args = %redacted,
                        is_error = tool.result.is_error,
                        "tool call audit"
                    );
                    on_event(ChatEvent::Tool(tool));
                }
            })
            .await;

        self.sessions
            .upsert(session)
            .await
            .map_err(|error| AppError::internal(format!("failed to persist session: {error}")))?;
        tracing::debug!(session_id = %session_id, "chat session persisted");

        match result {
            Ok(output) => {
                tracing::info!(
                    session_id = %session_id,
                    finish_reason = output.finish_reason.as_deref().unwrap_or("unknown"),
                    usage_total_tokens = output.usage.as_ref().map(|u| u.total_tokens),
                    "chat completed"
                );

                Ok(ChatResult {
                    session_id,
                    usage: output.usage,
                    finish_reason: output.finish_reason,
                    assistant_message: output.assistant_message.content,
                })
            }
            Err(error) => {
                tracing::warn!(session_id = %session_id, error = %error, "chat run_stream failed");
                Err(AppError::internal(error.to_string()))
            }
        }
    }

    async fn resolve_session(
        &self,
        requested: Option<String>,
    ) -> Result<(String, SessionState), AppError> {
        match requested {
            Some(id) => match self.sessions.get(&id).await {
                Ok(Some(existing)) => {
                    tracing::debug!(session_id = %id, "chat using existing session");
                    Ok((id, existing))
                }
                Ok(None) => {
                    tracing::info!(session_id = %id, "chat created missing session id");
                    Ok((id.clone(), SessionState::new(id)))
                }
                Err(e) => Err(AppError::internal(format!(
                    "failed to read session {id}: {e}"
                ))),
            },
            None => {
                let created = self
                    .sessions
                    .create()
                    .await
                    .map_err(|e| AppError::internal(format!("failed to create session: {e}")))?;
                tracing::info!(session_id = %created.id, "chat auto-created session");
                Ok((created.id.clone(), created))
            }
        }
    }
}
