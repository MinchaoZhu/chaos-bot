# chaos-bot

`chaos-bot` is now a pure CLI runtime. Each invocation loads config, runs one command, writes to stdout/stderr, and exits. Sessions are persisted on disk under the workspace so conversation state survives across separate commands.

## Overview

- Runtime: Rust CLI only
- Binary: `chaos-bot`
- Workspace data:
  - `~/.chaos-bot/config.json`
  - `~/.chaos-bot/sessions/*.json`
  - `~/.chaos-bot/logs/`
  - `~/.chaos-bot/memory/`
  - `~/.chaos-bot/skills/`

## Install Latest Release

Install the latest GitHub release on Linux x86_64:

```bash
curl -fsSL https://raw.githubusercontent.com/MinchaoZhu/chaos-bot/master/scripts/install-from-github.sh | bash
```

Install into a custom prefix:

```bash
curl -fsSL https://raw.githubusercontent.com/MinchaoZhu/chaos-bot/master/scripts/install-from-github.sh | bash -s -- --prefix /usr/local
```

## Build And Run

```bash
cargo build -p chaos-bot-backend --bin chaos-bot
cargo run -p chaos-bot-backend -- --help
```

Useful shortcuts:

```bash
make build
make run
make test-all
make package-verify
```

## Core Commands

Show help:

```bash
chaos-bot --help
chaos-bot chat --help
chaos-bot sessions --help
```

One-shot chat:

```bash
chaos-bot chat "Summarize the current repository"
```

Continue an existing session:

```bash
chaos-bot sessions list
chaos-bot chat <SESSION_ID> "Follow up on the last answer"
```

Read chat input from stdin:

```bash
printf 'Explain the failing test' | chaos-bot chat --stdin
printf 'Continue from stdin' | chaos-bot chat --session <SESSION_ID> --stdin
```

Streaming output:

```bash
chaos-bot --output jsonl chat --stream "Stream this reply"
```

Inspect or delete sessions:

```bash
chaos-bot sessions list
chaos-bot sessions get <SESSION_ID>
chaos-bot sessions delete <SESSION_ID>
```

Config, skills, and upgrade:

```bash
chaos-bot config get
chaos-bot config apply --raw '{"llm":{"provider":"mock","model":"mock-model"}}'
chaos-bot skills list
chaos-bot upgrade status
```

## Global Flags

- `--config <PATH>`: load a specific config file
- `--workspace <PATH>`: override the workspace root for the current command
- `--output text|json|jsonl`: choose output contract
- `--non-interactive`: fail instead of prompting when required input is missing

## Config Shape

Default config file: `~/.chaos-bot/config.json`

```json
{
  "workspace": ".chaos-bot",
  "llm": {
    "provider": "openai",
    "model": "gpt-4o-mini",
    "temperature": 0.2,
    "max_tokens": 1024,
    "max_iterations": 6,
    "token_budget": 12000
  },
  "search": {},
  "logging": {
    "level": "info",
    "retention_days": 7,
    "directory": "logs"
  },
  "secrets": {}
}
```

## Release Verification

Release and installer checks are CLI-only:

```bash
make release-check
make package-verify
make install-verify
make upgrade-verify
```
