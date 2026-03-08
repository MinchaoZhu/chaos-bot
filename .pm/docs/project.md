# Project Context

- Project: config
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/config
- Worktree Mode: enabled
- Worktree Path: /home/debian/projects/chaos-bot/.projects/config
- Updated At: 2026-03-08T20:13:24+08:00

## Requirements

- This project is dedicated to config-related changes in the chaos-bot repository.
- Use this worktree to plan and execute configuration updates without mixing them with unrelated feature work.
- Keep PM runtime files current so follow-up `pm new-task`, `pm update-task`, and `pm run-task` commands can continue from local context.
- The current completed frontend task restructured navigation, config sub-tabs, and the About/version surface for config-related UX.

## Technical Constraints

- Repository: git worktree based workflow rooted at `/home/debian/projects/chaos-bot`.
- Branch target: `feat/config`.
- Planning commands only update PM metadata and task files; implementation belongs to later `pm run-task` execution.
- `AGENTS.md` is the shared runtime source of truth and `CLAUDE.md` must remain a symlink to it.
