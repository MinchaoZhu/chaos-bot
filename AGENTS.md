# PM Runtime AGENTS

## Current Status
- Project: bot
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/bot
- Active Task: task-13
- Last Updated: 2026-02-25T14:47:48+08:00

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
- task-11: done
- task-12: done
- task-13: done

## Verification
- `task-13` 后续收敛已完成：删除根级 `docs/` 与旧 `frontend/`，仅保留 `frontend-react/`。
- `cargo test --workspace --test api_routes --test api_integration` passed (10/10).
- `make test-e2e` passed (Playwright 2 passed, 2 skipped; react-shell desktop/mobile).
- `make test-all` passed (unit + integration + e2e, post-cleanup gate).
- `task-13` 已完成：`llm/tools` 下沉到 `infrastructure/{model,tooling}`，README 文档收敛，AGENTS 治理约束落地。
- `cargo test --workspace --test unit_llm --test unit_tools --test unit_agent` passed.
- `cargo test --workspace --test api_routes --test api_integration` passed (13/13).
- `make test-e2e` passed (Playwright 11 passed, 2 skipped; includes llm/tools port-adapter regression case).
- `make test-all` passed (unit + integration + e2e, task-13 completion gate).
- `task-12` 已完成：`llm/tools` 端口化下沉（application 仅依赖 domain ports）。
- `cargo test --workspace --test unit_agent --test unit_llm --test unit_tools` passed.
- `cargo test --workspace --test api_routes --test api_integration` passed (13/13).
- `make test-e2e` passed (Playwright 11 passed, 2 skipped; includes llm/tools port-adapter regression case).
- `make test-all` passed (unit + integration + e2e, task-12 completion gate).
- `task-11` full flat-file migration 已完成（13 个 legacy root file 全部迁移出 `backend/src` 顶层）。
- `cargo test --workspace --test unit_config --test unit_bootstrap --test unit_logging` passed.
- `cargo test --workspace --test unit_agent --test unit_tools --test unit_memory --test unit_sessions --test unit_types --test unit_personality` passed.
- `cargo test --workspace --test api_routes --test api_integration` passed (13/13).
- `make test-e2e` passed (Playwright 10 passed, 2 skipped; includes modular-backend regression case).
- `make test-all` passed (unit + integration + e2e, task-11 completion gate).
- `make tauri-preflight` passed (`webkit2gtk-4.1`/`rsvg2`/Rust toolchain detected).
- `make tauri-build-desktop` passed (debug desktop binary generated at `src-tauri/target/debug/chaos-bot-app`).
- `make tauri-android-init` passed (Android project generated under `src-tauri/gen/android` with local SDK/NDK + Java 21).
- `make tauri-android-build` passed (debug universal APK generated at `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`).

## Mandatory Rules
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
- `.pm/bot/task-11.md`: Agent 后端模块化架构重构计划（已完成，legacy 平铺文件全量迁移）。
- `.pm/bot/task-12.md`: LLM/Tools 端口化下沉重构计划（已完成）。
- `.pm/bot/task-13.md`: LLM/Tools 下沉 + README 文档收敛 + DDD 治理约束重构计划。
- `AGENTS.md`: Shared runtime status, task index, and verification summary.
- `CLAUDE.md`: Symlink to `AGENTS.md`.

## Next Actions
1. 规划 `task-14`（如需新增功能）并确保继续遵守 DDD 边界与 README 单一文档入口。
2. 如需继续收敛 e2e，可拆分 desktop/mobile spec 以去除跨项目 `skip`。
3. 持续以 `make test-all` 作为所有后续任务完成门禁。
