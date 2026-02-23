mod support;

use chaos_bot_backend::agent::AgentLoop;
use chaos_bot_backend::llm::LlmStreamEvent;
use chaos_bot_backend::memory::MemoryHit;
use chaos_bot_backend::types::{Message, SessionState, ToolCall};
use serde_json::json;
use std::sync::Arc;
use support::*;

// -------------------------------------------------------------------------
// build_system_prompt
// -------------------------------------------------------------------------

#[test]
fn build_system_prompt_without_memory() {
    let prompt = AgentLoop::build_system_prompt("You are helpful.", &[]);
    assert_eq!(prompt, "You are helpful.");
    assert!(!prompt.contains("Memory Context"));
}

#[test]
fn build_system_prompt_with_memory() {
    let hits = vec![
        MemoryHit {
            path: "MEMORY.md".into(),
            line: 1,
            snippet: "fact: important".into(),
        },
        MemoryHit {
            path: "2024-01-01.md".into(),
            line: 5,
            snippet: "log entry".into(),
        },
    ];
    let prompt = AgentLoop::build_system_prompt("Base prompt.", &hits);
    assert!(prompt.contains("Base prompt."));
    assert!(prompt.contains("Relevant Memory Context"));
    assert!(prompt.contains("fact: important"));
    assert!(prompt.contains("log entry"));
}

#[test]
fn build_system_prompt_limits_to_6_hits() {
    let hits: Vec<MemoryHit> = (0..10)
        .map(|i| MemoryHit {
            path: format!("file{i}.md"),
            line: i,
            snippet: format!("hit {i}"),
        })
        .collect();
    let prompt = AgentLoop::build_system_prompt("Base.", &hits);
    // Should only contain first 6 hits
    assert!(prompt.contains("hit 0"));
    assert!(prompt.contains("hit 5"));
    assert!(!prompt.contains("hit 6"));
}

#[test]
fn build_system_prompt_trims_personality() {
    let prompt = AgentLoop::build_system_prompt("  padded  \n\n", &[]);
    assert_eq!(prompt, "padded");
}

// -------------------------------------------------------------------------
// enforce_token_budget
// -------------------------------------------------------------------------

#[test]
fn enforce_token_budget_removes_middle_messages() {
    let mut messages = vec![
        Message::system("s".repeat(100)),
        Message::user("u1"),
        Message::assistant("a1".repeat(200)),
        Message::user("u2"),
    ];
    // With a small budget, middle messages should be removed
    AgentLoop::enforce_token_budget(&mut messages, 80);
    // Should keep at least system + one other
    assert!(messages.len() >= 2);
    // System should always be first
    assert_eq!(messages[0].role, chaos_bot_backend::types::Role::System);
}

#[test]
fn enforce_token_budget_keeps_at_least_two() {
    let mut messages = vec![
        Message::system("sys"),
        Message::user("usr"),
    ];
    AgentLoop::enforce_token_budget(&mut messages, 1);
    assert_eq!(messages.len(), 2);
}

#[test]
fn enforce_token_budget_no_removal_when_under() {
    let mut messages = vec![
        Message::system("sys"),
        Message::user("usr"),
    ];
    AgentLoop::enforce_token_budget(&mut messages, 100_000);
    assert_eq!(messages.len(), 2);
}

// -------------------------------------------------------------------------
// estimate_tokens
// -------------------------------------------------------------------------

#[test]
fn estimate_tokens_basic() {
    let messages = vec![
        Message::system("hello world"), // 11 chars -> 11/4 + 8 = 10
    ];
    let estimate = AgentLoop::estimate_tokens(&messages);
    assert_eq!(estimate, (11 / 4 + 8) as u32);
}

#[test]
fn estimate_tokens_multiple() {
    let messages = vec![
        Message::system("a"),   // 1/4 + 8 = 8
        Message::user("b"),     // 1/4 + 8 = 8
    ];
    let estimate = AgentLoop::estimate_tokens(&messages);
    assert_eq!(estimate, 16);
}

// -------------------------------------------------------------------------
// run with mock (no tools → stop)
// -------------------------------------------------------------------------

#[tokio::test]
async fn run_no_tools_returns_stop() {
    let provider = MockStreamProvider::text("Hello!");
    let (_temp, agent) = build_test_agent(Arc::new(provider));

    let mut session = SessionState::new("s1");
    let output = agent.run(&mut session, "hi".to_string()).await.unwrap();

    assert_eq!(output.assistant_message.content, "Hello!");
    assert_eq!(output.finish_reason.as_deref(), Some("stop"));
    assert!(output.tool_events.is_empty());
    // Session should have user + assistant messages
    assert_eq!(session.messages.len(), 2);
}

// -------------------------------------------------------------------------
// run with tool calls → loop
// -------------------------------------------------------------------------

#[tokio::test]
async fn run_with_tool_calls_loops() {
    let tool_call = ToolCall {
        id: "tc_1".to_string(),
        name: "mock_tool".to_string(),
        arguments: json!({}),
    };
    let provider = MockStreamProvider::tool_then_text(tool_call, "Done!");
    let mut registry = chaos_bot_backend::tools::ToolRegistry::new();
    registry.register(MockTool::fixed("mock_tool", "tool output"));

    let (_temp, agent) = build_test_agent_with_registry(Arc::new(provider), registry);

    let mut session = SessionState::new("s1");
    let output = agent.run(&mut session, "do something".to_string()).await.unwrap();

    assert_eq!(output.assistant_message.content, "Done!");
    assert_eq!(output.finish_reason.as_deref(), Some("stop"));
    assert_eq!(output.tool_events.len(), 1);
    assert_eq!(output.tool_events[0].call.name, "mock_tool");
}

// -------------------------------------------------------------------------
// run_stream delivers events
// -------------------------------------------------------------------------

#[tokio::test]
async fn run_stream_delivers_delta_events() {
    let provider = MockStreamProvider::new(vec![vec![
        LlmStreamEvent {
            delta: "chunk1".to_string(),
            tool_call: None,
            done: false,
            usage: None,
        },
        LlmStreamEvent {
            delta: "chunk2".to_string(),
            tool_call: None,
            done: false,
            usage: None,
        },
        LlmStreamEvent {
            delta: String::new(),
            tool_call: None,
            done: true,
            usage: None,
        },
    ]]);

    let (_temp, agent) = build_test_agent(Arc::new(provider));
    let mut session = SessionState::new("s1");

    let mut deltas = Vec::new();
    agent
        .run_stream(&mut session, "hi".to_string(), |event| {
            if let chaos_bot_backend::agent::AgentStreamEvent::Delta(d) = event {
                deltas.push(d);
            }
        })
        .await
        .unwrap();

    assert_eq!(deltas, vec!["chunk1", "chunk2"]);
}

// -------------------------------------------------------------------------
// max iterations → fallback
// -------------------------------------------------------------------------

#[tokio::test]
async fn max_iterations_returns_fallback() {
    // Provider that always returns a tool call (never stops)
    let tool_call = || ToolCall {
        id: "tc_1".to_string(),
        name: "mock_tool".to_string(),
        arguments: json!({}),
    };

    let provider = MockStreamProvider::new(vec![
        vec![
            LlmStreamEvent { delta: String::new(), tool_call: Some(tool_call()), done: false, usage: None },
            LlmStreamEvent { delta: String::new(), tool_call: None, done: true, usage: None },
        ],
        vec![
            LlmStreamEvent { delta: String::new(), tool_call: Some(tool_call()), done: false, usage: None },
            LlmStreamEvent { delta: String::new(), tool_call: None, done: true, usage: None },
        ],
        vec![
            LlmStreamEvent { delta: String::new(), tool_call: Some(tool_call()), done: false, usage: None },
            LlmStreamEvent { delta: String::new(), tool_call: None, done: true, usage: None },
        ],
        vec![
            LlmStreamEvent { delta: String::new(), tool_call: Some(tool_call()), done: false, usage: None },
            LlmStreamEvent { delta: String::new(), tool_call: None, done: true, usage: None },
        ],
        vec![
            LlmStreamEvent { delta: String::new(), tool_call: Some(tool_call()), done: false, usage: None },
            LlmStreamEvent { delta: String::new(), tool_call: None, done: true, usage: None },
        ],
        vec![
            LlmStreamEvent { delta: String::new(), tool_call: Some(tool_call()), done: false, usage: None },
            LlmStreamEvent { delta: String::new(), tool_call: None, done: true, usage: None },
        ],
    ]);

    let mut registry = chaos_bot_backend::tools::ToolRegistry::new();
    registry.register(MockTool::fixed("mock_tool", "ok"));

    let (_temp, agent) = build_test_agent_with_registry(Arc::new(provider), registry);
    let mut session = SessionState::new("s1");

    let output = agent.run(&mut session, "loop forever".to_string()).await.unwrap();
    assert!(output.assistant_message.content.contains("max iterations"));
}

// -------------------------------------------------------------------------
// run with error from provider
// -------------------------------------------------------------------------

#[tokio::test]
async fn run_provider_error_propagates() {
    let provider = ErrorProvider::new("API key invalid");
    let (_temp, agent) = build_test_agent(Arc::new(provider));
    let mut session = SessionState::new("s1");

    let result = agent.run(&mut session, "hi".to_string()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("API key invalid"));
}
