# task-3: Dependency Injection Refactoring Plan

## Task
- Description: Refactor current architecture to explicit Rust-native DI (`trait + Arc<dyn Trait> + constructor injection`) without introducing any DI container.
- Scope: `backend/src/memory.rs`, `backend/src/agent.rs`, `backend/src/tools/mod.rs`, `backend/src/main.rs`, `backend/src/personality.rs`, `backend/src/config.rs`, `backend/tests/support/mod.rs`, `backend/tests/unit_tools.rs`, `backend/tests/unit_agent.rs`, `backend/tests/api_integration.rs`, `e2e/tests/chat.spec.ts` (if assertions need DI-aware updates).
- Risk: Trait-object migration touches multiple module boundaries; constructor and test fixture signatures will change across agent/tool/main/test paths.
- Status: done

## Phase 1: AgentConfig + MemoryBackend trait

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Add `AgentConfig` in `agent.rs` to group model/runtime scalar fields (`model`, `temperature`, `max_tokens`, `max_iterations`, `token_budget`, `working_dir`). | done | Reduce `AgentLoop` constructor parameters from 10 to 5. |
| 1.2 | Replace scalar fields in `AgentLoop` with `config: AgentConfig`; migrate internal references to `self.config.*`. | done | Keep runtime behavior unchanged. |
| 1.3 | Add `MemoryBackend` async trait in `memory.rs` and implement it for existing `MemoryStore`. | done | Methods: `search`, `append_daily_log`, `get_file`, `read_curated`, `write_curated`, `ensure_layout`. |
| 1.4 | Change `AgentLoop` memory dependency to `Arc<dyn MemoryBackend>` and update constructor signature accordingly. | done | `new(provider, tools, personality, memory, config)`. |
| 1.5 | Change `ToolContext.memory` to `Arc<dyn MemoryBackend>` in `tools/mod.rs`; keep tool call sites unchanged. | done | `MemoryGetTool` / `MemorySearchTool` method use should remain stable. |
| 1.6 | Update composition wiring in `main.rs` to build `Arc<dyn MemoryBackend>` and `AgentConfig` before creating `AgentLoop`. | done | Keep startup flow unchanged. |
| 1.7 | Update tests (`tests/support/mod.rs`, `tests/unit_tools.rs`, `tests/unit_agent.rs`) to use `AgentConfig` + trait-object memory wiring. | done | Optional: add in-memory mock backend for agent-level tests. |
| 1.v1 | Verify: `cargo check` passes after Phase 1. | done | `cargo check -p chaos-bot-backend` passed. |
| 1.v2 | Verify: `cargo test` passes after Phase 1. | done | `cargo test -p chaos-bot-backend --no-run` and `cargo test -p chaos-bot-backend` passed. |
| 1.v3 | Verify: `cargo llvm-cov --fail-under-lines 85` passes after Phase 1. | done | `cargo llvm-cov --workspace --summary-only --fail-under-lines 85` passed (TOTAL Lines 87.98%). |

## Phase 2: PersonalitySource trait

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Add async trait `PersonalitySource` in `personality.rs` with `system_prompt() -> Result<String>`. | done | |
| 2.2 | Implement `PersonalitySource` for `PersonalityLoader`. | done | |
| 2.3 | Change `AgentLoop` personality dependency to `Arc<dyn PersonalitySource>`. | done | Replace concrete `PersonalityLoader` field. |
| 2.4 | Update `main.rs` wiring to construct `Arc<dyn PersonalitySource>` from `PersonalityLoader`. | done | |
| 2.5 | Add `MockPersonality` in tests and remove temporary-file-based personality setup where possible. | done | Simplify `build_test_agent` path. |
| 2.v1 | Verify: `cargo check` passes after Phase 2. | done | Passed. |
| 2.v2 | Verify: `cargo test` passes after Phase 2. | done | Passed. |
| 2.v3 | Verify: `cargo llvm-cov --fail-under-lines 85` passes after Phase 2. | done | Passed. |

## Phase 3: ToolContext constructor + assembly cleanup

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Add `ToolContext::new(root_dir: PathBuf, memory: Arc<dyn MemoryBackend>) -> Self` in `tools/mod.rs`. | done | |
| 3.2 | Use `ToolContext::new(...)` in `agent.rs` to replace inline struct construction. | done | Improve readability only; no behavior change. |
| 3.v1 | Verify: `cargo check` passes after Phase 3. | done | Passed. |
| 3.v2 | Verify: `cargo test` passes after Phase 3. | done | Passed. |
| 3.v3 | Verify: `cargo llvm-cov --fail-under-lines 85` passes after Phase 3. | done | Passed. |

## Phase 4: Composition root + AppConfig defaults

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | Extract `build_app(config: &AppConfig) -> Result<AppState>` from `main.rs` as composition root. | done | Centralize dependency graph wiring. |
| 4.2 | Simplify `main()` to `let state = build_app(&config).await?;`. | done | |
| 4.3 | Add `Default` implementation for `AppConfig` in `config.rs` with test-friendly baseline values. | done | |
| 4.4 | Add `impl From<&AppConfig> for AgentConfig` in `agent.rs` (or suitable module). | done | |
| 4.v1 | Verify: `cargo check` passes after Phase 4. | done | Passed. |
| 4.v2 | Verify: `cargo test` passes after Phase 4. | done | Passed. |
| 4.v3 | Verify: `cargo llvm-cov --fail-under-lines 85` passes after Phase 4. | done | Passed. |

## Phase 5: End-to-End regression (完整 e2e)

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | Audit existing e2e assumptions for personality/memory bootstrapping and adjust to DI-based app assembly. | done | Ensure no hidden coupling to concrete `MemoryStore` / `PersonalityLoader` in test runtime setup. |
| 5.2 | Update e2e startup path to call the same composition-root behavior (`build_app`) used in production. | done | Avoid duplicated wiring logic in tests. |
| 5.3 | Add/adjust e2e scenario: create session -> send message -> stream response -> session persistence across reload. | done | Validate no regression from constructor/trait-object refactor. |
| 5.4 | Add/adjust e2e scenario: tool-call path still works with `ToolContext` trait-object memory dependency. | done | Verify tool events and final assistant output remain stable. |
| 5.5 | Add/adjust e2e scenario: memory-related interactions (`memory_get` / `memory_search`) still behave correctly after DI refactor. | done | Focus on observable UI/API behavior, not concrete backend type. |
| 5.6 | Add/adjust e2e scenario: mock-mode startup works without external API keys after DI migration. | done | Keep CI-friendly deterministic execution. |
| 5.v1 | Verify: `make test-e2e` passes locally. | done | Passed (Playwright 4/4). |
| 5.v2 | Verify: `make test-all` passes (unit + integration + e2e). | done | Passed. |
| 5.v3 | Verify: `cargo llvm-cov --fail-under-lines 85` remains green after e2e/integration assertion updates. | done | Passed (TOTAL Lines 87.98%). |

## Out of Scope (No Changes Planned)
- `SessionStore`: keep concrete type (current in-memory HashMap is sufficient).
- `ToolRegistry`: keep concrete type (lookup registry; extension point already at `Tool` trait).
- `LlmProvider`: no change (already `trait + Arc`).
- `Tool` trait: no change (already abstracted).

## Target Dependency Graph (Post-Refactor)
- `main.rs` / `build_app(&AppConfig)` composes:
  - `Arc<dyn MemoryBackend>` (`MemoryStore` / in-memory mock)
  - `Arc<dyn PersonalitySource>` (`PersonalityLoader` / `MockPersonality`)
  - `Arc<dyn LlmProvider>`
  - `Arc<ToolRegistry>`
  - `AgentConfig`
  - `AgentLoop::new(provider, tools, personality, memory, config)`

## Final Verification Goals
- Agent-level tests no longer require temporary filesystem for personality/memory (use mocks).
- `build_test_agent()` signature and setup become simpler and more explicit.
- `build_app()` in `main.rs` clearly documents full dependency composition.
- E2E flows pass without regression under DI refactor (`make test-e2e`, `make test-all`).

## Completion Notes (2026-02-23)
- Introduced `MemoryBackend` and `PersonalitySource` async traits, migrated `AgentLoop` and `ToolContext` to trait-object dependencies with constructor injection.
- Added `AgentConfig` and `impl From<&AppConfig> for AgentConfig`; added `AppConfig::default()`; extracted `build_app(&AppConfig)` as composition root.
- Updated tests and fixtures to new constructor signatures, added `MockPersonality`, and eliminated support-module dead-code warnings via module-level allowance.
- Verification passed: `cargo check -p chaos-bot-backend`, `cargo test -p chaos-bot-backend`, `cargo llvm-cov --workspace --summary-only --fail-under-lines 85` (TOTAL Lines 87.98%), `make test-e2e` (4/4), `make test-all`.
