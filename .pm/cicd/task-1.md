# task-1: GitHub CI/CD Release Foundation

## Task
- Description: Define and implement the GitHub Actions release pipeline that validates the repo, versions releases, and publishes artifacts when `master` updates.
- Scope: `.github/workflows/`, release/version scripts or config, release documentation updates, and PM tracking records.
- Risk: Medium. Release triggers, version source of truth, and artifact publishing rules can conflict with current branch and tagging behavior.
- Status: done

## Phase 1: Release Strategy And Versioning Design
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Audit the current CI workflow, branch triggers, and existing version sources across Rust, frontend, and Tauri packages. | done | Confirmed CI only targeted `main`/`feat/**`, while backend/frontend/Tauri each carried `0.1.0` independently. |
| 1.2 | Choose and document the release versioning strategy for `master` publishes, including whether releases are tag-driven, branch-driven, or both. | done | Added root `VERSION` as the base SemVer source and documented derived release tags as `v<base>-master.<commit-count>`. |
| 1.3 | Define the release artifact set and publish destinations for GitHub releases. | done | Task-1 publishes release metadata, notes, and checksum assets to GitHub Releases; installable bundles remain in task-2. |
| 1.v1 | Verify: documented release strategy covers `master` trigger behavior, SemVer ownership, and artifact naming. | done | `README.md` and workflow/scripts now encode the `master` publish trigger, shared base version, and release tag format. |

## Phase 2: GitHub Actions Publish Pipeline
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Update CI workflow(s) so `master` pushes run the full gate and release jobs in a controlled sequence. | done | Updated `.github/workflows/ci.yml` so `master` pushes run tests first, then a gated release job. |
| 2.2 | Implement version computation or validation logic used by the release job. | done | Added `VERSION`, `scripts/release/version.sh`, and `scripts/release/validate-version-sync.sh` to compute and validate release data. |
| 2.3 | Publish release artifacts and metadata to GitHub Releases. | done | Added metadata/note/checksum generation and GitHub Release publishing via `softprops/action-gh-release`. |
| 2.v1 | Verify: workflow passes in CI and a dry-run or local validation confirms release job conditions. | done | `make release-check` passed locally, generating release metadata and checksum outputs under `.tmp/release`. |

## Phase 3: Documentation And End-To-End Verification
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Update `README.md` with the supported release/versioning flow and operator steps. | done | Added a CI/CD section covering `VERSION`, `make release-check`, `master` release behavior, and tag format. |
| 3.2 | Add or update automated coverage for release-related scripts/config where feasible. | done | Added `make release-check` and wired it into CI before the full test gate to fail fast on release config drift. |
| 3.3 | Execute mandatory verification, including `make test-all`, before marking the task done. | done | `make test-all` passed after rerunning outside the sandbox so e2e servers could bind to local ports. |
| 3.v1 | Verify: `make test-all` passes and PM records capture the release pipeline outcome. | done | `make test-all` completed successfully on 2026-03-08; PM records synced with the release foundation outcome and remaining packaging/upgrade work. |
