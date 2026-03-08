# task-4: Release UX And Commit Policy Tightening

## Task
- Description: Summarize and record the completed follow-up work that made releases more user-facing, tightened version/publish constraints, and enforced changelog-safe commit formatting.
- Scope: `README.md`, `AGENTS.md`, `.github/workflows/ci.yml`, `scripts/release/*`, version manifests, and `.githooks/pre-push`.
- Risk: Medium. These changes affect release operator workflow, push behavior, and the repository's required commit discipline.
- Status: done

## Phase 1: User-Facing Release Documentation
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Move end-user install, run, and upgrade guidance to the first section of the README. | done | Reworked `README.md` so GitHub installation, runtime usage, and upgrade flow appear before architecture/development material. |
| 1.2 | Document GitHub release download naming and upgrade behavior from the packaged Linux bundle. | done | Added GitHub Releases download examples, launcher/install layout details, and `GET /api/upgrade` plus `POST /api/upgrade/apply` usage guidance. |
| 1.v1 | Verify: user documentation describes install, use, and upgrade paths before architecture/development sections. | done | Confirmed the README starts with a dedicated user guide and keeps architecture plus development guidance later in the document. |

## Phase 2: Versioning And Release Output Constraints
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Enforce that every push updates the repository `VERSION` manifest. | done | Added push-time version-change validation in `scripts/release/validate-version-sync.sh` and wired it into `.github/workflows/ci.yml`; the repo version was bumped from `0.1.0` to `0.1.1` as part of the change. |
| 2.2 | Align GitHub release titles and release asset stems to use the release version directly. | done | Updated release metadata/version scripts so the GitHub Release title is the computed release version and Linux bundle assets use `<release-version>-linux-x86_64.*` naming consistently. |
| 2.v1 | Verify: release checks and packaging/updater flows still pass after the naming and version-gate changes. | done | `make release-check`, `make package-verify`, and `make upgrade-verify` passed on 2026-03-08 after rerunning bundle verification serially outside the sandbox. |

## Phase 3: Commit Message Policy And Hook Enforcement
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Reduce allowed release-note commit categories to three explicit prefixes. | done | Documented `Feature:`, `Fix:`, and `Refactor:` as the only allowed commit subject prefixes in `README.md` and `AGENTS.md`. |
| 3.2 | Enforce the prefix policy through a versioned repository push hook. | done | Added `.githooks/pre-push`, enabled `core.hooksPath=.githooks` in the worktree, and validated that the hook rejects legacy subjects such as `docs(cicd): ...`. |
| 3.3 | Record the completed implementation history for these workflow changes. | done | Relevant commits include `a823386` (`feat(cicd): tighten version release flow`), `b55f851` (`docs(cicd): define changelog commit rules`), and `693b740` (`Refactor: enforce commit prefix policy`), followed by merge commits on `master`. |
| 3.v1 | Verify: documented rules and local hook behavior match the intended changelog-generation constraints. | done | The repository docs now require the three-prefix policy, and the local pre-push hook enforces it on all commits in the push range. |
