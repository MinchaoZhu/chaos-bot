# PM Runtime AGENTS

## Current Status
- Project: bot
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/bot
- Active Task: task-11
- Last Updated: 2026-02-25T02:24:18+08:00

## Task Index
- task-1: done
- task-2: done
- task-3: done
- task-4: done
- task-5: done
- task-6: done
- task-7: done
- task-8: done
- task-9: done
- task-10: done
- task-11: todo

## Verification
- `cargo test --workspace --test unit_config` passed (7/7).
- `cargo test --workspace --test api_routes --test api_integration` passed (13/13).
- `cargo test --workspace --test unit_bootstrap --test unit_logging --test unit_agent --test unit_llm --test unit_tools` passed (84/84).
- `make test-e2e` passed (Playwright 9 passed, 2 skipped; legacy + react-shell desktop/mobile).
- `make test-all` passed (unit + integration + e2e, task-10 completion run).
- `make tauri-preflight` passed (`webkit2gtk-4.1`/`rsvg2`/Rust toolchain detected).
- `make tauri-build-desktop` passed (debug desktop binary generated at `src-tauri/target/debug/chaos-bot-app`).
- `make tauri-android-init` passed (Android project generated under `src-tauri/gen/android` with local SDK/NDK + Java 21).
- `make tauri-android-build` passed (debug universal APK generated at `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`).

## Mandatory Rules
- Every task must run `make test-all` before it can be marked complete.
- Every new feature must include a dedicated e2e testing phase in its task plan.

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
- `.pm/docs/project.md`: Project context (repo path, branch, last update).
- `.pm/docs/AGENTS.md`: PM runtime status mirror for docs sync.
- `.pm/bot/`: Project-scoped task directory for `bot`.
- `.pm/bot/task-1.md`: Bootstrap scaffold execution history (completed).
- `.pm/bot/task-2.md`: Verification framework task plan and completion record.
- `.pm/bot/task-3.md`: Dependency injection refactor plan and completion record.
- `.pm/bot/task-4.md`: Agent JSON config refactor task plan and completion record.
- `.pm/bot/task-5.md`: Runtime 资源内嵌初始化与测试 `.tmp` 隔离计划。
- `.pm/bot/task-6.md`: Workspace 重构与 runtime 物化路径切换计划（默认 `~/.chaos-bot`）。
- `.pm/bot/task-7.md`: Workspace 日志队列、保留策略、关键点日志与规范落地计划。
- `.pm/bot/task-8.md`: Config 唯一来源约定、配置中心 UI、默认配置物化、备份轮转与 reset/apply/restart 计划。
- `.pm/bot/task-9.md`: CI 失败现场 artifact 保留与下载策略。
- `.pm/bot/task-10.md`: Tauri v2 + React 多平台前端重构计划。
- `.pm/bot/task-11.md`: Agent 后端模块化架构重构计划。
- `AGENTS.md`: Shared runtime status, task index, and verification summary.
- `CLAUDE.md`: Symlink to `AGENTS.md`.

## Next Actions
1. 启动 `task-11` Phase 1，冻结后端模块边界与迁移顺序。
2. 在具备 Linux 桌面依赖 + Android SDK/JDK 的构建机复跑 `tauri` 打包链路并产出安装包。
3. 评估 iOS 构建机（macOS + Xcode）以补齐移动发布矩阵。
