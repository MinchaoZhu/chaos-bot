# chaos-bot

A personal AI agent assistant.

## Overview

chaos-bot is a personal AI agent designed to assist with everyday tasks through natural conversation. It leverages large language models to understand context, use tools, and take actions on your behalf.

## Common Commands

```bash
make build          # cargo build -p chaos-bot-backend
make run            # cargo run -p chaos-bot-backend
make clean-runtime  # delete personality/, memory/, data/, MEMORY.md
```

## Runtime vs Source Files

The following directories and files are **runtime-generated** and must never be committed to git:

| Path | Description |
|------|-------------|
| `personality/` | Active personality files written on first boot |
| `memory/` | Session memory logs |
| `data/` | Session data (e.g. `data/sessions/`) |
| `MEMORY.md` | Long-term memory file |

These are all listed in `.gitignore`. If they appear in `git status`, something is wrong.

## Template Prototypes

Canonical templates live in `templates/` and **are** tracked by git:

```
templates/
  personality/
    SOUL.md
    IDENTITY.md
    USER.md
    AGENTS.md
  MEMORY.md
```

These are compiled into the binary via `include_str!` in `backend/src/bootstrap.rs`. On first
startup, if `personality/` does not exist, the binary writes out the defaults automatically.

## Cleaning Runtime Files

To delete all runtime-generated files and restore a clean state:

```bash
make clean-runtime
```

or directly:

```bash
bash scripts/clean-runtime.sh
```

Run this before tests or CI runs to ensure startup from a known-clean state.

## Modifying Personality

There are two ways to change the bot's personality:

1. **Edit `templates/personality/*.md`** — affects the next compiled binary and any fresh
   deployment. Changes are tracked in git and shared with the team.

2. **Edit `personality/*.md`** (runtime files) — affects the current running instance only.
   Changes are local and not committed to git.
