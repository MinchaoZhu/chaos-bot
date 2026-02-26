use crate::application::agent::{AgentLoop, AgentStreamEvent};
use crate::domain::chat::{
    ChannelContext, ChatCommand, ChatEvent, ChatResult, InboundChannelMessage,
    OutboundChannelMessage,
};
use crate::domain::ports::ChannelDispatcherPort;
use crate::domain::{audit, AppError};
use crate::domain::types::SessionState;
use crate::infrastructure::session_store::SessionStore;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ChatService {
    agent: Arc<RwLock<Arc<AgentLoop>>>,
    sessions: SessionStore,
    channel_dispatcher: Option<Arc<dyn ChannelDispatcherPort>>,
}

impl ChatService {
    pub fn new(
        agent: Arc<RwLock<Arc<AgentLoop>>>,
        sessions: SessionStore,
        channel_dispatcher: Option<Arc<dyn ChannelDispatcherPort>>,
    ) -> Self {
        Self {
            agent,
            sessions,
            channel_dispatcher,
        }
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
            channel = command
                .channel
                .as_ref()
                .map(|channel| channel.channel.as_str())
                .unwrap_or("web"),
            message_chars = command.message.chars().count(),
            "api chat request"
        );

        let (session_id, mut session) =
            self.resolve_session(command.session_id, command.channel.clone()).await;
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

        self.sessions.upsert(session).await;
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

    pub async fn run_channel_message(
        &self,
        inbound: InboundChannelMessage,
    ) -> Result<ChatResult, AppError> {
        let channel_context = ChannelContext {
            channel: inbound.channel.clone(),
            user_id: inbound.user_id.clone(),
            conversation_id: inbound.conversation_id.clone(),
        };
        let channel_name = channel_context.channel.clone();
        let conversation_id = channel_context.conversation_id.clone();
        let user_id = channel_context.user_id.clone();
        let metadata = inbound.metadata.clone();
        let delivery_text = inbound.text.clone();
        let result = self
            .run_stream(
                ChatCommand {
                    session_id: None,
                    message: inbound.text,
                    channel: Some(channel_context),
                },
                |_| {},
            )
            .await?;

        if let Some(dispatcher) = &self.channel_dispatcher {
            dispatcher
                .dispatch(OutboundChannelMessage {
                    channel: channel_name,
                    user_id,
                    conversation_id,
                    session_id: result.session_id.clone(),
                    text: result.assistant_message.clone(),
                    metadata: json!({
                        "source": "agent",
                        "inbound_message": delivery_text,
                        "inbound_metadata": metadata,
                    }),
                })
                .await
                .map_err(|error| AppError::internal(format!("channel dispatch failed: {error}")))?;
        }

        Ok(result)
    }

    async fn resolve_session(
        &self,
        requested: Option<String>,
        channel: Option<ChannelContext>,
    ) -> (String, SessionState) {
        match requested {
            Some(id) => match self.sessions.get(&id).await {
                Some(existing) => {
                    tracing::debug!(session_id = %id, "chat using existing session");
                    (id, existing)
                }
                None => {
                    tracing::info!(session_id = %id, "chat created missing session id");
                    (id.clone(), SessionState::new(id))
                }
            },
            None => {
                if let Some(channel) = channel {
                    let channel_key = channel_session_key(&channel);
                    if let Some(existing) = self.sessions.session_for_channel_key(&channel_key).await
                    {
                        tracing::info!(
                            channel = %channel.channel,
                            channel_key = %channel_key,
                            session_id = %existing.id,
                            "chat reusing mapped channel session"
                        );
                        return (existing.id.clone(), existing);
                    }
                    let created = self.sessions.create().await;
                    self.sessions
                        .bind_channel_session(&channel_key, &created.id)
                        .await;
                    tracing::info!(
                        channel = %channel.channel,
                        channel_key = %channel_key,
                        session_id = %created.id,
                        "chat mapped new channel session"
                    );
                    return (created.id.clone(), created);
                }

                let created = self.sessions.create().await;
                tracing::info!(session_id = %created.id, "chat auto-created session");
                (created.id.clone(), created)
            }
        }
    }
}

fn channel_session_key(channel: &ChannelContext) -> String {
    format!(
        "{}:{}:{}",
        channel.channel, channel.conversation_id, channel.user_id
    )
}
