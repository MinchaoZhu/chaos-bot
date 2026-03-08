# PM Runtime AGENTS

## Current Status
- Project: config
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/config
- Active Task: none
- Last Updated: 2026-03-08T20:13:24+08:00

## Task Index
- task-1: done
- task-2: done

## Verification
- Dedicated git worktree created at `/home/debian/projects/chaos-bot/.projects/config` on branch `feat/config`.
- PM runtime bootstrap completed for the config project.
- Config UX was restructured into top-level `Conversation` / `Sessions` / `Events` / `Config` / `Skills` / `About` panes.
- Config editing now uses `LLM`, `Search`, `IM Connectors`, `System`, and `Raw` sub-tabs, with Telegram moved into config and upgrade details moved into About.
- `npm --prefix frontend-react run build` and `npm --prefix frontend-react run test:unit` both passed on 2026-03-08.

## PM File Map
- .pm/docs/project.md: Config project requirements, worktree details, and planning constraints.
- .pm/docs/AGENTS.md: Mirror of the runtime status file for docs sync.
- .pm/config/task-1.md: Completed bootstrap task for the config project worktree.
- .pm/config/task-2.md: Completed navigation/config/about frontend restructure task.
- AGENTS.md: Shared runtime status source.
- CLAUDE.md: Symlink to AGENTS.md.

## Next Actions
1. Smoke-test the new pane flow in `frontend-react` against a live backend runtime.
2. Add UI-level automated coverage for tab switching and config editing if this surface will keep growing.
3. Keep config-related work isolated to this `feat/config` worktree.
