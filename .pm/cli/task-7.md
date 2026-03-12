# task-7: Tighten CLI Logging, Session Targeting, And Local-Time Tool Use

## Task
- Description: Remove tracing noise from CLI stdout/stderr so command output remains machine-readable, make `chat <SESSION_ID> <MESSAGE>` continuation semantics explicit instead of silently degrading to a new session when the first positional token looks like a session id but does not resolve, and improve agent/tool guidance so requests for live local runtime state such as the current system time prefer the available `bash` tool when safe.
- Scope: `backend/src/infrastructure/logging.rs`, `backend/src/runtime/cli/`, `backend/src/application/chat_service.rs`, `backend/src/application/agent.rs`, `backend/src/infrastructure/tooling/mod.rs`, personality templates under `templates/personality/`, CLI integration/unit tests covering logging, session continuation, and tool-use behavior.
- Risk: Medium. Tightening CLI logging can break current diagnostics if file logging is not preserved, session parsing changes can turn a previously permissive flow into an error path, and prompt/tool-use guidance must avoid over-triggering shell commands for questions that do not require live local state.
- Status: done

## Phase 1: Stop Tracing From Polluting CLI Output
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Change CLI logging initialization so tracing events are written only to the rotating log file by default, not to stdout/stderr, while keeping retention cleanup and log file metadata intact. | done | `init_logging()` now installs only the file-backed tracing layer; retention cleanup is still recorded, but only after subscriber init so success paths stay script-safe. |
| 1.2 | Audit CLI-facing commands and output helpers to ensure only user-facing command results go to stdout and structured errors stay on stderr. | done | CLI success flows now emit only command payloads; regression coverage checks `sessions list` text output plus JSON chat output with empty stderr. |
| 1.v1 | Verify: CLI integration or unit coverage proves that `sessions`, `chat`, and other normal commands emit clean stdout while log entries still land in `{workspace}/logs/*.log`. | done | `cargo test -p chaos-bot-backend --test cli_integration -- --nocapture` passed with `cli_success_output_is_clean_and_logs_stay_in_file`. |

## Phase 2: Fix Session Continuation Targeting Semantics
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Refine `chat` argument resolution so explicit session targeting is not silently lost when the first positional token is intended as a session id. | done | Positional `chat <SESSION_ID> <MESSAGE>` now preserves continuation intent by checking UUID-like or hex-prefix candidates before falling back to plain message text. |
| 2.2 | Decide and implement the CLI contract for unknown session ids in positional mode. | done | Unknown positional session-like identifiers now fail with `not_found` and a `--session <ID>` hint; explicit `--session <ID>` remains the escape hatch for intentional named-session creation/continuation. |
| 2.3 | Add regression coverage for exact id continuation, truncated/unknown id handling, and `--session <ID>` behavior so future parser changes do not reintroduce ambiguity. | done | Added CLI integration coverage for exact continuation, truncated-id failure, and explicit named-session creation, plus a unit test for session-id shape detection. |
| 2.v1 | Verify: automated tests cover both successful continuation and the intended failure/creation path for bad session identifiers. | done | `cargo test -p chaos-bot-backend --test cli_integration -- --nocapture` and `cargo test -p chaos-bot-backend` both passed. |

## Phase 3: Improve Agent Use Of Local Tools For Live Runtime Facts
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Update the agent/personality/tool guidance so safe local-environment questions (for example current system time, working directory, or local file presence) prefer a direct tool call over a speculative natural-language answer. | done | `AgentLoop::build_system_prompt()` now appends explicit runtime-tool guidance, and the default personality template reinforces the same behavior for bootstrapped workspaces. |
| 3.2 | Confirm the `bash` tool contract is clear enough for the model to select allowlisted commands such as `date`, and adjust tool descriptions or prompt wording if needed. | done | The `bash` tool description now names `date`, `pwd`, `ls`, `rg`, and `cat`, and disallowed-command errors explain the allowlist directly. |
| 3.3 | Add tests for the intended behavior, either at the agent level with a controllable provider/tool mock or at CLI integration level with deterministic expectations. | done | Added unit coverage for runtime tool guidance, `bash` description clarity, and an agent-level provider mock that only chooses `bash date` when the prompt plus tool contract are explicit enough. |
| 3.v1 | Verify: the documented/current-time style query results in a safe `bash` tool path under test coverage, and the final answer reflects live command output instead of “I cannot read your device time.” | done | `cargo test -p chaos-bot-backend --test unit_agent -- --nocapture` passed with `live_time_queries_use_bash_and_return_live_output`. |

## Verification Notes
- `cargo test -p chaos-bot-backend --test unit_logging -- --nocapture`
- `cargo test -p chaos-bot-backend --test unit_tools -- --nocapture`
- `cargo test -p chaos-bot-backend --test unit_agent -- --nocapture`
- `cargo test -p chaos-bot-backend --test cli_integration -- --nocapture`
- `cargo test -p chaos-bot-backend`
- `make test-all`
