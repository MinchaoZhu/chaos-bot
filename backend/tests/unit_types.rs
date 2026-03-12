use chaos_bot_backend::domain::types::*;
use pretty_assertions::assert_eq;

#[test]
fn message_constructors_and_roundtrip() {
    let system = Message::system("hello");
    let user = Message::user("question");
    let assistant = Message::assistant("answer");
    let tool = Message::tool("read", "tc_1", "file contents");

    assert_eq!(system.role, Role::System);
    assert_eq!(user.role, Role::User);
    assert_eq!(assistant.role, Role::Assistant);
    assert_eq!(tool.role, Role::Tool);

    let json = serde_json::to_string(&tool).unwrap();
    let back: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name.as_deref(), Some("read"));
    assert_eq!(back.tool_call_id.as_deref(), Some("tc_1"));
}

#[test]
fn role_serializes_lowercase() {
    assert_eq!(serde_json::to_string(&Role::System).unwrap(), "\"system\"");
    assert_eq!(serde_json::to_string(&Role::User).unwrap(), "\"user\"");
    assert_eq!(
        serde_json::to_string(&Role::Assistant).unwrap(),
        "\"assistant\""
    );
    assert_eq!(serde_json::to_string(&Role::Tool).unwrap(), "\"tool\"");
}

#[test]
fn tool_types_roundtrip() {
    let spec = ToolSpec {
        name: "read".into(),
        description: "Read a file".into(),
        parameters_schema: serde_json::json!({"type": "object"}),
    };
    let call = ToolCall {
        id: "tc_1".into(),
        name: "read".into(),
        arguments: serde_json::json!({"path": "foo.txt"}),
    };
    let result = ToolResult {
        tool_call_id: "tc_1".into(),
        name: "read".into(),
        output: "ok".into(),
        is_error: false,
    };

    let spec_back: ToolSpec = serde_json::from_str(&serde_json::to_string(&spec).unwrap()).unwrap();
    let call_back: ToolCall = serde_json::from_str(&serde_json::to_string(&call).unwrap()).unwrap();
    let result_back: ToolResult =
        serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();

    assert_eq!(spec_back.name, "read");
    assert_eq!(call_back.arguments["path"], "foo.txt");
    assert_eq!(result_back.tool_call_id, "tc_1");
}

#[test]
fn session_state_updates_timestamp() {
    let mut session = SessionState::new("s1");
    let before = session.updated_at;
    std::thread::sleep(std::time::Duration::from_millis(2));
    session.push_message(Message::user("hi"));
    session.push_message(Message::assistant("hello"));

    let json = serde_json::to_string(&session).unwrap();
    let back: SessionState = serde_json::from_str(&json).unwrap();

    assert_eq!(back.id, "s1");
    assert_eq!(back.messages.len(), 2);
    assert!(back.updated_at >= before);
}
