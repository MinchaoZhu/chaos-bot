# PM Runtime AGENTS

## Current Status
- Project: interaction
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/interaction
- Active Task: task-2
- Last Updated: 2026-02-26T01:25:59+08:00

## Task Index
- task-1: done
- task-2: done

## Verification
- `git worktree add /home/debian/projects/chaos-bot/.projects/interaction -b feat/interaction master` passed.
- `pm_bootstrap.sh interaction /home/debian/projects/chaos-bot feat/interaction` completed.
- Bootstrap artifacts verified: `AGENTS.md`, `CLAUDE.md`, `.pm/docs/project.md`, `.pm/interaction/task-1.md`.
- `pm new-task` planning completed: added `.pm/interaction/task-2.md` (slash command roadmap).
- `npm --prefix frontend-react run test:unit` passed (`slash-parser` unit tests).
- `make test-e2e` passed (Playwright 4 passed, 0 skipped; desktop/mobile + slash commands).
- `make test-all` passed (unit + integration + e2e; task-2 completion gate).
- `AGENTS.md` and `.pm/docs/AGENTS.md` synchronized after task execution update.

## Mandatory Rules
- Every task must run `make test-all` before it can be marked complete.
- Every new feature must include a dedicated e2e testing phase in its task plan.

## PM File Map
- `.pm/docs/project.md`: Project context (repo/branch/mode/requirements/constraints).
- `.pm/docs/AGENTS.md`: Runtime status mirror for docs sync.
- `.pm/interaction/task-1.md`: Bootstrap execution record.
- `.pm/interaction/task-2.md`: Slash command implementation record (`/model`, `/models`, `/new`, `/compact`, etc.).
- `AGENTS.md`: Shared runtime status source.
- `CLAUDE.md`: Symlink to `AGENTS.md`.

## Next Actions
1. Plan `task-3` for slash command persistence/history and keyboard navigation enhancements.
2. Consider adding backend-supported model catalog endpoint to replace frontend whitelist.
3. Keep `make test-all` as mandatory completion gate for subsequent tasks.
