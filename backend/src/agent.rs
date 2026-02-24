use crate::config::AppConfig;
use crate::llm::{LlmProvider, LlmRequest};
use crate::memory::{MemoryBackend, MemoryHit};
use crate::personality::PersonalitySource;
use crate::tools::{ToolContext, ToolRegistry};
use crate::types::{Message, SessionState, ToolCall, ToolResult, Usage};
use anyhow::Result;
use futures::StreamExt;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub max_iterations: usize,
    pub token_budget: u32,
    pub working_dir: PathBuf,
}

impl From<&AppConfig> for AgentConfig {
    fn from(value: &AppConfig) -> Self {
        Self {
            model: value.model.clone(),
            temperature: value.temperature,
            max_tokens: value.max_tokens,
            max_iterations: value.max_iterations,
            token_budget: value.token_budget,
            working_dir: value.working_dir.clone(),
        }
    }
}

#[derive(Clone)]
pub struct AgentLoop {
    provider: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    personality: Arc<dyn PersonalitySource>,
    memory: Arc<dyn MemoryBackend>,
    config: AgentConfig,
}

#[derive(Clone, Debug, Serialize)]
pub struct ToolEvent {
    pub call: ToolCall,
    pub result: ToolResult,
}

#[derive(Clone, Debug)]
pub enum AgentStreamEvent {
    Delta(String),
    Tool(ToolEvent),
}

#[derive(Clone, Debug, Serialize)]
pub struct AgentRunOutput {
    pub assistant_message: Message,
    pub tool_events: Vec<ToolEvent>,
    pub usage: Option<Usage>,
    pub finish_reason: Option<String>,
}

impl AgentLoop {
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        tools: Arc<ToolRegistry>,
        personality: Arc<dyn PersonalitySource>,
        memory: Arc<dyn MemoryBackend>,
        config: AgentConfig,
    ) -> Self {
        Self {
            provider,
            tools,
            personality,
            memory,
            config,
        }
    }

    pub async fn run(
        &self,
        session: &mut SessionState,
        user_input: String,
    ) -> Result<AgentRunOutput> {
        self.run_stream(session, user_input, |_| {}).await
    }

    pub async fn run_stream<F>(
        &self,
        session: &mut SessionState,
        user_input: String,
        mut on_event: F,
    ) -> Result<AgentRunOutput>
    where
        F: FnMut(AgentStreamEvent),
    {
        let system_prompt = self.personality.system_prompt().await?;
        let memory_context = self.memory.search(&user_input).await.unwrap_or_default();

        let user_message = Message::user(user_input.clone());
        session.push_message(user_message);

        let mut messages = vec![Message::system(Self::build_system_prompt(
            &system_prompt,
            &memory_context,
        ))];
        messages.extend(session.messages.clone());

        let mut usage = None;
        let mut finish_reason = None;
        let mut tool_events = Vec::new();

        for _ in 0..self.config.max_iterations {
            Self::enforce_token_budget(&mut messages, self.config.token_budget);

            let mut stream = self
                .provider
                .chat_stream(LlmRequest {
                    model: self.config.model.clone(),
                    messages: messages.clone(),
                    tools: self.tools.specs(),
                    temperature: self.config.temperature,
                    max_tokens: self.config.max_tokens,
                })
                .await?;

            let mut assistant_content = String::new();
            let mut tool_calls = Vec::new();

            while let Some(event) = stream.next().await {
                let event = event?;

                if !event.delta.is_empty() {
                    assistant_content.push_str(&event.delta);
                    on_event(AgentStreamEvent::Delta(event.delta));
                }

                if let Some(tool_call) = event.tool_call {
                    tool_calls.push(tool_call);
                }

                if event.done {
                    usage = event.usage;
                }
            }

            let assistant_message = Message::assistant(assistant_content.clone());
            session.push_message(assistant_message.clone());
            messages.push(assistant_message.clone());

            if tool_calls.is_empty() {
                finish_reason = Some("stop".to_string());
                let summary = format!(
                    "User: {} | Assistant: {}",
                    user_input,
                    assistant_message
                        .content
                        .chars()
                        .take(160)
                        .collect::<String>()
                );
                let _ = self.memory.append_daily_log(&summary).await;

                return Ok(AgentRunOutput {
                    assistant_message,
                    tool_events,
                    usage,
                    finish_reason,
                });
            }

            finish_reason = Some("tool_calls".to_string());
            let tool_context =
                ToolContext::new(self.config.working_dir.clone(), self.memory.clone());

            for call in tool_calls {
                let result = match self
                    .tools
                    .dispatch(&call.id, &call.name, call.arguments.clone(), &tool_context)
                    .await
                {
                    Ok(output) => output,
                    Err(error) => ToolResult {
                        tool_call_id: call.id.clone(),
                        name: call.name.clone(),
                        output: format!("tool error: {error}"),
                        is_error: true,
                    },
                };

                let tool_message = Message::tool(&call.name, &call.id, &result.output);
                session.push_message(tool_message.clone());
                messages.push(tool_message);

                let tool_event = ToolEvent { call, result };
                on_event(AgentStreamEvent::Tool(tool_event.clone()));
                tool_events.push(tool_event);
            }
        }

        let assistant_message =
            Message::assistant("Agent reached max iterations without a final answer.");
        session.push_message(assistant_message.clone());

        Ok(AgentRunOutput {
            assistant_message,
            tool_events,
            usage,
            finish_reason,
        })
    }

    pub fn build_system_prompt(personality_prompt: &str, memory_context: &[MemoryHit]) -> String {
        let mut prompt = personality_prompt.trim().to_string();
        if !memory_context.is_empty() {
            let memory_block = memory_context
                .iter()
                .take(6)
                .map(|hit| format!("- {}:{}: {}", hit.path, hit.line, hit.snippet))
                .collect::<Vec<_>>()
                .join("\n");
            prompt.push_str("\n\n# Relevant Memory Context\n");
            prompt.push_str(&memory_block);
        }
        prompt
    }

    pub fn enforce_token_budget(messages: &mut Vec<Message>, token_budget: u32) {
        while Self::estimate_tokens(messages) > token_budget && messages.len() > 2 {
            messages.remove(1);
        }
    }

    pub fn estimate_tokens(messages: &[Message]) -> u32 {
        messages
            .iter()
            .map(|message| (message.content.len() / 4 + 8) as u32)
            .sum()
    }
}
