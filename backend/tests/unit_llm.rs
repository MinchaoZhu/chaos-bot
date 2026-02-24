use chaos_bot_backend::llm::*;
use chaos_bot_backend::types::*;
use serde_json::json;
use std::collections::{HashMap, VecDeque};

// -------------------------------------------------------------------------
// map_messages
// -------------------------------------------------------------------------

#[test]
fn map_messages_system() {
    let msgs = vec![Message::system("hello")];
    let mapped = OpenAiProvider::map_messages(&msgs);
    assert_eq!(mapped[0]["role"], "system");
    assert_eq!(mapped[0]["content"], "hello");
}

#[test]
fn map_messages_user() {
    let msgs = vec![Message::user("question")];
    let mapped = OpenAiProvider::map_messages(&msgs);
    assert_eq!(mapped[0]["role"], "user");
}

#[test]
fn map_messages_assistant() {
    let msgs = vec![Message::assistant("answer")];
    let mapped = OpenAiProvider::map_messages(&msgs);
    assert_eq!(mapped[0]["role"], "assistant");
}

#[test]
fn map_messages_tool() {
    let msgs = vec![Message::tool("read", "tc_1", "contents")];
    let mapped = OpenAiProvider::map_messages(&msgs);
    assert_eq!(mapped[0]["role"], "tool");
    assert_eq!(mapped[0]["tool_call_id"], "tc_1");
}

#[test]
fn map_messages_multiple() {
    let msgs = vec![
        Message::system("sys"),
        Message::user("usr"),
        Message::assistant("asst"),
    ];
    let mapped = OpenAiProvider::map_messages(&msgs);
    assert_eq!(mapped.len(), 3);
}

// -------------------------------------------------------------------------
// map_tools
// -------------------------------------------------------------------------

#[test]
fn map_tools_empty() {
    let tools: Vec<ToolSpec> = vec![];
    let mapped = OpenAiProvider::map_tools(&tools);
    assert!(mapped.is_empty());
}

#[test]
fn map_tools_single() {
    let tools = vec![ToolSpec {
        name: "read".into(),
        description: "Read file".into(),
        parameters_schema: json!({"type": "object"}),
    }];
    let mapped = OpenAiProvider::map_tools(&tools);
    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped[0]["type"], "function");
    assert_eq!(mapped[0]["function"]["name"], "read");
    assert_eq!(mapped[0]["function"]["description"], "Read file");
}

// -------------------------------------------------------------------------
// parse_usage
// -------------------------------------------------------------------------

#[test]
fn parse_usage_present() {
    let data = json!({
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 20,
            "total_tokens": 30
        }
    });
    let usage = OpenAiProvider::parse_usage(&data).unwrap();
    assert_eq!(usage.prompt_tokens, 10);
    assert_eq!(usage.completion_tokens, 20);
    assert_eq!(usage.total_tokens, 30);
}

#[test]
fn parse_usage_absent() {
    let data = json!({});
    assert!(OpenAiProvider::parse_usage(&data).is_none());
}

// -------------------------------------------------------------------------
// parse_tool_calls_from_message
// -------------------------------------------------------------------------

#[test]
fn parse_tool_calls_from_message_with_calls() {
    let msg = json!({
        "tool_calls": [{
            "id": "tc_1",
            "function": {
                "name": "read",
                "arguments": "{\"path\": \"test.txt\"}"
            }
        }]
    });
    let calls = OpenAiProvider::parse_tool_calls_from_message(&msg);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "tc_1");
    assert_eq!(calls[0].name, "read");
    assert_eq!(calls[0].arguments["path"], "test.txt");
}

#[test]
fn parse_tool_calls_from_message_no_calls() {
    let msg = json!({"content": "hello"});
    let calls = OpenAiProvider::parse_tool_calls_from_message(&msg);
    assert!(calls.is_empty());
}

#[test]
fn parse_tool_calls_invalid_json_arguments() {
    let msg = json!({
        "tool_calls": [{
            "id": "tc_1",
            "function": {
                "name": "read",
                "arguments": "not json"
            }
        }]
    });
    let calls = OpenAiProvider::parse_tool_calls_from_message(&msg);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].arguments["raw"], "not json");
}

// -------------------------------------------------------------------------
// drain_sse_payloads
// -------------------------------------------------------------------------

#[test]
fn drain_sse_payloads_single() {
    let mut buffer = "data: {\"hello\":\"world\"}\n\n".to_string();
    let payloads = OpenAiProvider::drain_sse_payloads(&mut buffer);
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0], "{\"hello\":\"world\"}");
    assert!(buffer.is_empty());
}

#[test]
fn drain_sse_payloads_multiple() {
    let mut buffer = "data: first\n\ndata: second\n\n".to_string();
    let payloads = OpenAiProvider::drain_sse_payloads(&mut buffer);
    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0], "first");
    assert_eq!(payloads[1], "second");
}

#[test]
fn drain_sse_payloads_partial() {
    let mut buffer = "data: complete\n\ndata: incomp".to_string();
    let payloads = OpenAiProvider::drain_sse_payloads(&mut buffer);
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0], "complete");
    assert_eq!(buffer, "data: incomp");
}

#[test]
fn drain_sse_payloads_done_marker() {
    let mut buffer = "data: [DONE]\n\n".to_string();
    let payloads = OpenAiProvider::drain_sse_payloads(&mut buffer);
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0], "[DONE]");
}

#[test]
fn drain_sse_payloads_ignores_non_data_lines() {
    let mut buffer = "event: something\ndata: payload\n\n".to_string();
    let payloads = OpenAiProvider::drain_sse_payloads(&mut buffer);
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0], "payload");
}

#[test]
fn drain_sse_payloads_empty_buffer() {
    let mut buffer = String::new();
    let payloads = OpenAiProvider::drain_sse_payloads(&mut buffer);
    assert!(payloads.is_empty());
}

// -------------------------------------------------------------------------
// process_stream_payload
// -------------------------------------------------------------------------

fn empty_stream_state() -> OpenAiStreamState {
    OpenAiStreamState {
        stream: Box::pin(futures::stream::empty()),
        text_buffer: String::new(),
        pending: VecDeque::new(),
        tool_ids: HashMap::new(),
        tool_names: HashMap::new(),
        tool_args: HashMap::new(),
        usage: None,
        done: false,
        emitted_done: false,
    }
}

#[test]
fn process_done_marker() {
    let mut state = empty_stream_state();
    OpenAiProvider::process_stream_payload(&mut state, "[DONE]").unwrap();
    assert!(state.done);
    assert_eq!(state.pending.len(), 1);
    let event = state.pending.pop_front().unwrap().unwrap();
    assert!(event.done);
}

#[test]
fn process_done_marker_only_once() {
    let mut state = empty_stream_state();
    OpenAiProvider::process_stream_payload(&mut state, "[DONE]").unwrap();
    OpenAiProvider::process_stream_payload(&mut state, "[DONE]").unwrap();
    // Only one done event should be emitted
    assert_eq!(state.pending.len(), 1);
}

#[test]
fn process_text_delta() {
    let mut state = empty_stream_state();
    let payload = json!({
        "choices": [{
            "delta": {"content": "hello"},
            "index": 0
        }]
    })
    .to_string();

    OpenAiProvider::process_stream_payload(&mut state, &payload).unwrap();
    assert_eq!(state.pending.len(), 1);
    let event = state.pending.pop_front().unwrap().unwrap();
    assert_eq!(event.delta, "hello");
    assert!(!event.done);
}

#[test]
fn process_empty_content_not_emitted() {
    let mut state = empty_stream_state();
    let payload = json!({
        "choices": [{
            "delta": {"content": ""},
            "index": 0
        }]
    })
    .to_string();

    OpenAiProvider::process_stream_payload(&mut state, &payload).unwrap();
    assert!(state.pending.is_empty());
}

#[test]
fn process_tool_call_assembly() {
    let mut state = empty_stream_state();

    // First chunk: tool call start
    let p1 = json!({
        "choices": [{
            "delta": {
                "tool_calls": [{
                    "index": 0,
                    "id": "tc_1",
                    "function": {"name": "read", "arguments": "{\"pa"}
                }]
            }
        }]
    })
    .to_string();
    OpenAiProvider::process_stream_payload(&mut state, &p1).unwrap();
    assert_eq!(state.tool_ids.get(&0).unwrap(), "tc_1");
    assert_eq!(state.tool_names.get(&0).unwrap(), "read");

    // Second chunk: argument continuation
    let p2 = json!({
        "choices": [{
            "delta": {
                "tool_calls": [{
                    "index": 0,
                    "function": {"arguments": "th\": \"test.txt\"}"}
                }]
            }
        }]
    })
    .to_string();
    OpenAiProvider::process_stream_payload(&mut state, &p2).unwrap();

    // Finish with tool_calls reason
    let p3 = json!({
        "choices": [{
            "delta": {},
            "finish_reason": "tool_calls"
        }]
    })
    .to_string();
    OpenAiProvider::process_stream_payload(&mut state, &p3).unwrap();

    // Should have emitted a tool call event
    assert_eq!(state.pending.len(), 1);
    let event = state.pending.pop_front().unwrap().unwrap();
    let tc = event.tool_call.unwrap();
    assert_eq!(tc.id, "tc_1");
    assert_eq!(tc.name, "read");
    assert_eq!(tc.arguments["path"], "test.txt");
}

#[test]
fn process_usage_captured() {
    let mut state = empty_stream_state();
    let payload = json!({
        "usage": {
            "prompt_tokens": 5,
            "completion_tokens": 10,
            "total_tokens": 15
        },
        "choices": []
    })
    .to_string();

    OpenAiProvider::process_stream_payload(&mut state, &payload).unwrap();
    assert!(state.usage.is_some());
    assert_eq!(state.usage.unwrap().total_tokens, 15);
}

#[test]
fn process_invalid_json_errors() {
    let mut state = empty_stream_state();
    let result = OpenAiProvider::process_stream_payload(&mut state, "not json");
    assert!(result.is_err());
}

// -------------------------------------------------------------------------
// flush_tool_calls
// -------------------------------------------------------------------------

#[test]
fn flush_tool_calls_multiple_indexes() {
    let mut state = empty_stream_state();
    state.tool_ids.insert(0, "tc_0".into());
    state.tool_names.insert(0, "read".into());
    state.tool_args.insert(0, "{}".into());
    state.tool_ids.insert(1, "tc_1".into());
    state.tool_names.insert(1, "write".into());
    state.tool_args.insert(1, "{\"path\": \"out.txt\"}".into());

    OpenAiProvider::flush_tool_calls(&mut state);

    assert_eq!(state.pending.len(), 2);
    let e0 = state.pending.pop_front().unwrap().unwrap();
    let e1 = state.pending.pop_front().unwrap().unwrap();
    assert_eq!(e0.tool_call.unwrap().name, "read");
    assert_eq!(e1.tool_call.unwrap().name, "write");
}

#[test]
fn flush_tool_calls_invalid_json_args_wrapped() {
    let mut state = empty_stream_state();
    state.tool_ids.insert(0, "tc_0".into());
    state.tool_names.insert(0, "test".into());
    state.tool_args.insert(0, "not json".into());

    OpenAiProvider::flush_tool_calls(&mut state);

    let event = state.pending.pop_front().unwrap().unwrap();
    let tc = event.tool_call.unwrap();
    assert_eq!(tc.arguments["raw"], "not json");
}

// -------------------------------------------------------------------------
// build_provider
// -------------------------------------------------------------------------

#[test]
fn build_provider_mock() {
    use chaos_bot_backend::config::AppConfig;
    let config = AppConfig {
        host: "0.0.0.0".into(),
        port: 3000,
        provider: "mock".into(),
        model: "mock".into(),
        openai_api_key: None,
        anthropic_api_key: None,
        gemini_api_key: None,
        temperature: 0.0,
        max_tokens: 128,
        max_iterations: 1,
        token_budget: 1024,
        working_dir: std::path::PathBuf::from("."),
        personality_dir: std::path::PathBuf::from("."),
        memory_dir: std::path::PathBuf::from("."),
        memory_file: std::path::PathBuf::from("."),
    };
    let provider = build_provider(&config).unwrap();
    assert_eq!(provider.name(), "mock");
}

#[test]
fn build_provider_unsupported() {
    use chaos_bot_backend::config::AppConfig;
    let config = AppConfig {
        host: "0.0.0.0".into(),
        port: 3000,
        provider: "unknown".into(),
        model: "x".into(),
        openai_api_key: None,
        anthropic_api_key: None,
        gemini_api_key: None,
        temperature: 0.0,
        max_tokens: 128,
        max_iterations: 1,
        token_budget: 1024,
        working_dir: std::path::PathBuf::from("."),
        personality_dir: std::path::PathBuf::from("."),
        memory_dir: std::path::PathBuf::from("."),
        memory_file: std::path::PathBuf::from("."),
    };
    assert!(build_provider(&config).is_err());
}

#[test]
fn build_provider_openai_without_key_errors() {
    use chaos_bot_backend::config::AppConfig;
    let config = AppConfig {
        host: "0.0.0.0".into(),
        port: 3000,
        provider: "openai".into(),
        model: "gpt-4o".into(),
        openai_api_key: None,
        anthropic_api_key: None,
        gemini_api_key: None,
        temperature: 0.0,
        max_tokens: 128,
        max_iterations: 1,
        token_budget: 1024,
        working_dir: std::path::PathBuf::from("."),
        personality_dir: std::path::PathBuf::from("."),
        memory_dir: std::path::PathBuf::from("."),
        memory_file: std::path::PathBuf::from("."),
    };
    assert!(build_provider(&config).is_err());
}

#[test]
fn build_provider_anthropic_without_key_errors() {
    use chaos_bot_backend::config::AppConfig;
    let config = AppConfig {
        host: "0.0.0.0".into(),
        port: 3000,
        provider: "anthropic".into(),
        model: "claude".into(),
        openai_api_key: None,
        anthropic_api_key: None,
        gemini_api_key: None,
        temperature: 0.0,
        max_tokens: 128,
        max_iterations: 1,
        token_budget: 1024,
        working_dir: std::path::PathBuf::from("."),
        personality_dir: std::path::PathBuf::from("."),
        memory_dir: std::path::PathBuf::from("."),
        memory_file: std::path::PathBuf::from("."),
    };
    assert!(build_provider(&config).is_err());
}

#[test]
fn build_provider_gemini_without_key_errors() {
    use chaos_bot_backend::config::AppConfig;
    let config = AppConfig {
        host: "0.0.0.0".into(),
        port: 3000,
        provider: "gemini".into(),
        model: "gemini-pro".into(),
        openai_api_key: None,
        anthropic_api_key: None,
        gemini_api_key: None,
        temperature: 0.0,
        max_tokens: 128,
        max_iterations: 1,
        token_budget: 1024,
        working_dir: std::path::PathBuf::from("."),
        personality_dir: std::path::PathBuf::from("."),
        memory_dir: std::path::PathBuf::from("."),
        memory_file: std::path::PathBuf::from("."),
    };
    assert!(build_provider(&config).is_err());
}

// -------------------------------------------------------------------------
// MockProvider (built-in)
// -------------------------------------------------------------------------

#[tokio::test]
async fn mock_provider_chat_stream_text() {
    use futures::StreamExt;

    let config = chaos_bot_backend::config::AppConfig {
        host: "0.0.0.0".into(),
        port: 3000,
        provider: "mock".into(),
        model: "mock".into(),
        openai_api_key: None,
        anthropic_api_key: None,
        gemini_api_key: None,
        temperature: 0.0,
        max_tokens: 128,
        max_iterations: 1,
        token_budget: 1024,
        working_dir: std::path::PathBuf::from("."),
        personality_dir: std::path::PathBuf::from("."),
        memory_dir: std::path::PathBuf::from("."),
        memory_file: std::path::PathBuf::from("."),
    };
    let provider = build_provider(&config).unwrap();

    let request = LlmRequest {
        model: "mock".into(),
        messages: vec![Message::user("hello")],
        tools: vec![],
        temperature: 0.0,
        max_tokens: 128,
    };

    let mut stream = provider.chat_stream(request).await.unwrap();
    let mut text = String::new();
    let mut got_done = false;
    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        text.push_str(&event.delta);
        if event.done {
            got_done = true;
        }
    }

    assert!(text.contains("Mock response to: hello"));
    assert!(got_done);
}

#[tokio::test]
async fn mock_provider_chat_stream_tool_call() {
    use futures::StreamExt;

    let config = chaos_bot_backend::config::AppConfig {
        host: "0.0.0.0".into(),
        port: 3000,
        provider: "mock".into(),
        model: "mock".into(),
        openai_api_key: None,
        anthropic_api_key: None,
        gemini_api_key: None,
        temperature: 0.0,
        max_tokens: 128,
        max_iterations: 1,
        token_budget: 1024,
        working_dir: std::path::PathBuf::from("."),
        personality_dir: std::path::PathBuf::from("."),
        memory_dir: std::path::PathBuf::from("."),
        memory_file: std::path::PathBuf::from("."),
    };
    let provider = build_provider(&config).unwrap();

    let request = LlmRequest {
        model: "mock".into(),
        messages: vec![Message::user("use_tool: read")],
        tools: vec![],
        temperature: 0.0,
        max_tokens: 128,
    };

    let mut stream = provider.chat_stream(request).await.unwrap();
    let mut got_tool = false;
    while let Some(event) = stream.next().await {
        let event = event.unwrap();
        if let Some(tc) = event.tool_call {
            assert_eq!(tc.name, "read");
            got_tool = true;
        }
    }
    assert!(got_tool);
}
