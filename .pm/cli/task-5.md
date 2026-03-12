# task-5: Remove Frontend And Tauri Surfaces After CLI Parity

## Task
- Description: Delete the React/Tauri shell and related GUI-only assets once the CLI and automation layers cover the retained workflows.
- Scope: `frontend-react/`, `src-tauri/`, `e2e/`, SPA fallback serving, make targets, generated reports, and final repository cleanup.
- Risk: High. GUI files are still referenced by runtime serving, packaging, version sync, and docs, so deletion must happen only after tasks 2-4 have landed.
- Status: done

## Phase 1: Confirm Deletion Readiness
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Check that CLI commands cover the workflows the GUI currently exposes: session management, chat streaming, config editing, skills, upgrade visibility, and channel status as needed. | done | Task-3/4 already moved these flows onto the shared CLI runtime, and `make test-all` now verifies the retained sessions/chat/config/skills/upgrade/channels command surface without GUI dependencies. |
| 1.2 | Confirm no retained script, release path, or version rule still references `frontend-react`, `src-tauri`, or `e2e`. | done | Cleared active references from `Makefile`, `README.md`, backend runtime/config code, and removed `scripts/run-e2e.sh`; release validation stayed CLI-only. |
| 1.v1 | Verify: A deletion checklist exists and passes before any GUI directories are removed. | done | Inventory/grep pass completed before deletion, covering runtime config hooks, HTTP fallback routing, Make targets, docs, and release verification scripts. |

## Phase 2: Remove GUI Code And Runtime Branches
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Delete `frontend-react/`, `src-tauri/`, `e2e/`, and their package/dependency artifacts once replacement CLI coverage is in place. | done | Deleted the React shell, Tauri bridge, Playwright suite, generated report artifacts, and `scripts/run-e2e.sh`. |
| 2.2 | Remove SPA/static-asset fallback handling from `backend/src/interface/http.rs` and any config fields or environment variables that only exist to serve the frontend. | done | Removed `AppState.frontend_dist`, `AppConfig.frontend_dist`, `CHAOS_BOT_FRONTEND_DIST`, the SPA fallback router branch, and frontend-dist test coverage; unknown non-API routes now return `404`. |
| 2.3 | Prune GUI-only make targets, docs, and compatibility glue introduced solely for the migration window. | done | Dropped frontend/Tauri targets from `Makefile` and rewrote README language so `serve` is documented as API-only compatibility. |
| 2.v1 | Verify: The repository tree, build graph, and runtime config no longer include GUI-only paths. | done | Passed `cargo test --workspace --test api_routes`, `make release-check`, and final tree/grep inspection after the deletions. |

## Phase 3: Finish Repository Simplification
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Tighten the remaining docs and architecture notes so the backend plus CLI story is the only supported path. | done | README now describes a backend + CLI product, removes GUI prerequisites, and explicitly states that `serve` no longer hosts static frontend assets. |
| 3.2 | Run a final grep-based cleanup for stale frontend/Tauri names in scripts, manifests, comments, and release metadata. | done | Active runtime/build/docs references were removed; intentional residual matches are limited to PM history/work logs and release assertions that enforce the absence of `frontend_dist`. |
| 3.v1 | Verify: The repo presents a coherent CLI-first product with no GUI prerequisite for development, testing, or release. | done | Passed `make test-all`, `make package-verify`, `make install-verify`, and `make upgrade-verify` after the cleanup. |
