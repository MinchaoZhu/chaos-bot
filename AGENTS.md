# PM Runtime AGENTS

## Current Status
- Project: bot
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/bot
- Active Task: task-7
- Last Updated: 2026-02-24T22:46:04+08:00

## Task Index
- task-1: done
- task-2: done
- task-3: done
- task-4: done
- task-5: done
- task-6: done
- task-7: done

## Verification
- `cargo test --workspace --test unit_bootstrap --test unit_config --test unit_logging` passed (12/12).
- `cargo test --workspace --test unit_agent --test unit_llm --test unit_tools` passed (80/80).
- `cargo test --workspace --test api_integration --test api_routes` passed (12/12).
- `make test-e2e` passed (Playwright 5/5).
- `make test-all` passed (unit + integration + e2e).

## Mandatory Rules
- Every task must run `make test-all` before it can be marked complete.
- Every new feature must include a dedicated e2e testing phase in its task plan.

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
- `AGENTS.md`: Shared runtime status, task index, and verification summary.
- `CLAUDE.md`: Symlink to `AGENTS.md`.

## Next Actions
1. 评估 `task-8`：CI 中日志文件（失败现场）artifact 保留与下载策略。
2. 为日志系统增加可选字段白名单/黑名单，进一步统一敏感信息脱敏策略。
3. 将日志清理行为增加开关（默认保留当前自动清理策略）。
