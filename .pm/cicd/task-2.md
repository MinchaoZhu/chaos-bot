# task-2: Installable Artifact Packaging

## Task
- Description: Produce installable release artifacts that package the backend executable and frontend deliverables in a distribution model users can install consistently.
- Scope: build/release scripts, packaging manifests, installer/archive structure, frontend/backend asset composition, and release validation notes.
- Risk: Medium. Packaging differs by platform and must align with Tauri, backend runtime assets, and release automation from task-1.
- Status: done

## Phase 1: Packaging Design
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Inventory current build outputs for backend, frontend, and Tauri targets. | done | Confirmed the repo already produces a Rust backend binary, Vite `frontend-react/dist`, and a Tauri shell that still expects an external backend at `127.0.0.1:3000`. |
| 1.2 | Decide the supported artifact formats for initial release packaging. | done | Chose an initial `linux-x86_64` `.tar.gz` install bundle with a stable `${artifact_stem}-linux-x86_64` root and deferred native Tauri bundles until the desktop shell can launch the packaged backend itself. |
| 1.3 | Define installation layout and runtime asset placement for packaged builds. | done | Added a release installer layout under `share/chaos-bot/releases/<release_version>` with `bin/chaos-bot-backend`, bundled `frontend/`, and a `bin/chaos-bot` launcher that exports `CHAOS_BOT_FRONTEND_DIST`. |
| 1.v1 | Verify: packaging design names the exact contents of each released artifact. | done | The package manifest, installer, README release notes, and workflow now encode the exact archive root, backend binary path, frontend path, and launcher/install layout. |

## Phase 2: Build And Bundle Automation
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Add scripts or workflow steps to build frontend assets and backend binaries for release packaging. | done | Added `scripts/release/package-linux-x86_64.sh`, `make package-linux-x86_64`, and a dedicated CI `package` job plus release-job packaging step. |
| 2.2 | Implement bundle assembly for the chosen installable artifacts. | done | The package script now assembles `bin/`, `frontend/`, `VERSION`, `README.md`, `release-manifest.json`, `install.sh`, and a bundle `.sha256` for updater-facing metadata. |
| 2.3 | Wire packaging outputs into the GitHub release process. | done | The release workflow now uploads the Linux bundle, its checksum, and manifest alongside the existing release metadata assets. |
| 2.v1 | Verify: generated artifacts can be unpacked or installed and contain the expected FE+BE assets. | done | `scripts/release/verify-packaged-runtime.sh` unpacks the bundle, runs `install.sh`, launches the installed backend/launcher, and validates `/api/health` plus the packaged frontend shell over HTTP. |

## Phase 3: Verification Coverage
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Add automated tests or validation scripts for packaging outputs where deterministic checks are possible. | done | Added backend route coverage for static frontend serving and a deterministic package verification script for bundle/install/runtime checks. |
| 3.2 | Add a dedicated e2e verification phase for packaged runtime behavior. | done | Added `make package-verify`, which exercises the built archive as an installed runtime instead of only validating source-tree builds. |
| 3.3 | Run the mandatory full gate before marking packaging complete. | done | `make test-all` and `make package-verify` were executed locally after the packaging/runtime changes. |
| 3.v1 | Verify: packaged artifact validation and `make test-all` both succeed. | done | Local verification completed on 2026-03-08 with the package smoke test and the required full gate both passing. |
