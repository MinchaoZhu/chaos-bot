# task-2: Complete Verification Framework

## Task
- Description: Build a comprehensive test suite covering unit tests (85%+ line coverage), API integration tests, and Playwright e2e tests, with DI-based mocking for all third-party dependencies.
- Scope: Unit tests for all backend modules, HTTP-chain integration tests, Playwright e2e tests against mock-mode backend, CI Makefile targets, and coverage gating.
- Risk: Coverage target may require exposing private helpers via pub(crate); e2e tests need a mock provider mode to run without API keys.
- Status: done

## Phase 1: Test infrastructure & shared fixtures

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Add dev-deps to Cargo.toml: serial_test, assert_json_diff | done | |
| 1.2 | Create backend/tests/support/mod.rs with MockStreamProvider, MockTool, helpers | done | |
| 1.3 | Add Makefile targets: test, coverage, coverage-check | done | |
| 1.v1 | Verify: cargo test compiles with new support module | done | |

## Phase 2: Unit tests for all backend modules (85%+ coverage)

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | types.rs — Message constructors, serde round-trip, SessionState operations | done | |
| 2.2 | sessions.rs — CRUD, ordering, concurrent access | done | |
| 2.3 | memory.rs — ensure_layout, curated R/W, daily log, search, get_file with ranges | done | |
| 2.4 | personality.rs — load order, missing files, empty dir | done | |
| 2.5 | config.rs — env var defaults and overrides (serial_test) | done | |
| 2.6 | tools/mod.rs — ToolRegistry register/dispatch/specs; each tool; helper functions | done | |
| 2.7 | llm/mod.rs — map_messages, map_tools, drain_sse_payloads, process_stream_payload, flush_tool_calls | done | |
| 2.8 | agent.rs — build_system_prompt, enforce_token_budget, run with mock | done | |
| 2.v1 | Verify: cargo test --workspace passes; cargo llvm-cov ≥ 85% | done | `cargo llvm-cov --workspace --summary-only --fail-under-lines 85` passed, total line coverage 85.94% |

## Phase 3: API integration tests (HTTP chain)

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Health endpoint (200 + JSON body) | done | |
| 3.2 | Session CRUD lifecycle (create → get → list → delete → 404) | done | |
| 3.3 | Chat SSE streaming — simple text response | done | |
| 3.4 | Chat SSE streaming — with tool calls | done | |
| 3.5 | Chat with existing session (conversation accumulates) | done | |
| 3.6 | Chat error handling (mock returns error → SSE error event) | done | |
| 3.7 | Static file serving (/, /app.js, /style.css) | done | |
| 3.v1 | Verify: cargo test --workspace passes | done | `make test-integration` and `cargo test --workspace` passed |

## Phase 4: Playwright e2e tests

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | Create e2e/ with package.json and playwright.config.ts | done | |
| 4.2 | Add MockProvider variant when CHAOS_PROVIDER=mock | done | |
| 4.3 | Tests: page load, session creation, send message, tool calls, session switching | done | |
| 4.v1 | Verify: npx playwright test passes against mock-mode backend | done | `make test-e2e` passed (4/4 tests) |

## Phase 5: CI integration & AGENTS.md

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | Makefile targets: test-unit, test-integration, test-e2e, test-all, coverage-report, coverage-check | done | |
| 5.2 | Write Verification section in AGENTS.md | done | |
| 5.3 | Update AGENTS.md task index for task-2 | done | |
| 5.v1 | Verify: make test-all runs successfully | done | `make test-all` passed (unit + integration + e2e) |

## Completion Notes (2026-02-23)
- Added missing Playwright project under `e2e/` (`package.json`, `playwright.config.ts`, `tests/chat.spec.ts`) and stabilized tests for persistent session history.
- Expanded root `Makefile` with `test-unit`, `test-integration`, `test-e2e`, `test-all`, `coverage`, `coverage-report`, and `coverage-check` targets.
- Added `backend/tests/unit_bootstrap.rs` so bootstrap path is covered and overall line coverage reaches threshold.
