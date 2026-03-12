# Project Context

- Project: cli
- Main Repository: /home/debian/projects/chaos-bot
- Worktree: /home/debian/projects/chaos-bot/.projects/cli
- Worktree Mode: enabled
- Base Branch: master
- Branch: feat/cli-first
- Updated At: 2026-03-13T01:22:15+08:00

## Requirements
- Refactor the project into a CLI-first product so code agents can develop and test features without GUI interference.
- Remove HTTP server, Telegram/channel integration, and other long-running runtime modes from the core product.
- Persist chat sessions on disk so CLI invocations can continue conversations safely across separate processes.

## Technical Constraints
- Keep backend crates and agent/domain logic intact while simplifying execution to a single-command CLI runtime.
- Prefer deterministic CLI commands and scriptable verification over interactive GUI flows.
- Future channel or network adapters, if needed, should live outside the core CLI as separate binaries or plugins.
