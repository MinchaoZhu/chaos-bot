# task-3: Self-Upgrade Flow

## Task
- Description: Implement a safe self-upgrade path so installed binaries can discover, download, and transition to newer GitHub releases.
- Scope: updater design, release metadata consumption, upgrade command/UI integration, rollback or failure handling, and verification records.
- Risk: High. Self-upgrade touches platform behavior, release trust, and failure recovery paths.
- Status: done

## Phase 1: Upgrade Contract And Safety Model
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Decide which runtime surfaces expose self-upgrade controls and status. | done | Chose backend-first HTTP endpoints (`GET /api/upgrade`, `POST /api/upgrade/apply`) with React config-panel controls; web and Tauri both consume the same HTTP contract. |
| 1.2 | Define release metadata, checksum verification, and trust model for downloads. | done | The updater now consumes GitHub Release assets, verifies `release-metadata.json` against `release-metadata.sha256`, derives the Linux bundle asset names from `artifact_stem`, and verifies the bundle tarball checksum before install. |
| 1.3 | Define failure handling, restart behavior, and rollback expectations. | done | Upgrades only target installed Linux bundles, install into a new versioned release directory under the same prefix, leave the currently running release intact on failure, and require an explicit relaunch instead of attempting an unsafe in-process hot swap. |
| 1.v1 | Verify: upgrade contract covers discovery, download, integrity checks, swap/restart, and failure recovery. | done | README, runtime contract, backend responses, and the staged self-upgrade smoke test now cover discovery, verification, install, relaunch guidance, and failure reporting. |

## Phase 2: Upgrade Implementation
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Implement release discovery and version comparison against installed binaries. | done | Added an infrastructure updater that discovers the latest GitHub Release, parses release metadata, and compares installed versus remote versions with `semver`. |
| 2.2 | Implement download, verification, and install/swap flow for the selected artifact format. | done | Added Linux bundle download, checksum verification, tarball unpack, and `install.sh --prefix` execution using the task-2 archive format and launcher/install prefix metadata. |
| 2.3 | Expose upgrade status, errors, and restart guidance in the relevant runtime surface. | done | Added backend upgrade endpoints, frontend config-panel controls/status text, launcher environment exports for installed bundles, and package metadata fields for repository/API discovery. |
| 2.v1 | Verify: upgrade implementation handles no-op, success, and checksum or download failure paths. | done | Library tests cover available-update, no-op, successful install, and checksum failure cases for the updater. |

## Phase 3: End-To-End Validation
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Add automated coverage for version comparison and upgrade failure handling. | done | Added updater-focused library tests plus route coverage for the upgrade endpoints. |
| 3.2 | Add a dedicated e2e scenario that validates upgrade UX or command flow against staged release metadata. | done | Added `scripts/release/verify-self-upgrade.sh` and `make upgrade-verify`, which stage a newer local release feed, drive `/api/upgrade` + `/api/upgrade/apply`, and relaunch the installed launcher to confirm the version transition. |
| 3.3 | Run the mandatory full gate before marking self-upgrade complete. | done | Ran `make test-all` after the updater changes. |
| 3.v1 | Verify: upgrade tests and `make test-all` pass, and PM records capture any residual platform gaps. | done | On 2026-03-08, `make release-check`, `make package-verify`, `make upgrade-verify`, and `make test-all` all passed; current self-upgrade support is intentionally limited to installed Linux release bundles. |
