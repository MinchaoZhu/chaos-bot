# task-2: Introduce A First-Class CLI Runtime

## Task
- Description: Add a Rust CLI entrypoint that can boot the existing backend runtime and expose command groups without requiring the React or Tauri shells.
- Scope: Cargo targets, runtime bootstrap, shared app context, global flags, and server compatibility wiring.
- Risk: Medium. The current startup path is centered on `chaos_bot_backend.rs`, so careless refactoring could break config loading, logging, or restart semantics.
- Status: done

## Phase 1: Establish CLI Binary And Command Model
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Choose the binary layout for the new CLI and record whether it lives as an additional backend bin target or a new workspace crate. | done | Kept churn low by adding a second bin target inside `backend/Cargo.toml`: the package now exposes `chaos-bot` as the default run target while retaining `chaos-bot-backend` for compatibility. |
| 1.2 | Define top-level CLI commands and global flags for config path, workspace override, output mode, and non-interactive operation. | done | Added a Clap-driven command tree in `backend/src/runtime/cli.rs` covering `serve`, `chat`, `sessions`, `config`, `skills`, `upgrade`, and `channels`, plus global `--config`, `--workspace`, `--output`, and `--non-interactive` flags. |
| 1.3 | Update Cargo and make targets so the CLI-first executable becomes the primary developer entrypoint. | done | `Makefile` now builds `chaos-bot`, runs the new CLI via `cargo run -p chaos-bot-backend -- serve`, and keeps `run-backend-compat` for the legacy wrapper binary during migration. |
| 1.v1 | Verify: `cargo run ... -- --help` and project task notes describe the planned command tree and primary binary name. | done | Verified with `cargo run -p chaos-bot-backend -- --help`, which now reports the `chaos-bot` binary name and the expected top-level command groups. |

## Phase 2: Extract Shared Runtime Bootstrap
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Refactor startup so config loading, logging initialization, and app-state construction can be reused by both `serve` and local CLI commands. | done | Extracted `load_runtime_config`, `build_server_context`, `build_command_context`, and `serve_http` into `backend/src/runtime/mod.rs`; both binaries now flow through the same logging/bootstrap path instead of duplicating startup in `chaos_bot_backend.rs`. |
| 2.2 | Introduce a CLI app context that exposes `AppConfig`, session/config/skill/upgrade services, and agent state without coupling to HTTP routing. | done | Added `AppContext` with direct accessors for chat, sessions, config, and upgrade services, while CLI handlers call skill and channel state directly without going through HTTP routes. |
| 2.3 | Make restart behavior explicit for CLI invocations so config mutations can disable `std::process::exit` unless `serve` is running. | done | Added `RestartMode::server_default()` for server boot and force `RestartMode::Disabled` in local CLI commands so config mutations stay deterministic for agents and tests. |
| 2.v1 | Verify: Local CLI boot no longer depends on `frontend_dist`, SPA fallback serving, or a bound TCP socket. | done | Verified with `cargo test -p chaos-bot-backend build_command_context_omits_frontend_dist -- --nocapture`; command-mode boot now strips `frontend_dist` and builds shared state without binding a listener. |

## Phase 3: Preserve Server Compatibility During Migration
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Decide whether `chaos-bot-backend` remains as a thin wrapper around `serve` during the migration window. | done | Retained `chaos-bot-backend` as a thin wrapper over `runtime::cli::run`, which defaults to `serve` when no command is passed so existing server-oriented usage keeps working. |
| 3.2 | Record any temporary compatibility shims needed for Telegram/webhook or upgrade routes that still rely on the HTTP server. | done | The HTTP router still owns webhook/API surfaces under `serve`, while CLI-native command handlers now call config, upgrade, sessions, chat, skills, and channel-health services directly. |
| 3.v1 | Verify: The migration notes state which server entrypoints are retained temporarily and which become CLI-native first. | done | The command tree, wrapper binary, and task notes now explicitly separate CLI-native flows from server-only compatibility routes, giving task-3 a concrete baseline for deeper command behavior. |
