# PM Runtime AGENTS

## Current Status
- Project: cicd
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/cicd
- Active Task: none
- Last Updated: 2026-03-08T18:31:09+08:00

## Task Index
- task-1: done
- task-2: done
- task-3: done
- task-4: done

## Verification
- Dedicated git worktree created at `/home/debian/projects/chaos-bot/.projects/cicd` on branch `feat/cicd`.
- Branch gate satisfied for task execution: after fetching `origin`, `feat/cicd`, local `master`, and `origin/master` all matched the same commit.
- Task-1 implemented a `master` release pipeline with shared `VERSION` validation, release metadata generation, and GitHub Release publishing.
- Task-2 added a Linux `x86_64` install bundle, bundled-frontend serving via `CHAOS_BOT_FRONTEND_DIST`, release manifest/checksum assets, and GitHub workflow packaging/release wiring.
- Task-3 added Linux bundle self-upgrade discovery/install APIs, frontend upgrade controls, package metadata/launcher env support, updater unit coverage, and a staged release smoke test for relaunch-based upgrades.
- Task-4 reorganized the README around user installation and upgrade flows, enforced push-time `VERSION` bumps plus version-named release outputs, and added a versioned pre-push hook for `Feature:` / `Fix:` / `Refactor:` commit subjects.
- `make release-check`, `make package-verify`, `make upgrade-verify`, and `make test-all` all passed on 2026-03-08; install/upgrade verification and the full gate were rerun outside the sandbox for local binary execution and e2e port binding.

## PM File Map
- `.pm/docs/project.md`: Project context, requirements, constraints, and planning assumptions for the CI/CD initiative.
- `.pm/docs/AGENTS.md`: Runtime status mirror for docs sync.
- `.pm/cicd/`: Project-scoped task directory for the CI/CD workstream.
- `.pm/cicd/task-1.md`: Completed GitHub CI/CD release and versioning implementation record for `master` publishes.
- `.pm/cicd/task-2.md`: Completed installable frontend/backend packaging implementation and verification record.
- `.pm/cicd/task-3.md`: Completed self-upgrade implementation and verification record for installed Linux bundles.
- `.pm/cicd/task-4.md`: Completed release UX, version gate, and commit policy tightening summary for the post-upgrade follow-up commits.
- `AGENTS.md`: Shared runtime status source.
- `CLAUDE.md`: Symlink to `AGENTS.md`.

## Next Actions
1. Decide whether the commit-prefix policy should also be enforced in CI or `commit-msg` hooks, not only in the versioned `pre-push` hook.
2. If cross-platform installers are required, generalize the release metadata, asset naming, and updater discovery rules beyond the current Linux `x86_64` bundle.
3. Preserve the `VERSION` source-of-truth approach while evolving release channels, updater policy, and asset compatibility rules.

## Commit Rules
- Only three commit subject prefixes are allowed: `Feature:`, `Fix:`, and `Refactor:`.
- Every commit message must start with one of those exact prefixes, followed by a concise one-line summary.
- The repository pre-push hook at `.githooks/pre-push` rejects pushes containing commits that do not match that rule.
- New clones and worktrees must enable the versioned hook path with `git config core.hooksPath .githooks`.
- If a commit changes compatibility, upgrade behavior, or operator workflow, the commit body must explain the impact so generated changelogs stay usable.
