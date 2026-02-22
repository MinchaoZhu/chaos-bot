use crate::config::AppConfig;
use crate::types::{Message, Role, ToolCall, ToolSpec, Usage};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use futures::{stream, Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::Arc;

pub type LlmStream = Pin<Box<dyn Stream<Item = Result<LlmStreamEvent>> + Send>>;

type ByteStream = Pin<Box<dyn Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send>>;

#[derive(Clone, Debug)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSpec>,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Clone, Debug)]
pub struct LlmResponse {
    pub message: Message,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Option<Usage>,
    pub finish_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmStreamEvent {
    pub delta: String,
    pub tool_call: Option<ToolCall>,
    pub done: bool,
    pub usage: Option<Usage>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn chat(&self, request: LlmRequest) -> Result<LlmResponse>;
    async fn chat_stream(&self, request: LlmRequest) -> Result<LlmStream>;
}

pub fn build_provider(config: &AppConfig) -> Result<Arc<dyn LlmProvider>> {
    match config.provider.to_lowercase().as_str() {
        "openai" => {
            let api_key = config
                .openai_api_key
                .clone()
                .ok_or_else(|| anyhow!("OPENAI_API_KEY is required when CHAOS_PROVIDER=openai"))?;
            Ok(Arc::new(OpenAiProvider::new(api_key)))
        }
        "anthropic" => Ok(Arc::new(AnthropicProvider)),
        "gemini" => Ok(Arc::new(GeminiProvider)),
        other => Err(anyhow!("unsupported provider: {other}")),
    }
}

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

struct OpenAiStreamState {
    stream: ByteStream,
    text_buffer: String,
    pending: VecDeque<Result<LlmStreamEvent>>,
    tool_ids: HashMap<u64, String>,
    tool_names: HashMap<u64, String>,
    tool_args: HashMap<u64, String>,
    usage: Option<Usage>,
    done: bool,
    emitted_done: bool,
}

impl OpenAiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: std::env::var("OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
        }
    }

    fn map_messages(messages: &[Message]) -> Vec<Value> {
        messages
            .iter()
            .map(|message| match message.role {
                Role::System => json!({"role": "system", "content": message.content}),
                Role::User => json!({"role": "user", "content": message.content}),
                Role::Assistant => json!({"role": "assistant", "content": message.content}),
                Role::Tool => json!({
                    "role": "tool",
                    "content": message.content,
                    "tool_call_id": message.tool_call_id,
                }),
            })
            .collect()
    }

    fn map_tools(tools: &[ToolSpec]) -> Vec<Value> {
        tools
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters_schema,
                    }
                })
            })
            .collect()
    }

    fn parse_usage(data: &Value) -> Option<Usage> {
        data.get("usage").and_then(|value| {
            Some(Usage {
                prompt_tokens: value.get("prompt_tokens")?.as_u64()? as u32,
                completion_tokens: value.get("completion_tokens")?.as_u64()? as u32,
                total_tokens: value.get("total_tokens")?.as_u64()? as u32,
            })
        })
    }

    fn parse_tool_calls_from_message(message_data: &Value) -> Vec<ToolCall> {
        message_data
            .get("tool_calls")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .map(|item| {
                        let id = item
                            .get("id")
                            .and_then(|value| value.as_str())
                            .unwrap_or("tool_call")
                            .to_string();
                        let function = item.get("function").cloned().unwrap_or_default();
                        let name = function
                            .get("name")
                            .and_then(|value| value.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let arguments_raw = function
                            .get("arguments")
                            .and_then(|value| value.as_str())
                            .unwrap_or("{}");
                        let arguments = serde_json::from_str(arguments_raw)
                            .unwrap_or_else(|_| json!({"raw": arguments_raw}));
                        ToolCall {
                            id,
                            name,
                            arguments,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn drain_sse_payloads(buffer: &mut String) -> Vec<String> {
        let mut payloads = Vec::new();
        while let Some(index) = buffer.find("\n\n") {
            let chunk = buffer[..index].to_string();
            buffer.drain(..index + 2);

            let mut data = String::new();
            for line in chunk.lines() {
                if let Some(rest) = line.strip_prefix("data:") {
                    if !data.is_empty() {
                        data.push('\n');
                    }
                    data.push_str(rest.trim_start());
                }
            }

            if !data.is_empty() {
                payloads.push(data);
            }
        }
        payloads
    }

    fn flush_tool_calls(state: &mut OpenAiStreamState) {
        let mut indexes = state.tool_args.keys().copied().collect::<Vec<_>>();
        indexes.sort_unstable();

        for index in indexes {
            let args_raw = state.tool_args.remove(&index).unwrap_or_default();
            let id = state
                .tool_ids
                .remove(&index)
                .unwrap_or_else(|| format!("tool_call_{index}"));
            let name = state
                .tool_names
                .remove(&index)
                .unwrap_or_else(|| "unknown".to_string());

            let arguments = serde_json::from_str(&args_raw)
                .unwrap_or_else(|_| json!({"raw": args_raw}));
            state.pending.push_back(Ok(LlmStreamEvent {
                delta: String::new(),
                tool_call: Some(ToolCall {
                    id,
                    name,
                    arguments,
                }),
                done: false,
                usage: None,
            }));
        }
    }

    fn process_stream_payload(state: &mut OpenAiStreamState, payload: &str) -> Result<()> {
        if payload.trim() == "[DONE]" {
            state.done = true;
            if !state.emitted_done {
                state.pending.push_back(Ok(LlmStreamEvent {
                    delta: String::new(),
                    tool_call: None,
                    done: true,
                    usage: state.usage.clone(),
                }));
                state.emitted_done = true;
            }
            return Ok(());
        }

        let data: Value = serde_json::from_str(payload)
            .with_context(|| format!("failed to decode OpenAI stream payload: {payload}"))?;

        if let Some(usage) = Self::parse_usage(&data) {
            state.usage = Some(usage);
        }

        let choice = data
            .get("choices")
            .and_then(|value| value.as_array())
            .and_then(|choices| choices.first())
            .cloned()
            .unwrap_or_default();

        let delta = choice.get("delta").cloned().unwrap_or_default();
        if let Some(text) = delta.get("content").and_then(|value| value.as_str()) {
            if !text.is_empty() {
                state.pending.push_back(Ok(LlmStreamEvent {
                    delta: text.to_string(),
                    tool_call: None,
                    done: false,
                    usage: None,
                }));
            }
        }

        if let Some(calls) = delta.get("tool_calls").and_then(|value| value.as_array()) {
            for call in calls {
                let index = call.get("index").and_then(|value| value.as_u64()).unwrap_or(0);

                if let Some(id) = call.get("id").and_then(|value| value.as_str()) {
                    state.tool_ids.insert(index, id.to_string());
                }
                if let Some(name) = call
                    .get("function")
                    .and_then(|value| value.get("name"))
                    .and_then(|value| value.as_str())
                {
                    state.tool_names.insert(index, name.to_string());
                }
                if let Some(partial_args) = call
                    .get("function")
                    .and_then(|value| value.get("arguments"))
                    .and_then(|value| value.as_str())
                {
                    state
                        .tool_args
                        .entry(index)
                        .and_modify(|value| value.push_str(partial_args))
                        .or_insert_with(|| partial_args.to_string());
                }
            }
        }

        if choice.get("finish_reason").and_then(|value| value.as_str()) == Some("tool_calls") {
            Self::flush_tool_calls(state);
        }

        Ok(())
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    async fn chat(&self, request: LlmRequest) -> Result<LlmResponse> {
        let mut payload = json!({
            "model": request.model,
            "messages": Self::map_messages(&request.messages),
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        });

        if !request.tools.is_empty() {
            payload["tools"] = json!(Self::map_tools(&request.tools));
            payload["tool_choice"] = json!("auto");
        }

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url.trim_end_matches('/')))
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .context("failed to call OpenAI chat completions")?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI API error {status}: {text}"));
        }

        let data: Value = response.json().await?;
        let choice = data
            .get("choices")
            .and_then(|value| value.as_array())
            .and_then(|choices| choices.first())
            .ok_or_else(|| anyhow!("OpenAI response does not contain choices"))?;

        let message_data = choice
            .get("message")
            .ok_or_else(|| anyhow!("OpenAI response does not contain message"))?;
        let finish_reason = choice
            .get("finish_reason")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned);

        let content = message_data
            .get("content")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();

        let usage = Self::parse_usage(&data);

        Ok(LlmResponse {
            message: Message::assistant(content),
            tool_calls: Self::parse_tool_calls_from_message(message_data),
            usage,
            finish_reason,
        })
    }

    async fn chat_stream(&self, request: LlmRequest) -> Result<LlmStream> {
        let mut payload = json!({
            "model": request.model,
            "messages": Self::map_messages(&request.messages),
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "stream": true,
            "stream_options": {
                "include_usage": true
            }
        });

        if !request.tools.is_empty() {
            payload["tools"] = json!(Self::map_tools(&request.tools));
            payload["tool_choice"] = json!("auto");
        }

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url.trim_end_matches('/')))
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .context("failed to call OpenAI chat completions (stream)")?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI API stream error {status}: {text}"));
        }

        let state = OpenAiStreamState {
            stream: Box::pin(response.bytes_stream()),
            text_buffer: String::new(),
            pending: VecDeque::new(),
            tool_ids: HashMap::new(),
            tool_names: HashMap::new(),
            tool_args: HashMap::new(),
            usage: None,
            done: false,
            emitted_done: false,
        };

        let stream = stream::unfold(state, |mut state| async move {
            loop {
                if let Some(event) = state.pending.pop_front() {
                    return Some((event, state));
                }

                if state.done {
                    return None;
                }

                match state.stream.next().await {
                    Some(Ok(bytes)) => {
                        let chunk = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
                        state.text_buffer.push_str(&chunk);

                        for payload in OpenAiProvider::drain_sse_payloads(&mut state.text_buffer) {
                            if let Err(error) = OpenAiProvider::process_stream_payload(&mut state, &payload) {
                                state.done = true;
                                state.pending.push_back(Err(error));
                                break;
                            }
                        }
                    }
                    Some(Err(error)) => {
                        state.done = true;
                        return Some((Err(anyhow!("OpenAI streaming read error: {error}")), state));
                    }
                    None => {
                        state.done = true;
                        if !state.emitted_done {
                            state.pending.push_back(Ok(LlmStreamEvent {
                                delta: String::new(),
                                tool_call: None,
                                done: true,
                                usage: state.usage.clone(),
                            }));
                            state.emitted_done = true;
                        }
                    }
                }
            }
        });

        Ok(Box::pin(stream))
    }
}

pub struct AnthropicProvider;
pub struct GeminiProvider;

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    async fn chat(&self, _request: LlmRequest) -> Result<LlmResponse> {
        Err(anyhow!("Anthropic provider scaffold not implemented yet"))
    }

    async fn chat_stream(&self, _request: LlmRequest) -> Result<LlmStream> {
        Err(anyhow!("Anthropic streaming scaffold not implemented yet"))
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    fn name(&self) -> &'static str {
        "gemini"
    }

    async fn chat(&self, _request: LlmRequest) -> Result<LlmResponse> {
        Err(anyhow!("Gemini provider scaffold not implemented yet"))
    }

    async fn chat_stream(&self, _request: LlmRequest) -> Result<LlmStream> {
        Err(anyhow!("Gemini streaming scaffold not implemented yet"))
    }
}
