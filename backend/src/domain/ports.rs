use crate::domain::types::{Message, ToolCall, ToolResult, ToolSpec, Usage};
use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

pub type ModelStream = Pin<Box<dyn Stream<Item = Result<ModelStreamEvent>> + Send>>;

#[derive(Clone, Debug)]
pub struct ModelRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSpec>,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Clone, Debug)]
pub struct ModelResponse {
    pub message: Message,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Option<Usage>,
    pub finish_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelStreamEvent {
    pub delta: String,
    pub tool_call: Option<ToolCall>,
    pub done: bool,
    pub usage: Option<Usage>,
}

#[async_trait]
pub trait ModelPort: Send + Sync {
    fn name(&self) -> &'static str;
    async fn chat(&self, request: ModelRequest) -> Result<ModelResponse>;
    async fn chat_stream(&self, request: ModelRequest) -> Result<ModelStream>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryHit {
    pub path: String,
    pub line: usize,
    pub snippet: String,
}

#[async_trait]
pub trait MemoryPort: Send + Sync {
    async fn search(&self, keyword: &str) -> Result<Vec<MemoryHit>>;
    async fn append_daily_log(&self, summary: &str) -> Result<PathBuf>;
    async fn get_file(
        &self,
        relative_path: &str,
        start_line: Option<usize>,
        end_line: Option<usize>,
    ) -> Result<String>;
    async fn read_curated(&self) -> Result<String>;
    async fn write_curated(&self, content: &str) -> Result<()>;
    async fn ensure_layout(&self) -> Result<()>;
}

#[derive(Clone)]
pub struct ToolExecutionContext {
    pub root_dir: PathBuf,
    pub memory: Arc<dyn MemoryPort>,
}

impl ToolExecutionContext {
    pub fn new(root_dir: PathBuf, memory: Arc<dyn MemoryPort>) -> Self {
        Self { root_dir, memory }
    }
}

#[async_trait]
pub trait ToolExecutorPort: Send + Sync {
    fn specs(&self) -> Vec<ToolSpec>;
    async fn execute(
        &self,
        tool_call_id: &str,
        name: &str,
        args: Value,
        context: &ToolExecutionContext,
    ) -> Result<ToolResult>;
}
