# Project Context

- Project: interaction
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/interaction
- Mode: git worktree (default)
- Base Branch: master
- Action: new-project
- Requirements:
  - Initialize a dedicated PM runtime context for `interaction`.
  - Keep task planning files under `.pm/interaction/task-{n}.md`.
  - Keep `AGENTS.md`, `CLAUDE.md`, and `.pm/docs/AGENTS.md` synchronized.
- Technical Constraints:
  - Follow repository DDD boundaries and README single-doc entry rule.
  - Use `make test-all` as the completion gate for future tasks.
  - Any new feature task must include a dedicated e2e verification phase.
- Updated At: 2026-02-26T01:01:35+08:00
