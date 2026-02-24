# chaos-bot

A personal AI agent assistant.

## Overview

chaos-bot is a personal AI agent designed to assist with everyday tasks through natural conversation. It leverages large language models to understand context, use tools, and take actions on your behalf.

## Common Commands

```bash
make build          # cargo build -p chaos-bot-backend
make run            # cargo run -p chaos-bot-backend
make clean-runtime  # delete runtime-generated files and .tmp
make test-all       # unit + integration + e2e (all in .tmp, auto-cleaned)
```

## Runtime Initialization Model

Runtime config and templates are embedded into the backend binary at compile time:

- `templates/config/agent.json`
- `templates/config/.env.example`
- `templates/MEMORY.md`
- `templates/personality/*.md`

At runtime, missing files are materialized automatically:

- `agent.json`
- `.env.example`
- `MEMORY.md`
- `personality/SOUL.md`
- `personality/IDENTITY.md`
- `personality/USER.md`
- `personality/AGENTS.md`
- `data/sessions/`

Existing files are preserved; only missing files are generated.

## Runtime Configuration (`agent.json`)

`agent.json` is runtime-generated from the embedded template if missing.

```json
{
  "server": { "host": "0.0.0.0", "port": 3000 },
  "llm": { "provider": "openai", "model": "gpt-4o-mini" },
  "paths": {
    "working_dir": ".",
    "personality_dir": "./personality",
    "memory_dir": "./memory",
    "memory_file": "./MEMORY.md"
  },
  "secrets": {}
}
```

Priority order:

1. Embedded defaults
2. Environment secrets (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GEMINI_API_KEY`)
3. `agent.json` secrets (final override)

`CHAOS_*` runtime environment variables are not used.
You can override config path with `AGENT_CONFIG_PATH`.

## Test Isolation (`.tmp`)

All test suites run in dedicated `.tmp` sandboxes and are deleted after execution:

- `make test-unit` -> `.tmp/unit`
- `make test-integration` -> `.tmp/integration`
- `make test-e2e` -> `.tmp/e2e`

e2e runtime files and Playwright artifacts are also redirected into `.tmp/e2e`.

## Runtime vs Source Files

Repository source-of-truth templates are tracked under `templates/`.
Runtime-generated files at repo root are ignored via `.gitignore`:

- `agent.json`
- `.env.example`
- `MEMORY.md`
- `memory/`
- `personality/`
- `data/`
- `.tmp/`

## Cleaning Runtime Files

To delete runtime-generated files and test temporary directories:

```bash
make clean-runtime
```
