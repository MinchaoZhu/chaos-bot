# task-6: Strip HTTP Server, Telegram, And Channel Layers — Simplify To Pure CLI

## Task
- Description: Remove the HTTP/API server (`serve` command), Telegram integration, and channel dispatcher subsystem entirely. Simplify the runtime so every CLI invocation is a self-contained process that loads config, runs exactly one command, outputs to stdout/stderr, and exits. Merge the redundant `build_server_context`/`build_command_context` paths into a single `build_context`. Eliminate the `chaos-bot-backend` compatibility binary. Refactor `cli.rs` into a module directory. Move shared response types out of the HTTP layer.
- Scope: `runtime/mod.rs`, `runtime/cli.rs`, `interface/http.rs`, `infrastructure/channels/`, `infrastructure/config.rs`, `infrastructure/session_store.rs`, `application/chat_service.rs`, `domain/ports.rs`, `domain/chat.rs`, `Cargo.toml`, `Makefile`, CI, release scripts, tests (`api_routes.rs`, `api_integration.rs`, `cli_integration.rs`, `support/mod.rs`).
- Risk: High. The HTTP server was the Telegram webhook receiver and the only network interface. After this task, the product is intentionally local-only until future adapters are designed as separate binaries or plugins.
- Status: done

## Phase 1: Remove Telegram And Channel Dispatcher
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Delete `backend/src/infrastructure/channels/telegram.rs` and `backend/src/infrastructure/channels/mod.rs`. Remove `mod channels` from `infrastructure/mod.rs`. | done | Removed the entire channels subsystem from `backend/src/infrastructure/`. |
| 1.2 | Remove `ChannelConnectorPort` and `ChannelDispatcherPort` traits from `domain/ports.rs`. Remove `InboundChannelMessage`, `OutboundChannelMessage`, `ChannelDelivery`, `ChannelHealth`, and `ChannelContext` from `domain/chat.rs`. | done | Domain chat/port surfaces are CLI-only now. |
| 1.3 | Strip `channel_dispatcher` from `ChatService` — remove `run_channel_message()`, the `channel` field on `ChatCommand`, and channel-session mapping logic (`resolve_session` channel branch, `channel_session_key`, `bind_channel_session`). `ChatService::new` should only take `agent` + `sessions`. | done | `ChatService` now only orchestrates agent execution plus persisted sessions. |
| 1.4 | Remove all Telegram config fields from `AppConfig` (`telegram_bot_token`, `telegram_enabled`, `telegram_webhook_secret`, `telegram_webhook_base_url`, `telegram_polling`, `telegram_api_base_url`) and from `AgentFileConfig` (`AgentTelegramConfig`). Remove `TELEGRAM_BOT_TOKEN` from `EnvSecrets`. | done | Runtime config keeps only CLI-relevant workspace/llm/search/logging/secrets state. |
| 1.5 | Remove `maybe_spawn_telegram_poller()` and `build_dispatcher()` calls from `runtime/mod.rs`. | done | Runtime bootstrap no longer starts long-lived background tasks. |
| 1.v1 | Verify: `cargo check -p chaos-bot-backend` compiles with zero telegram/channel references in `backend/src/`. | done | Verified with `cargo check -p chaos-bot-backend`. |

## Phase 2: Remove HTTP Server And `serve` Command
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Delete `backend/src/interface/http.rs` (the entire Axum router, all API handlers, SSE streaming, request/response types). Remove `mod http` from `interface/mod.rs`. If `interface/` becomes empty, remove the directory. | done | Removed `backend/src/interface/` entirely. |
| 2.2 | Remove the `Serve` variant from `CommandGroup`, `ServeArgs`, and `run_serve()` from `cli.rs`. Change the default command (when no subcommand given) from `Serve` to printing help or a suitable default like `chat`. | done | Default invocation now prints help; `serve` is gone. |
| 2.3 | Remove `serve_http()`, `shutdown_signal()`, and the `TcpListener`/`axum` imports from `runtime/mod.rs`. | done | `runtime/mod.rs` only builds CLI context now. |
| 2.4 | Remove `axum`, `tokio-stream`, and `futures` from `Cargo.toml` dependencies if no longer used. Keep `tokio` (needed for async runtime). | done | `tokio-stream` was removed; `axum` moved to dev-dependencies for local test servers; `futures` remains in production for model streaming. |
| 2.5 | Delete `backend/tests/api_routes.rs`, `backend/tests/api_integration.rs`, and `backend/tests/support/mod.rs`. Update `Makefile` to remove `test-integration` target (or redefine it to only run `cli_integration`). | done | API tests/support were removed and the Makefile test gate is now unit + CLI only. |
| 2.v1 | Verify: `cargo check -p chaos-bot-backend` compiles. No `axum`, `Router`, `Sse`, or `/api/` references remain in `backend/src/`. | done | Verified with `cargo check -p chaos-bot-backend`; no HTTP runtime paths remain. |

## Phase 3: Persist Sessions To Disk And Redesign Chat UX
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Replace the in-memory `SessionStore` (`HashMap` behind `Arc<RwLock>`) with a file-backed store. Each session is a JSON file under `{workspace}/sessions/{session-id}.json` containing the serialized `SessionState` (id, created_at, updated_at, messages). `list` scans the directory; `get`/`upsert`/`delete` operate on individual files. | done | `SessionStore` is file-backed and validates session ids before touching disk. |
| 3.2 | Simplify `ChatCommand` to remove `channel` field (already gone from Phase 1). `ChatService::new` takes `agent` + `SessionStore` only. `resolve_session` no longer needs channel-key mapping — just look up by ID or auto-create. | done | `ChatCommand` is message + optional session id only. |
| 3.3 | Redesign the `chat` CLI UX: `chat <MESSAGE>` sends a one-shot message (auto-creates a new session, prints response, exits). `chat <SESSION_ID> <MESSAGE>` continues an existing session (loads history from disk, appends exchange, saves back). `chat --session <ID> --stdin` reads message from stdin for piping. `chat --stream` variants emit streaming output. | done | Implemented one-shot, continuation, stdin, and streaming flows in `runtime/cli/chat.rs`. |
| 3.4 | Redesign `sessions` commands: `sessions list` reads the `sessions/` directory and prints session summaries (id, created, updated, message count). `sessions get <ID>` prints full conversation history. `sessions delete <ID>` removes the file. `sessions create` is no longer needed as a standalone command (chat auto-creates). | done | `sessions create` was removed; list/get/delete operate on persisted session files. |
| 3.5 | Remove `channel_bindings` from `SessionStore` and `bind_channel_session`/`session_for_channel_key` methods (channel concept is gone). | done | Channel/session mapping code was deleted with the in-memory store. |
| 3.v1 | Verify: `cargo test -p chaos-bot-backend` passes. Sessions survive across CLI invocations: `chaos-bot chat "hello"` creates a session, `chaos-bot sessions list` shows it, `chaos-bot chat <ID> "follow up"` appends to it. | done | Verified by `cargo test -p chaos-bot-backend` and the expanded `cli_integration.rs` smoke coverage. |

## Phase 4: Unify Runtime Bootstrap And Remove Redundant Binary
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | Merge `build_server_context()` and `build_command_context()` into a single `build_context()` in `runtime/mod.rs`. Remove `RuntimeKind` enum since there is only one mode now. Simplify `build_state()` to always create `ConfigRuntime` (no `Option<AgentFileConfig>` branch). | done | `runtime/mod.rs` now exposes a single CLI bootstrap path. |
| 4.2 | Flatten `AppState` — remove `channel_dispatcher`, `telegram_*` fields. Consider whether `AppState` should be renamed or merged into `AppContext` since the HTTP layer is gone. | done | `AppState` was removed and its remaining state was merged into `AppContext`. |
| 4.3 | Remove `backend/src/runtime/bin/chaos_bot_backend.rs`. Remove the `chaos-bot-backend` `[[bin]]` target from `Cargo.toml`. Update `Makefile` to remove `run-backend-compat`. The package name can stay `chaos-bot-backend` (crate name) but the only binary is `chaos-bot`. | done | The crate package name stays, but only `chaos-bot` is built and shipped. |
| 4.4 | Avoid double-creating `SkillStore` — pass the `Arc<dyn SkillPort>` created in `build_state()` into `build_agent_loop()` instead of constructing a second one. | done | Runtime bootstrap now creates one `SkillStore` and reuses it. |
| 4.v1 | Verify: `cargo build -p chaos-bot-backend --bin chaos-bot` succeeds. Only one `[[bin]]` entry in `Cargo.toml`. `build_context` is the single bootstrap path. | done | Verified during package/build gates, including `make package-verify`. |

## Phase 5: Refactor `cli.rs` Into Module Directory
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | Split `backend/src/runtime/cli.rs` (~900 lines) into `backend/src/runtime/cli/mod.rs` (Clap definitions, `run()`, `run_parsed()`, `bootstrap_runtime()`, error types) and per-command-group handler files: `cli/chat.rs`, `cli/sessions.rs`, `cli/config.rs`, `cli/skills.rs`, `cli/upgrade.rs`. | done | Added `runtime/cli/{mod,chat,sessions,config,skills,upgrade}.rs`. |
| 5.2 | Extract output formatting functions (`write_output`, `render_*_text`, `write_chat_stream_event`, etc.) into `cli/output.rs`. | done | Output contracts and renderers now live in `runtime/cli/output.rs`. |
| 5.3 | Remove the `channels` command group entirely (no dispatcher means no channel status to report). | done | `channels` no longer exists. |
| 5.v1 | Verify: `cargo test -p chaos-bot-backend` passes. No single file exceeds ~300 lines in the cli module. | done | Verified with `cargo test -p chaos-bot-backend`; the CLI is split across small files. |

## Phase 6: Update Release, CI, Docs, And Final Verification
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 6.1 | Update `scripts/release/package-linux-x86_64.sh` to ship only `chaos-bot` (no `chaos-bot-backend`). Update installer scripts and release metadata generation. | done | Linux bundles/installers now ship one launcher and one runtime binary. |
| 6.2 | Update `scripts/release/verify-packaged-runtime.sh` — remove health-check HTTP probe, serve-mode startup test, and any API-based assertions. Verify CLI commands instead. | done | Packaged runtime verification now checks chat/session/config flows only. |
| 6.3 | Update `scripts/release/verify-self-upgrade.sh` — remove `upgrade relaunch` if it depends on a running server process, or redefine it as a CLI-only operation. | done | Self-upgrade verification keeps `upgrade relaunch` as a CLI-only JSON contract with no API health probe. |
| 6.4 | Update `.github/workflows/ci.yml` and `.githooks/pre-push` to remove any `serve`/API test references. | done | CI now uploads unit/CLI artifacts only; Makefile/API references were removed. |
| 6.5 | Update `README.md` — remove all HTTP API documentation, `serve` command references, Telegram setup instructions, and channel configuration. Document the pure CLI workflow with session persistence and `chat <SESSION_ID>` continuation. | done | README now documents the pure CLI workflow and persisted sessions. |
| 6.6 | Expand `cli_integration.rs` — add coverage for session persistence across invocations, `chat <SESSION_ID> <MESSAGE>` continuation, `--config`/`--workspace` global flags, stdin input modes, error exit codes, and edge cases previously only tested via `api_routes.rs`. | done | `cli_integration.rs` now covers persisted sessions, workspace override, stdin, streaming, delete/not-found, skills, and upgrade status. |
| 6.v1 | Verify: `make test-all`, `make release-check`, `make package-verify`, `make install-verify`, `make upgrade-verify` all pass. No references to `serve`, `/api/`, `telegram`, `webhook`, or `channel_dispatcher` remain in active code (excluding PM history files). | done | Verified with `make release-check`, `make test-all`, `make package-verify`, `make install-verify`, and `make upgrade-verify`. |
