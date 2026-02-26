# PM Runtime AGENTS

## Current Status
- Project: multi-channel-agents
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/multi-channel-agents
- Active Task: task-1
- Last Updated: 2026-02-26T01:48:39+08:00

## Task Index
- task-1: in_progress

## Verification
- new-project initialized for multi-channel-agents on branch feat/multi-channel-agents in git worktree mode.
- task-1 updated: connector framework planning and Telegram (BotFather) integration phases added.
- Pre-task gate executed: rebased current branch onto latest local `master` and latest remote `origin/master`.
- Implemented channel dispatcher framework (`ChannelConnectorPort`/`ChannelDispatcherPort`) with runtime wiring.
- Implemented Telegram connector + webhook ingress route + channel session mapping/reuse.
- Implemented Telegram polling runtime loop (`getUpdates`) when `channels.telegram.polling=true`.
- Added connector lifecycle and health contract (`start/stop/health`) plus retry/backoff for Telegram sends.
- Added config/secrets support: `channels.telegram.*` and `TELEGRAM_BOT_TOKEN` / `secrets.telegram_bot_token`.
- Added channel status API and frontend runtime display (`/api/channels/status`, rail connector meta).
- Added test coverage: unit (types/config), integration (telegram webhook/session reuse/secret check), e2e (telegram webhook route).
- Added failure-path coverage for retry/outage in integration and e2e.
- `make test-e2e` passed (desktop/mobile + telegram webhook case).
- `make test-all` passed (unit + integration + e2e).

## Mandatory Rules
- Before executing any coding task, the current branch must be rebased onto the latest local `master` and latest remote `origin/master`.
- Every task must run `make test-all` before it can be marked complete.
- Every new feature must include a dedicated e2e testing phase in its task plan.

## Architecture Governance (Frozen)
- Backend source root is limited to DDD five layers + `lib.rs`: `application/`, `domain/`, `infrastructure/`, `interface/`, `runtime/`.
- New business directories at `backend/src` root are forbidden.
- `infrastructure/model` is the only implementation location for model providers.
- `infrastructure/tooling` is the only implementation location for tool registry and tool implementations.
- `application` must depend on `domain::ports` contracts only, not concrete adapters.
- Dependency injection and adapter wiring must be handled in `runtime`.
- Reverse dependencies across layers are forbidden.
- `README.md` is the single maintained documentation entry for architecture/packaging/runtime/testing.
- Root-level `docs/` directory is removed; do not re-introduce it for primary project documentation.

## Config Standards
- Config file path is fixed by runtime rules; env vars do not select/override config file path.
- Default config path: `~/.chaos-bot/config.json`.
- Legacy compatibility: if `~/.chaos-bot/config.json` is missing but `~/.chaos-bot/agent.json` exists, runtime loads `agent.json`.
- Startup materialization: if no config file exists, runtime creates `~/.chaos-bot/config.json` from embedded defaults.
- Secrets merge order: env API keys first, then config secrets override if provided.
- Backup policy: every config write rotates backups as `config.json.bak1` and `config.json.bak2`.
- Runtime config actions:
  - `reset`: restore disk config to current running config snapshot.
  - `apply`: hot-apply new config and rebuild runtime agent.
  - `restart`: apply config and request process restart (can be disabled by runtime mode).

## CI Artifact Standards
- CI workflow: `.github/workflows/ci.yml`.
- Full gate command: `make test-all`.
- Failure-preserve switch: `CHAOS_BOT_KEEP_TMP_ON_FAIL=1`.
- Failure artifact upload paths:
  - `.tmp/unit`
  - `.tmp/integration`
  - `.tmp/e2e/runtime`
  - `.tmp/e2e/artifacts`
- Retention: 14 days.

## Logging Standards
- Workspace path: default `~/.chaos-bot`; log dir default `<workspace>/logs`.
- File naming: one file per date, `YYYY-MM-DD.log`.
- Retention: startup cleanup keeps only `logging.retention_days` (default 7 days).
- Levels: `debug`, `info`, `warning`, `error` (`warning` maps to runtime `warn`).
- Required structured fields:
  - Startup/config: `workspace`, `log_dir`, `log_file`, `log_level`, `retention_days`.
  - API/chat: `session_id`, `message_chars`, `finish_reason`, `usage_total_tokens`.
  - Tool chain: `tool_call_id`, `tool_name`, `is_error`.
- Sensitive data: never log secrets or raw API keys.
- Troubleshooting flow:
  - Check latest file: `tail -f ~/.chaos-bot/logs/$(date +%F).log`
  - Validate retention cleanup on restart.
  - Correlate issues by `session_id` and `tool_call_id`.

## PM File Map
- `.pm/docs/project.md`: Project context for `multi-channel-agents` including requirements and constraints.
- `.pm/docs/AGENTS.md`: PM runtime status mirror for docs sync.
- `.pm/multi-channel-agents/`: Project-scoped task directory for this project.
- `.pm/multi-channel-agents/task-1.md`: Updated task plan for connector framework and Telegram.
- `.pm/bot/`: Historical completed task records retained from prior project context.
- `AGENTS.md`: Shared runtime status, task index, and verification summary.
- `CLAUDE.md`: Symlink to `AGENTS.md`.

## Next Actions
1. Complete frontend connector configuration editing surface (not only status display) via existing config APIs.
2. Add graceful shutdown hooks to invoke connector `stop_all` on process termination.
3. Reassess task-1 completion after frontend config UX is implemented and re-verified.
