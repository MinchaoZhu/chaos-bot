# task-3: Implement CLI Command Coverage For Agent Workflows

## Task
- Description: Expose the core user and agent workflows as deterministic CLI commands backed directly by the existing backend services.
- Scope: Chat, sessions, config, skills, upgrades, output formats, and command-level error contracts.
- Risk: Medium. The current frontend absorbs protocol details like SSE parsing and session selection, so the CLI must make those flows explicit without changing backend behavior.
- Status: done

## Phase 1: Cover Session And Chat Flows
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Implement `sessions list/create/get/delete` commands using `SessionService` and the existing session store behavior. | done | `backend/src/runtime/cli.rs` now routes these subcommands directly through `SessionService`, preserving the in-memory ordering and session ID semantics already used by `/api/sessions`. |
| 1.2 | Implement `chat send` and `chat stream` commands on top of `ChatService::run_stream`, including automatic session creation when no session ID is provided. | done | `chat send` and `chat stream` both use `ChatService::run_stream`; `chat stream` emits direct CLI events (`session` / `delta` / `tool_call` / `done`) instead of SSE framing, and missing session IDs still auto-create sessions via the existing service logic. |
| 1.3 | Decide how local CLI chat handles channel-specific behavior and whether channel mapping remains out of scope for the initial local workflow. | done | The first CLI slice keeps local chat focused on direct session workflows. Channel-specific bindings remain server/webhook concerns, while `channels status` stays available as a diagnostics command and README documents that scope decision. |
| 1.v1 | Verify: Session and chat commands have stable output/exit-code rules for both plain-text and machine-readable modes. | done | Added parser/contract tests in `backend/src/runtime/cli.rs`, documented `text/json/jsonl` behavior in `README.md`, and verified `cargo test -p chaos-bot-backend runtime::cli::tests -- --nocapture` plus `cargo run -p chaos-bot-backend -- chat --help`. |

## Phase 2: Cover Config, Skills, Upgrade, And Status Flows
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Implement `config get/apply/reset/restart` commands using `ConfigService`, including raw JSON input and structured mutation modes. | done | `config get` is now the primary subcommand (with `show` kept as an alias), and `apply/restart` accept `--raw`, `--file`, or `--stdin` so config mutation still flows through `ConfigService` and `ConfigRuntime` backup/reload semantics. |
| 2.2 | Implement `skills list/get/install` and `upgrade status/apply/relaunch` commands without requiring HTTP requests. | done | CLI commands now call the same `SkillStore` and `UpgradeService` used by the API layer, removing the need to bounce through HTTP for local automation paths. |
| 2.3 | Add `channels status` or equivalent diagnostics if connector visibility is still needed once the GUI disappears. | done | `channels status` remains part of the CLI surface and renders both connector health and Telegram runtime flags for operator diagnostics. |
| 2.v1 | Verify: Every mutable command supports non-interactive invocation with flags/stdin suitable for code-agent automation. | done | Config mutation commands now support stdin-based raw JSON, `skills install` remains flag/arg driven, and the README includes non-interactive CLI examples for code-agent usage. |

## Phase 3: Define Output Contracts For Automation
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Add structured output modes such as `--json` or line-delimited stream output for sessions, config, and chat events. | done | `--output text|json|jsonl` is now available across the CLI; non-stream commands can emit JSON/JSONL, and `chat stream` emits line-delimited `jsonl` events for automation. |
| 3.2 | Standardize stderr/error messages and exit codes for config validation errors, network/provider failures, and tool execution failures. | done | The CLI now maps failures into explicit categories with documented exit codes: invalid input, not found, unavailable, config validation, network, provider, tool, and generic execution failures. |
| 3.v1 | Verify: The command contract is documented well enough to replace frontend-driven feature development and tests. | done | `README.md` now documents the CLI-first command surface, output modes, exit-code contract, and the local-chat/channel scope decision, while `cargo run -p chaos-bot-backend -- --help`, `chat --help`, and `config --help` were verified against the compiled binary. |
