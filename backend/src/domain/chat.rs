use crate::domain::types::{ToolCall, ToolResult, Usage};
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct ChatCommand {
    pub session_id: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum ChatEvent {
    Session { session_id: String },
    Delta(String),
    Tool(ToolEvent),
}

#[derive(Clone, Debug, Serialize)]
pub struct ToolEvent {
    pub call: ToolCall,
    pub result: ToolResult,
}

#[derive(Clone, Debug)]
pub struct ChatResult {
    pub session_id: String,
    pub usage: Option<Usage>,
    pub finish_reason: Option<String>,
}
