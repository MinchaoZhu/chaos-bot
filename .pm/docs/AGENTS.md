# PM Runtime AGENTS

## Current Status
- Project: bot
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/bot
- Active Task: task-5
- Last Updated: 2026-02-24T02:26:04+08:00

## Task Index
- task-1: done
- task-2: done
- task-3: done
- task-4: done
- task-5: done

## Verification
- `cargo test --workspace --test unit_bootstrap` passed (2/2).
- `cargo test --workspace --test unit_config --test unit_llm` passed (40/40).
- `cargo test --workspace --test api_integration --test api_routes` passed (12/12).
- `make test-e2e` passed (Playwright 4/4).
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
- `AGENTS.md`: Shared runtime status, task index, and verification summary.
- `CLAUDE.md`: Symlink to `AGENTS.md`.

## Next Actions
1. 评估是否对 `task-6` 拆分 CI 覆盖率与 artifact 保留策略。
2. 如需保留失败现场，增加可选开关控制 `.tmp` 清理行为（默认仍清理）。
3. 保持 PM 文件与 AGENTS 状态同步。
