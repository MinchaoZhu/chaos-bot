# PM Runtime AGENTS

## Current Status
- Project: cli
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/cli-first
- Active Task: task-8
- Last Updated: 2026-03-13T02:37:04+08:00

## Task Index
- task-1: done
- task-2: done
- task-3: done
- task-4: done
- task-5: done
- task-6: done
- task-7: done
- task-8: done

## Verification
- Completed `task-6` by removing the HTTP server, Telegram/channel dispatcher layers, `backend/src/interface/`, and the `chaos-bot-backend` compatibility binary so the runtime is pure CLI only.
- Replaced the in-memory session store with file-backed persistence under `{workspace}/sessions/*.json` and redesigned chat/session flows around cross-invocation continuation.
- Unified runtime bootstrap in [`/home/debian/projects/chaos-bot/.projects/cli/backend/src/runtime/mod.rs`](/home/debian/projects/chaos-bot/.projects/cli/backend/src/runtime/mod.rs), merging state into `AppContext` and reusing a single `SkillStore`.
- Split the CLI entrypoint into [`/home/debian/projects/chaos-bot/.projects/cli/backend/src/runtime/cli/mod.rs`](/home/debian/projects/chaos-bot/.projects/cli/backend/src/runtime/cli/mod.rs) plus per-command modules and shared output formatting in [`/home/debian/projects/chaos-bot/.projects/cli/backend/src/runtime/cli/output.rs`](/home/debian/projects/chaos-bot/.projects/cli/backend/src/runtime/cli/output.rs).
- Updated packaging, installer, self-upgrade, CI, Makefile, and README for the single-binary CLI release layout.
- Verified task-6 with `cargo check -p chaos-bot-backend`, `cargo test -p chaos-bot-backend`, `make release-check`, `make test-all`, `make package-verify`, `make install-verify`, and `make upgrade-verify`.
- Completed `task-7` by routing tracing exclusively to rotating log files, keeping successful CLI stdout/stderr clean, and adding CLI regression coverage that proves logs still land under `{workspace}/logs/*.log`.
- Tightened `chat` continuation semantics so positional UUID-like or truncated session identifiers fail fast with `not_found` plus a `--session <ID>` hint instead of silently creating a new session, while explicit `--session <ID>` still supports named-session creation/continuation.
- Strengthened agent/runtime tool guidance by appending live-local-state instructions to the system prompt, clarifying `bash` tool examples such as `date` and `pwd`, and updating the default personality template for new workspaces.
- Verified task-7 with `cargo test -p chaos-bot-backend --test unit_logging -- --nocapture`, `cargo test -p chaos-bot-backend --test unit_tools -- --nocapture`, `cargo test -p chaos-bot-backend --test unit_agent -- --nocapture`, `cargo test -p chaos-bot-backend --test cli_integration -- --nocapture`, `cargo test -p chaos-bot-backend`, and `make test-all`.
- Advanced `task-8` by auditing the full test surface, adding high-value unit coverage for CLI output/config/runtime and config/session/upgrade services, and extending CLI integration scenarios for `config --stdin`, `config restart`, and `skills get`.
- Added `scripts/check-coverage.sh`, `make quality-gate`, and a nightly `make coverage-branch-check` entrypoint; verified `make quality-gate` and `cargo llvm-cov --workspace --summary-only` with total line coverage at `87.76%`.
- Installed nightly locally and confirmed the remaining blocker is upstream tooling: `make coverage-branch-check` reproduces a nightly `llvm-cov report/export` SIGSEGV before branch coverage can be summarized, so branch >= 85% is not yet enforceable despite the explicit failing entrypoint.
- Resolved nightly `llvm-cov` SIGSEGV by replacing the report/export step with `grcov` LCOV branch parsing in `scripts/check-coverage.sh`; `make coverage-branch-check` now passes with branch coverage 95.90% >= 85%.

## Quality Standards
- 每次迭代只要改动了行为、接口、输出契约、持久化、脚本或工具链，就必须同步补齐受影响范围的单元测试与集成测试，不能只改代码不补验证。
- 任务完成前至少要覆盖：happy path、error path、boundary path，以及文本/JSON/JSONL 等受影响输出契约路径；这部分作为“场景覆盖率”的最低要求。
- 核心 CLI 流程的场景覆盖必须覆盖 `chat`、`sessions`、`config`、`skills`、`upgrade` 以及相关 logging/tooling/runtime 行为中的受影响链路。
- 默认质量门禁必须可执行，并要求完整测试套件通过；覆盖率目标为 line coverage >= 85% 且 branch coverage >= 85%。如果当前工具链暂未完全满足，该缺口必须作为显式任务跟踪，不能静默跳过。
- `AGENTS.md` 与 `.pm/docs/AGENTS.md` 中的质量规范必须同步维护；后续新任务或任务收尾都要显式对照这些标准验收。

## PM File Map
- .pm/docs/project.md: CLI-first project context, requirements, constraints, and worktree metadata.
- .pm/docs/AGENTS.md: Mirror of the runtime status file for docs sync.
- .pm/cli/task-1.md: Completed code audit and migration planning baseline for the CLI-first project.
- .pm/cli/task-2.md: Completed CLI runtime bootstrap, command parser, and shared app-context extraction.
- .pm/cli/task-3.md: Completed command-surface implementation for sessions, chat, config, skills, upgrade, channels, and CLI automation contracts.
- .pm/cli/task-4.md: Completed verification, packaging, and release refactor away from GUI assumptions.
- .pm/cli/task-5.md: Completed frontend/Tauri removal and final repository cleanup.
- .pm/cli/task-6.md: Completed pure-CLI simplification: no HTTP server, no Telegram/channels, persisted sessions, unified runtime bootstrap, modular CLI, and CLI-only release/docs/test flows.
- .pm/cli/task-7.md: Completed CLI polish for file-only logging, explicit session targeting semantics, and stronger local-runtime tool guidance.
- .pm/cli/task-8.md: Completed whole-repo test audit, coverage uplift, scenario coverage expansion, and grcov workaround for nightly branch-coverage gate.
- AGENTS.md: Shared runtime status source.
- CLAUDE.md: Symlink to AGENTS.md.

## Next Actions
1. Promote `make coverage-branch-check` from informational tracking to a required CI gate now that grcov workaround is stable.
2. Continue raising targeted coverage in `chat_service`, `runtime/cli/upgrade`, and `infrastructure/model` when those paths change.
3. Revisit the grcov workaround when LLVM upstream fixes the multi-object branch SIGSEGV (track llvm-project issues).
