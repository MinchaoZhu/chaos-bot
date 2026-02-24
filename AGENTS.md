# PM Runtime AGENTS

## Current Status
- Project: bot
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/bot
- Active Task: task-7
- Last Updated: 2026-02-24T22:30:09+08:00

## Task Index
- task-1: done
- task-2: done
- task-3: done
- task-4: done
- task-5: done
- task-6: done
- task-7: todo

## Verification
- `cargo test --workspace --test unit_bootstrap --test unit_config` passed (10/10).
- `cargo test --workspace --test api_integration --test api_routes` passed (12/12).
- `make test-e2e` passed (Playwright 5/5).
- `make test-all` passed (unit + integration + e2e).

## Mandatory Rules
- Every task must run `make test-all` before it can be marked complete.
- Every new feature must include a dedicated e2e testing phase in its task plan.

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
1. 启动 `task-7` Phase 1，完成日志配置 schema（级别/保留天数/目录）与默认值。
2. 设计并实现日志队列写入与 retention 清理策略，补充单测覆盖。
3. 将日志规范写入 `AGENTS.md` 并完成 `make test-all` 最终门禁。
