# PM Runtime AGENTS

## Current Status
- Project: cli
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/cli-first
- Active Task: task-6
- Last Updated: 2026-03-13T01:22:15+08:00

## Task Index
- task-1: done
- task-2: done
- task-3: done
- task-4: done
- task-5: done
- task-6: done

## Verification
- Completed `task-6` by removing the HTTP server, Telegram/channel dispatcher layers, `backend/src/interface/`, and the `chaos-bot-backend` compatibility binary so the runtime is pure CLI only.
- Replaced the in-memory session store with file-backed persistence under `{workspace}/sessions/*.json` and redesigned chat/session flows around cross-invocation continuation.
- Unified runtime bootstrap in [`/home/debian/projects/chaos-bot/.projects/cli/backend/src/runtime/mod.rs`](/home/debian/projects/chaos-bot/.projects/cli/backend/src/runtime/mod.rs), merging state into `AppContext` and reusing a single `SkillStore`.
- Split the CLI entrypoint into [`/home/debian/projects/chaos-bot/.projects/cli/backend/src/runtime/cli/mod.rs`](/home/debian/projects/chaos-bot/.projects/cli/backend/src/runtime/cli/mod.rs) plus per-command modules and shared output formatting in [`/home/debian/projects/chaos-bot/.projects/cli/backend/src/runtime/cli/output.rs`](/home/debian/projects/chaos-bot/.projects/cli/backend/src/runtime/cli/output.rs).
- Updated packaging, installer, self-upgrade, CI, Makefile, and README for the single-binary CLI release layout.
- Verified task-6 with `cargo check -p chaos-bot-backend`, `cargo test -p chaos-bot-backend`, `make release-check`, `make test-all`, `make package-verify`, `make install-verify`, and `make upgrade-verify`.

## PM File Map
- .pm/docs/project.md: CLI-first project context, requirements, constraints, and worktree metadata.
- .pm/docs/AGENTS.md: Mirror of the runtime status file for docs sync.
- .pm/cli/task-1.md: Completed code audit and migration planning baseline for the CLI-first project.
- .pm/cli/task-2.md: Completed CLI runtime bootstrap, command parser, and shared app-context extraction.
- .pm/cli/task-3.md: Completed command-surface implementation for sessions, chat, config, skills, upgrade, channels, and CLI automation contracts.
- .pm/cli/task-4.md: Completed verification, packaging, and release refactor away from GUI assumptions.
- .pm/cli/task-5.md: Completed frontend/Tauri removal and final repository cleanup.
- .pm/cli/task-6.md: Completed pure-CLI simplification: no HTTP server, no Telegram/channels, persisted sessions, unified runtime bootstrap, modular CLI, and CLI-only release/docs/test flows.
- AGENTS.md: Shared runtime status source.
- CLAUDE.md: Symlink to AGENTS.md.

## Next Actions
1. Treat `feat/cli-first` as the complete CLI-only runtime baseline and prepare the branch for review/merge.
2. If future integrations are needed, implement them as separate adapter binaries or plugins instead of reopening core runtime server mode.
3. Keep release verification centered on CLI contracts and persisted workspace state, not long-running process health checks.
