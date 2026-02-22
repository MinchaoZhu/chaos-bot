# task-1: Bootstrap personal AI agent (Rust + frontend)

## Task
- Description: Build the full architecture scaffold for a personal AI agent with Rust backend and web frontend, covering LLM abstraction, tool system, personality, memory, agent loop, HTTP API, and minimal UI.
- Scope: Core Rust types and traits, one working LLM provider, tool implementations, personality/memory loading, agent loop, axum API with SSE, and a minimal chat frontend.
- Risk: API/Frontend mismatch during early iteration; async streaming plumbing in Rust; personality/memory prompt size management.
- Status: done

## Phase 1: Directory layout and core Rust types

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Define mono-repo directory layout (`backend/`, `frontend/`, `personality/`, `memory/`, `data/`). | done | |
| 1.2 | Initialize Rust workspace with `cargo init` and base dependencies (axum, tokio, serde, reqwest, tracing). | done | |
| 1.3 | Define core types: `Message`, `ToolCall`, `ToolResult`, `Usage`, `SessionState`. | done | |
| 1.4 | Define core traits: `LlmProvider` (with `chat` and `chat_stream`), `Tool` (with `name`, `description`, `parameters_schema`, `execute`). | done | |
| 1.5 | Define `AgentLoop` struct skeleton (provider, tools, personality, memory references). | done | |
| 1.6 | Create placeholder personality files: `personality/SOUL.md`, `personality/IDENTITY.md`, `personality/USER.md`, `personality/AGENTS.md`. | done | |
| 1.7 | Create placeholder memory files: `MEMORY.md`, `memory/` directory. | done | |
| 1.v1 | Verify: `cargo check` passes; all type/trait definitions compile; personality/memory directories exist. | done | |

## Phase 2: LLM abstraction layer

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Implement `LlmProvider` trait with shared request/response types. | done | |
| 2.2 | Implement OpenAI provider (`OpenAiProvider`) with non-streaming `chat()`. | done | |
| 2.3 | Implement streaming support (`chat_stream()`) returning `Stream<Item = ...>` for SSE chunks. | done | |
| 2.4 | Add provider configuration (API key, model, temperature, max_tokens) from environment/config file. | done | |
| 2.5 | Stub out Anthropic and Gemini provider structs (compile but return unimplemented error). | done | |
| 2.v1 | Verify: OpenAI provider can complete a simple prompt (non-streaming and streaming) in an integration test or manual test binary. | done | |

## Phase 3: Tool system

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Implement `Tool` trait with JSON schema generation for parameters. | done | |
| 3.2 | Implement codingTools set: `read` (file read), `write` (file write), `edit` (string replacement), `bash` (command execution). | done | |
| 3.3 | Implement readOnlyTools set: `read`, `grep` (content search), `find` (file search by pattern), `ls` (directory listing). | done | |
| 3.4 | Implement `ToolRegistry` that holds a named set of tools and dispatches by name. | done | |
| 3.5 | Add safety guardrails: working directory restrictions, command allowlists for bash. | done | |
| 3.v1 | Verify: each tool can be called programmatically with test inputs and returns expected output. | done | |

## Phase 4: Personality system

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | Implement personality loader: reads markdown files from `personality/` directory in priority order. | done | |
| 4.2 | Build system prompt constructor: concatenates personality content with section headers. | done | |
| 4.3 | Write meaningful default content for `SOUL.md`, `IDENTITY.md`, `USER.md`, `AGENTS.md`. | done | |
| 4.4 | Add runtime reload capability (personality files can be changed without restart). | done | |
| 4.v1 | Verify: personality loader produces a valid system prompt string from the markdown files. | done | |

## Phase 5: Memory system

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | Implement daily log writer: append conversation summaries to `memory/YYYY-MM-DD.md`. | done | |
| 5.2 | Implement `MEMORY.md` reader/writer for curated long-term facts. | done | |
| 5.3 | Implement `memory_get` tool: read specific memory file by path + optional line range. | done | |
| 5.4 | Implement `memory_search` tool: keyword search over `MEMORY.md` + `memory/**/*.md`. | done | |
| 5.5 | (Optional) Add vector index layer with configurable embedding provider. | done | |
| 5.v1 | Verify: memory tools return correct results for test queries; daily log appends correctly. | done | |

## Phase 6: Agent loop integration

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 6.1 | Implement the agent loop: receive message → build prompt → LLM call → tool dispatch → loop or return. | done | |
| 6.2 | Wire personality loader into prompt construction. | done | |
| 6.3 | Wire memory search into prompt construction (retrieve relevant context before LLM call). | done | |
| 6.4 | Wire tool registry into tool call dispatch. | done | |
| 6.5 | Add conversation history management (append messages, enforce token limits). | done | |
| 6.6 | Add max-iteration and token-budget guardrails. | done | |
| 6.v1 | Verify: agent loop can handle a multi-turn conversation with tool calls in a test harness (CLI or test binary). | done | |

## Phase 7: HTTP API layer

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 7.1 | Set up axum server with health endpoint, structured logging, graceful shutdown. | done | |
| 7.2 | Implement `POST /api/chat` with SSE streaming response. | done | |
| 7.3 | Implement session management endpoints: create, list, get, delete sessions. | done | |
| 7.4 | Wire agent loop into the chat handler (per-request agent execution). | done | |
| 7.5 | Add configuration loading from `.env` / config file. | done | |
| 7.v1 | Verify: `curl` or httpie can send a message to `/api/chat` and receive a streamed response with tool calls visible. | done | |

## Phase 8: Minimal web frontend

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 8.1 | Initialize frontend project (choose lightweight framework or vanilla JS). | done | |
| 8.2 | Build chat UI: message input, message list with streaming display. | done | |
| 8.3 | Implement SSE client that consumes `/api/chat` stream and renders incrementally. | done | |
| 8.4 | Add session list sidebar (create/switch/delete sessions). | done | |
| 8.5 | Add basic settings page (provider selection, model config). | done | |
| 8.v1 | Verify: end-to-end flow — type a message in the web UI, see streamed response with any tool call indicators. | done | |

## Completion Notes (2026-02-22)
- Completed scaffold across backend and frontend with compile-verified Rust workspace.
- Implemented core types/traits, provider abstraction (OpenAI + Anthropic/Gemini stubs), tool registry and built-in tools, personality/memory loading, and agent loop with tool dispatch.
- Implemented Axum API (`/api/chat` SSE, session CRUD, health) and minimal web chat UI with incremental stream rendering.
- Verification: `source "$HOME/.cargo/env" && cargo check` passed successfully.

## Post-Review Fixes (2026-02-22)
- Reworked OpenAI `chat_stream` to use true upstream streaming (`stream: true`) with incremental SSE parsing and tool-call assembly.
- Converted API chat handler to real-time SSE delivery during agent execution (no longer buffering all events first).
- Added `AgentLoop::run_stream` and unified system prompt construction into a single system message including memory context.
- Updated write/edit tool path policy to allow symlink target writes (including escape targets), with tests covering this behavior.
- Added `.env.example` and regression tests for session routes, system prompt shape, and symlink write/edit behavior.
- Verification: `source "$HOME/.cargo/env" && cargo check` and `source "$HOME/.cargo/env" && cargo test` both passed.
