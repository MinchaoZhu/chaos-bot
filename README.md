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
make frontend-dev   # run React shell dev server (http://127.0.0.1:1420)
make tauri-dev      # run Tauri v2 desktop app shell
```

## Tauri v2 + React (Phase 1 Foundation)

The repo uses a unified multi-platform frontend scaffold:

- `frontend-react/`: Vite + React + TypeScript shell UI.
- `src-tauri/`: Tauri v2 runtime crate and invoke command bridge.
- Runtime contract: `frontend-react/RUNTIME_CONTRACT.md`.

`frontend-react/` and Tauri shell use the same backend API (`/api/*`) and SSE stream protocol.

### Build/Run Entry Points

```bash
make frontend-install     # install frontend-react dependencies
make frontend-dev         # web shell dev mode
make frontend-build       # build React shell
make tauri-preflight      # check host prerequisites for Tauri desktop/mobile
make tauri-dev            # desktop shell
make tauri-build-desktop  # desktop debug build (no native bundle)
make tauri-android-init   # generate Android project scaffold
make tauri-android-dev    # Android dev shell (requires Android toolchain)
make tauri-android-build  # Android debug APK build
make tauri-ios-dev        # iOS dev shell (requires Xcode/macOS)
```

### Dependency Matrix

- Desktop: Rust toolchain + Node.js 20+ + Tauri CLI.
- Android: desktop requirements + Android SDK/NDK/JDK.
- iOS: desktop requirements + macOS + Xcode command line tools.

### Android Reusable Build Profile

Recommended host baseline (Debian trixie):

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  librsvg2-dev \
  libayatana-appindicator3-dev \
  build-essential \
  pkg-config \
  openjdk-21-jdk
```

Project-local Android SDK/NDK profile (keeps host clean and reproducible):

```bash
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64
export ANDROID_HOME="$PWD/.tmp/android-sdk"
export ANDROID_SDK_ROOT="$PWD/.tmp/android-sdk"
export ANDROID_NDK_HOME="$PWD/.tmp/android-sdk/ndk/26.3.11579264"
export NDK_HOME="$PWD/.tmp/android-sdk/ndk/26.3.11579264"
export PATH="$PWD/.tmp/android-sdk/cmdline-tools/latest/bin:$PWD/.tmp/android-sdk/platform-tools:$PATH"
```

With the profile above:

```bash
make tauri-android-init
make tauri-android-build
```

APK output (debug universal):

- `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`

### Adaptive Layout (Phase 2)

`frontend-react` now uses a shared component architecture with a layout adapter:

- `src/layout/adapter.ts`: viewport-driven desktop/mobile mode selection.
- `src/components/SessionRail.tsx`: session list + runtime controls.
- `src/components/ConversationPanel.tsx`: message timeline + composer.
- `src/components/EventTimeline.tsx`: streamed event log + runtime error surface.
- `src/components/MobilePaneTabs.tsx`: mobile pane switcher (`chat/sessions/events`).

Desktop uses a multi-panel landscape layout. Mobile uses a single-column portrait flow
with pane switching, while reusing the same runtime contract and business actions.

### Packaging Matrix (Phase 3)

| Target | Purpose | Command |
|---|---|---|
| Desktop preflight | Verify host dependencies and toolchain | `make tauri-preflight` |
| Desktop build | Compile Tauri app without native bundling | `make tauri-build-desktop` |
| Android init | Generate Android project (`src-tauri/gen/android`) | `make tauri-android-init` |
| Android debug build | Build universal debug APK | `make tauri-android-build` |
| Android dev run | Run Android dev target | `make tauri-android-dev` |
| iOS dev run | Run iOS dev target (macOS only) | `make tauri-ios-dev` |

Linux desktop prerequisites (Debian/Ubuntu typical):

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  librsvg2-dev \
  libayatana-appindicator3-dev
```

Common desktop packaging failures:

- `pango-sys` / `gdk-sys` missing system libraries.
- Missing `PKG_CONFIG_PATH` when libraries are installed in non-standard locations.

Android prerequisites:

- Java 21 (`JAVA_HOME` configured), Android SDK/NDK, platform tools.
- `src-tauri/package.json` keeps Android Gradle `rustBuild*` tasks compatible by proxying `npm run tauri ...`.

iOS prerequisite:

- macOS + Xcode toolchain (Linux hosts cannot run iOS packaging chain).

Release signing inputs must come from CI environment secrets; no signing secrets are committed to repository.

Recommended CI progression:

1. `make tauri-preflight` + `make tauri-build-desktop`
2. `make tauri-android-init` + `make tauri-android-build`
3. `make tauri-ios-dev` on macOS runner

### Build Architecture Notes (Reusable)

- Tauri JS CLI is managed in `frontend-react/` (`@tauri-apps/cli` installed once there).
- `src-tauri/package.json` provides a bridge script for Android Gradle tasks that execute `npm run tauri ...`.
- This avoids duplicating Node dependencies inside `src-tauri/` while keeping Android `rustBuild*` tasks compatible.

## Backend Architecture

`README.md` is the maintained architecture and packaging reference.

task-11/task-12/task-13 completion status:

- Legacy backend root flat files were migrated into layered modules.
- Runtime binary entry moved to `backend/src/runtime/bin/chaos_bot_backend.rs`.
- LLM/tools adapters were migrated under `infrastructure::{model,tooling}`.

Current backend module layout:

```text
backend/src
  application/      # use cases (agent/chat/config/session)
  domain/           # core models, error, audit, shared types, ports contracts
  infrastructure/   # adapter implementations and runtime-facing infra
    model/          # ModelPort adapters (OpenAI/Anthropic/Gemini/Mock)
    tooling/        # ToolExecutorPort adapters (ToolRegistry + tool impls)
  interface/        # HTTP API/router and protocol mapping
  runtime/          # bootstrap, config runtime, app composition, binary entry
  lib.rs            # crate module exports
```

Dependency direction (frozen):

1. `interface -> application -> domain`
2. `runtime -> {application, interface, infrastructure}`
3. `application` only depends on `domain::ports` for model/tool/memory contracts
4. `domain` does not depend on upper layers
5. Cross-layer reverse dependencies are forbidden

Port/adapter chain:

- `application::agent` depends on `domain::ports::{ModelPort, ToolExecutorPort}` only.
- `infrastructure::model` acts as `ModelPort` adapters (OpenAI/Anthropic/Gemini/Mock).
- `infrastructure::tooling::ToolRegistry` acts as `ToolExecutorPort` adapter.
- `runtime` composes and injects concrete adapters into `AgentLoop`.

Request/runtime flow:

1. `runtime/bin/chaos_bot_backend.rs` loads config, initializes logging, and picks restart mode.
2. `runtime::build_app_with_config_runtime` composes `AgentLoop`, `ConfigRuntime`, and shared `AppState`.
3. `interface::api::router` builds the Axum router and binds handlers.
4. `interface::http` maps HTTP/SSE payloads to `application` commands.
5. `application` calls `domain` models and delegates concrete IO to `infrastructure`.

Startup sequence (high level):

1. Load `~/.chaos-bot/config.json` (or legacy `agent.json`) from `infrastructure::config`.
2. Initialize daily log file and retention cleanup via `infrastructure::logging`.
3. Bootstrap workspace personality/memory/session directories via `runtime::bootstrap`.
4. Build provider (`infrastructure::model`), tool registry (`infrastructure::tooling`), and agent loop (`application::agent`).
5. Start Axum server and serve `/api/*` and UI static assets.

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

Current e2e matrix (`make test-e2e`):

- `react-shell-desktop`: React shell desktop landscape flow.
- `react-shell-mobile`: React shell mobile portrait flow (Playwright device emulation).

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
