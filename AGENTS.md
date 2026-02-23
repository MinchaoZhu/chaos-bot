# PM Runtime AGENTS

## Current Status
- Project: bot
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/bot
- Active Task: task-3
- Last Updated: 2026-02-23T14:52:57+08:00

## Task Index
- task-1: done
- task-2: done
- task-3: done

## Verification
- `cargo llvm-cov --workspace --summary-only --fail-under-lines 85` passed (TOTAL Lines 87.98%).
- `make test-e2e` passed (Playwright 4/4).
- `make test-all` passed (unit + integration + e2e).

## Mandatory Rules
- Every task must run `make test-all` before it can be marked complete.
- Every new feature must include a dedicated e2e testing phase in its task plan.

## PM File Map
- `.pm/docs/project.md`: Project context (repo path, branch, last update).
- `.pm/task-1.md`: Bootstrap scaffold execution history (completed).
- `.pm/task-2.md`: Verification framework task plan and completion record.
- `.pm/task-3.md`: Dependency injection refactor plan and completion record.
- `AGENTS.md`: Shared runtime status, task index, and verification summary.
- `CLAUDE.md`: Symlink to `AGENTS.md`.

## Next Actions
1. Create `task-4` for CI workflow automation (coverage gate + e2e in pipeline).
2. Decide whether to improve `backend/src/llm/mod.rs` coverage beyond current baseline.
3. Keep AGENTS/task files synchronized after each pm update.
