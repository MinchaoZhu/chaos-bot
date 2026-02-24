# chaos-bot

A personal AI agent assistant.

## Overview

chaos-bot is a personal AI agent designed to assist with everyday tasks through natural conversation. It leverages large language models to understand context, use tools, and take actions on your behalf.

## Common Commands

```bash
make build          # cargo build -p chaos-bot-backend
make run            # cargo run -p chaos-bot-backend
make clean-runtime  # delete workspace runtime files and .tmp
make test-all       # unit + integration + e2e (all in .tmp, auto-cleaned)
```

## Runtime Workspace

chaos-bot uses a dedicated runtime workspace:

- Default workspace: `~/.chaos-bot`
- All runtime-generated files are created under this workspace.
- Runtime config is loaded from `~/.chaos-bot/config.json` by default.

## Runtime Initialization Model

Runtime config and templates are embedded into the backend binary at compile time:

- `templates/config/agent.json`
- `templates/config/.env.example`
- `templates/MEMORY.md`
- `templates/personality/*.md`

At runtime, missing files are materialized automatically:

- `~/.chaos-bot/config.json` (default config source)
- `~/.chaos-bot/.env.example`
- `<workspace>/MEMORY.md`
- `<workspace>/personality/SOUL.md`
- `<workspace>/personality/IDENTITY.md`
- `<workspace>/personality/USER.md`
- `<workspace>/personality/AGENTS.md`
- `<workspace>/data/sessions/`

Existing files are preserved; only missing files are generated.

## Runtime Configuration (`config.json`)

`~/.chaos-bot/config.json` is runtime-generated from the embedded template if missing.
Legacy compatibility: if `config.json` is absent but `~/.chaos-bot/agent.json` exists, runtime uses `agent.json`.

```json
{
  "workspace": ".chaos-bot",
  "server": { "host": "0.0.0.0", "port": 3000 },
  "llm": { "provider": "openai", "model": "gpt-4o-mini" },
  "logging": {
    "level": "info",
    "retention_days": 7,
    "directory": "logs"
  },
  "secrets": {}
}
```

Workspace resolution rules:

- Relative `workspace` values are resolved under `HOME`
- Absolute `workspace` values are used directly
- Default `.chaos-bot` resolves to `~/.chaos-bot`

Logging rules:

- `logging.level`: `debug | info | warning | error` (`warning` maps to runtime `warn`)
- `logging.retention_days`: max days to keep dated log files (default `7`)
- `logging.directory`: relative path resolves under workspace (default `logs`)

Priority order:

1. Embedded defaults
2. Environment API keys (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`)
3. Config file values (`config.json` / legacy `agent.json`) as final override

`CHAOS_*` runtime environment variables are not used for config.

### Config Management API

- `GET /api/config`: read current running/disk config snapshot
- `POST /api/config/reset`: restore disk config to running snapshot
- `POST /api/config/apply`: hot-apply config (`raw` JSON or structured `config`)
- `POST /api/config/restart`: apply config then request process restart

Every config write rotates backups in-place:

- `<config_file>.bak1`
- `<config_file>.bak2`

## Logging

chaos-bot writes logs to both stdout and workspace log files:

- Log directory: `<workspace>/logs` by default
- Log filename: `YYYY-MM-DD.log`
- Writer model: async queue (non-blocking writer), flushed on process shutdown
- Retention: files older than `logging.retention_days` are removed on startup

Useful commands:

```bash
tail -f ~/.chaos-bot/logs/$(date +%F).log
ls -lah ~/.chaos-bot/logs
```

## Test Isolation (`.tmp`)

All test suites run in dedicated `.tmp` sandboxes and are deleted after execution:

- `make test-unit` -> `.tmp/unit`
- `make test-integration` -> `.tmp/integration`
- `make test-e2e` -> `.tmp/e2e`

e2e runtime files and Playwright artifacts are also redirected into `.tmp/e2e`.

## CI Failure Artifacts

GitHub Actions workflow: `.github/workflows/ci.yml`

- CI runs `make test-all` with `CHAOS_BOT_KEEP_TMP_ON_FAIL=1`.
- On failure, CI uploads these artifact directories:
  - `.tmp/unit`
  - `.tmp/integration`
  - `.tmp/e2e/runtime`
  - `.tmp/e2e/artifacts`
- Retention policy: 14 days.

This captures failure-time runtime evidence including:

- workspace logs (`.tmp/e2e/runtime/workspace/logs/*.log`)
- config and backups (`.tmp/e2e/runtime/home/.chaos-bot/config.json*`)
- Playwright report and traces (`.tmp/e2e/artifacts/*`)

## Runtime vs Source Files

Repository source-of-truth templates are tracked under `templates/`.
Runtime-generated files are stored under workspace (`~/.chaos-bot` by default).
Only test sandbox output is expected in repo-local `.tmp/`.

## Cleaning Runtime Files

To delete runtime-generated files and test temporary directories:

```bash
make clean-runtime
```
