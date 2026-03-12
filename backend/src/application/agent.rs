use crate::domain::chat::ToolEvent;
use crate::domain::ports::{
    MemoryHit, MemoryPort, ModelPort, ModelRequest, SkillPort, ToolExecutionContext,
    ToolExecutorPort,
};
use crate::domain::skills::SkillMeta;
use crate::domain::types::{Message, SessionState, ToolResult, Usage};
use crate::infrastructure::config::AppConfig;
use crate::infrastructure::personality::PersonalitySource;
use anyhow::Result;
use futures::StreamExt;
use serde::Serialize;
use serde_json;
use std::path::PathBuf;
use std::sync::Arc;

const RUNTIME_TOOL_GUIDANCE: &str = r#"# Runtime Tool Guidance
- For live local-environment facts, prefer a tool call over a guessed answer.
- Use `bash` for safe allowlisted commands such as `date`, `pwd`, `ls`, `rg`, `cat`, `head`, `tail`, `wc`, and `echo`.
- Use `bash date` for the current system time or date, `bash pwd` for the working directory, and `bash ls` or `bash rg` to confirm local files.
- Do not claim you cannot inspect the local machine when a safe tool can answer the question directly.
"#;

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
    provider: Arc<dyn ModelPort>,
    tools: Arc<dyn ToolExecutorPort>,
    personality: Arc<dyn PersonalitySource>,
    memory: Arc<dyn MemoryPort>,
    skills: Arc<dyn SkillPort>,
    config: AgentConfig,
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
        provider: Arc<dyn ModelPort>,
        tools: Arc<dyn ToolExecutorPort>,
        personality: Arc<dyn PersonalitySource>,
        memory: Arc<dyn MemoryPort>,
        skills: Arc<dyn SkillPort>,
        config: AgentConfig,
    ) -> Self {
        Self {
            provider,
            tools,
            personality,
            memory,
            skills,
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
        let memory_context = match self.memory.search(&user_input).await {
            Ok(hits) => hits,
            Err(error) => {
                tracing::warn!(error = %error, "memory search failed; continuing without context");
                Vec::new()
            }
        };

        // Load skill summaries for the system prompt header.
        let skill_list = match self.skills.list().await {
            Ok(list) => list,
            Err(error) => {
                tracing::warn!(error = %error, "skills list failed; continuing without skills");
                Vec::new()
            }
        };

        tracing::debug!(
            session_id = %session.id,
            input_chars = user_input.chars().count(),
            memory_hits = memory_context.len(),
            skill_count = skill_list.len(),
            "agent run_stream start"
        );

        let user_message = Message::user(user_input.clone());
        session.push_message(user_message);

        let mut messages = vec![Message::system(Self::build_system_prompt(
            &system_prompt,
            &memory_context,
            &skill_list,
        ))];
        messages.extend(session.messages.clone());

        let mut usage = None;
        let mut finish_reason = None;
        let mut tool_events = Vec::new();

        for iteration in 0..self.config.max_iterations {
            tracing::debug!(
                session_id = %session.id,
                iteration = iteration + 1,
                max_iterations = self.config.max_iterations,
                "agent iteration"
            );
            Self::enforce_token_budget(&mut messages, self.config.token_budget);
            let tool_specs = self.tools.specs();
            Self::log_assembled_chat(&session.id, iteration + 1, &messages, &tool_specs);

            let mut stream = self
                .provider
                .chat_stream(ModelRequest {
                    model: self.config.model.clone(),
                    messages: messages.clone(),
                    tools: tool_specs,
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

            let assistant_message = if tool_calls.is_empty() {
                Message::assistant(assistant_content.clone())
            } else {
                Message::assistant_with_tool_calls(assistant_content.clone(), tool_calls.clone())
            };
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
                tracing::info!(
                    session_id = %session.id,
                    assistant_chars = assistant_message.content.chars().count(),
                    "agent completed without tool calls"
                );

                return Ok(AgentRunOutput {
                    assistant_message,
                    tool_events,
                    usage,
                    finish_reason,
                });
            }

            finish_reason = Some("tool_calls".to_string());
            let tool_context =
                ToolExecutionContext::new(self.config.working_dir.clone(), self.memory.clone());
            tracing::debug!(
                session_id = %session.id,
                tool_calls = tool_calls.len(),
                "agent executing tool calls"
            );

            for call in tool_calls {
                tracing::debug!(
                    session_id = %session.id,
                    tool_name = %call.name,
                    tool_call_id = %call.id,
                    "agent dispatch tool call"
                );
                let result = match self
                    .tools
                    .execute(&call.id, &call.name, call.arguments.clone(), &tool_context)
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
                if result.is_error {
                    tracing::warn!(
                        session_id = %session.id,
                        tool_name = %call.name,
                        tool_call_id = %call.id,
                        "agent tool call returned error"
                    );
                }

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
        tracing::warn!(
            session_id = %session.id,
            max_iterations = self.config.max_iterations,
            "agent reached max iterations"
        );

        Ok(AgentRunOutput {
            assistant_message,
            tool_events,
            usage,
            finish_reason,
        })
    }

    /// Build the system prompt from personality, memory context, and available skill headers.
    pub fn build_system_prompt(
        personality_prompt: &str,
        memory_context: &[MemoryHit],
        skills: &[SkillMeta],
    ) -> String {
        let mut prompt = personality_prompt.trim().to_string();
        prompt.push_str("\n\n");
        prompt.push_str(RUNTIME_TOOL_GUIDANCE);

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

        if !skills.is_empty() {
            prompt.push_str("\n\n# Available Skill Headers\n");
            prompt.push_str(
                "When needed, call tool `load_skill` with JSON: {\"skill_name\":\"<skill-folder-id>\"}.\n",
            );
            for skill in skills {
                prompt.push_str("\n---\n");
                prompt.push_str(&format!("name: {}\n", skill.id));
                prompt.push_str(&format!(
                    "description: {}\n",
                    if skill.description.is_empty() {
                        "(no description)"
                    } else {
                        skill.description.as_str()
                    }
                ));
                prompt.push_str("---\n");
            }
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

    fn log_assembled_chat(
        session_id: &str,
        iteration: usize,
        messages: &[Message],
        tool_specs: &[crate::domain::types::ToolSpec],
    ) {
        if !tracing::enabled!(tracing::Level::DEBUG) {
            return;
        }

        let mut tool_names = tool_specs
            .iter()
            .map(|spec| spec.name.clone())
            .collect::<Vec<_>>();
        tool_names.sort_unstable();
        let assembled_chat = serde_json::to_string_pretty(messages)
            .unwrap_or_else(|error| format!("failed to serialize assembled chat: {error}"));

        tracing::debug!(
            session_id,
            iteration,
            message_count = messages.len(),
            tool_count = tool_names.len(),
            tool_names = ?tool_names,
            assembled_chat = %assembled_chat,
            "agent final assembled chat"
        );
    }
}
