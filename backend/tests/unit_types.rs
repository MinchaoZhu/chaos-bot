use chaos_bot_backend::types::*;
use pretty_assertions::assert_eq;

#[test]
fn message_system_constructor() {
    let msg = Message::system("hello");
    assert_eq!(msg.role, Role::System);
    assert_eq!(msg.content, "hello");
    assert!(msg.name.is_none());
    assert!(msg.tool_call_id.is_none());
}

#[test]
fn message_user_constructor() {
    let msg = Message::user("question");
    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.content, "question");
}

#[test]
fn message_assistant_constructor() {
    let msg = Message::assistant("answer");
    assert_eq!(msg.role, Role::Assistant);
    assert_eq!(msg.content, "answer");
}

#[test]
fn message_tool_constructor() {
    let msg = Message::tool("read", "tc_1", "file contents");
    assert_eq!(msg.role, Role::Tool);
    assert_eq!(msg.content, "file contents");
    assert_eq!(msg.name.as_deref(), Some("read"));
    assert_eq!(msg.tool_call_id.as_deref(), Some("tc_1"));
}

#[test]
fn message_serde_roundtrip() {
    let msg = Message::user("test");
    let json = serde_json::to_string(&msg).unwrap();
    let back: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(back.role, Role::User);
    assert_eq!(back.content, "test");
}

#[test]
fn message_tool_serde_roundtrip() {
    let msg = Message::tool("grep", "tc_42", "result");
    let json = serde_json::to_string(&msg).unwrap();
    let back: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(back.role, Role::Tool);
    assert_eq!(back.name.as_deref(), Some("grep"));
    assert_eq!(back.tool_call_id.as_deref(), Some("tc_42"));
}

#[test]
fn role_serde_lowercase() {
    let json = serde_json::to_string(&Role::System).unwrap();
    assert_eq!(json, "\"system\"");
    let json = serde_json::to_string(&Role::User).unwrap();
    assert_eq!(json, "\"user\"");
    let json = serde_json::to_string(&Role::Assistant).unwrap();
    assert_eq!(json, "\"assistant\"");
    let json = serde_json::to_string(&Role::Tool).unwrap();
    assert_eq!(json, "\"tool\"");
}

#[test]
fn tool_spec_serde() {
    let spec = ToolSpec {
        name: "read".into(),
        description: "Read a file".into(),
        parameters_schema: serde_json::json!({"type": "object"}),
    };
    let json = serde_json::to_string(&spec).unwrap();
    let back: ToolSpec = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, "read");
}

#[test]
fn tool_call_serde() {
    let call = ToolCall {
        id: "tc_1".into(),
        name: "read".into(),
        arguments: serde_json::json!({"path": "foo.txt"}),
    };
    let json = serde_json::to_string(&call).unwrap();
    let back: ToolCall = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, "tc_1");
    assert_eq!(back.arguments["path"], "foo.txt");
}

#[test]
fn tool_execution_serde() {
    let exec = ToolExecution {
        name: "read".into(),
        output: "contents".into(),
        is_error: false,
    };
    let json = serde_json::to_string(&exec).unwrap();
    let back: ToolExecution = serde_json::from_str(&json).unwrap();
    assert!(!back.is_error);
}

#[test]
fn tool_result_serde() {
    let result = ToolResult {
        tool_call_id: "tc_1".into(),
        name: "read".into(),
        output: "ok".into(),
        is_error: false,
    };
    let json = serde_json::to_string(&result).unwrap();
    let back: ToolResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.tool_call_id, "tc_1");
}

#[test]
fn usage_serde() {
    let u = Usage {
        prompt_tokens: 10,
        completion_tokens: 20,
        total_tokens: 30,
    };
    let json = serde_json::to_string(&u).unwrap();
    let back: Usage = serde_json::from_str(&json).unwrap();
    assert_eq!(back.total_tokens, 30);
}

#[test]
fn session_state_new() {
    let s = SessionState::new("s1");
    assert_eq!(s.id, "s1");
    assert!(s.messages.is_empty());
    assert!(s.created_at <= s.updated_at);
}

#[test]
fn session_state_push_message_updates_timestamp() {
    let mut s = SessionState::new("s1");
    let before = s.updated_at;
    // Small sleep to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(2));
    s.push_message(Message::user("hello"));
    assert_eq!(s.messages.len(), 1);
    assert!(s.updated_at >= before);
}

#[test]
fn session_state_serde_roundtrip() {
    let mut s = SessionState::new("s1");
    s.push_message(Message::user("hi"));
    s.push_message(Message::assistant("hello"));

    let json = serde_json::to_string(&s).unwrap();
    let back: SessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, "s1");
    assert_eq!(back.messages.len(), 2);
}

#[test]
fn message_system_accepts_string() {
    let msg = Message::system(String::from("owned"));
    assert_eq!(msg.content, "owned");
}

#[test]
fn message_skip_serializing_none_fields() {
    let msg = Message::user("hi");
    let json = serde_json::to_string(&msg).unwrap();
    assert!(!json.contains("name"));
    assert!(!json.contains("tool_call_id"));
}
