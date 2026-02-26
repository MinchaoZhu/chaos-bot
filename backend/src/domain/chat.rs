use crate::domain::types::{ToolCall, ToolResult, Usage};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct ChatCommand {
    pub session_id: Option<String>,
    pub message: String,
    pub channel: Option<ChannelContext>,
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
    pub assistant_message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelContext {
    pub channel: String,
    pub user_id: String,
    pub conversation_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InboundChannelMessage {
    pub channel: String,
    pub user_id: String,
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub text: String,
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutboundChannelMessage {
    pub channel: String,
    pub user_id: String,
    pub conversation_id: String,
    pub session_id: String,
    pub text: String,
    pub metadata: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelDelivery {
    pub channel: String,
    pub external_message_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelHealth {
    pub channel: String,
    pub status: String,
    pub detail: Value,
}
