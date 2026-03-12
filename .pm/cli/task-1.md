# task-1: Audit And Plan CLI-First Migration

## Task
- Description: Complete a code-backed migration plan for turning the repository into a CLI-first project while preserving the backend service core.
- Scope: Runtime entrypoints, backend service boundaries, frontend/Tauri coupling, release/test scripts, and the resulting execution backlog.
- Risk: Medium. Several release, version, and static-asset paths still assume a shipped frontend, so deleting GUI code prematurely would break packaging and verification.
- Status: done

## Phase 1: Inventory Current Runtime Surface
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Audit backend, frontend-react, src-tauri, e2e, and release paths to separate CLI-critical code from GUI-only code. | done | Confirmed the current process entrypoint is `backend/src/runtime/bin/chaos_bot_backend.rs`; GUI layers sit in `frontend-react/` and `src-tauri/`, while release/test scripts in `scripts/` and `Makefile` still orchestrate them. |
| 1.2 | Record the keep/remove boundary for backend modules, shared scripts, frontend assets, and Tauri packaging. | done | Keep: `backend/src/{application,domain,infrastructure,interface,runtime}` and shell-safe scripts. Remove after parity: `frontend-react/`, `src-tauri/`, `e2e/`, SPA fallback serving, and GUI-specific packaging/version checks. |
| 1.v1 | Verify: The inventory notes identify all current entrypoints, build targets, and GUI-dependent flows. | done | Verified against `Makefile`, `README.md`, `backend/src/interface/http.rs`, `frontend-react/src/runtime/*`, `src-tauri/src/lib.rs`, `scripts/run-e2e.sh`, and `scripts/release/package-linux-x86_64.sh`. |

## Phase 2: Define CLI-First Target
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Specify the target CLI UX, primary commands, and how each command maps onto existing backend services. | done | Planned a first-class Rust CLI with command families for `serve`, `chat`, `sessions`, `config`, `skills`, `upgrade`, and optional channel status, mapped onto `ChatService`, `SessionService`, `ConfigService`, `SkillStore`, and `UpgradeService`. |
| 2.2 | Decide the migration order for introducing CLI flows before deleting frontend and Tauri surfaces. | done | Sequence is: add CLI bootstrap and shared runtime context, implement agent-facing commands, replace test/release automation, then remove GUI/Tauri and the SPA-serving branch. |
| 2.v1 | Verify: The plan names the first implementation slice plus the retained compatibility constraints. | done | First slice is a CLI runtime skeleton that boots `AppConfig`, logging, and shared service state without binding HTTP unless `serve` is requested; `chaos-bot-backend` remains temporarily as the server-compatible path during migration. |

## Phase 3: Seed Execution Tasks
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Break the migration into follow-up tasks for CLI bootstrap, command coverage, backend adaptations, and GUI removal. | done | Created `task-2` through `task-5` for CLI bootstrap, command implementation, automation/release refactor, and GUI removal. |
| 3.2 | Define the automated verification commands that future `run-task` work must satisfy. | done | Future checks must converge on Rust/CLI-native verification such as `cargo test --workspace`, command-level integration tests, packaging smoke tests, and grep-based confirmation that GUI references are removed. |
| 3.v1 | Verify: Follow-up tasks and verification strategy are captured before implementation starts. | done | The migration backlog now covers the current binary, HTTP/runtime coupling, CLI command surface, packaging/version scripts, and final frontend/Tauri deletion gates. |
