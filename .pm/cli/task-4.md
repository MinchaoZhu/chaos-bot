# task-4: Rebuild Verification, Packaging, And Release Around CLI

## Task
- Description: Replace frontend- and Tauri-dependent automation with a CLI-native build, test, and release pipeline.
- Scope: Make targets, integration tests, release packaging, version sync rules, installer behavior, and developer docs.
- Risk: High. Current release scripts, version checks, and e2e flows explicitly build and package frontend assets, so partial migration would leave shipping artifacts in an inconsistent state.
- Status: done

## Phase 1: Replace GUI-Centric Verification
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Design a CLI-native test matrix that replaces Playwright and shell-UI smoke tests with Rust integration and command-level smoke coverage. | done | `Makefile` now promotes `test-unit` + `test-integration` + `test-cli`, with `test-cli` backed by Rust integration tests instead of Playwright. |
| 1.2 | Add or plan command-focused integration coverage for session lifecycle, chat streaming, config mutation, skill install, and upgrade diagnostics. | done | Added `backend/tests/cli_integration.rs` to smoke the real `chaos-bot` binary for sessions create/not-found, chat send/stream, config mutation, skills install/list/get, channels status, and upgrade diagnostics. |
| 1.3 | Redefine `make test`, `make test-all`, and related developer workflows so they can run in a CLI-only environment. | done | The required test gate no longer references Node, frontend install, or Playwright; `make test-all` is now Rust-only. |
| 1.v1 | Verify: The planned gate can execute on a machine with Rust and only the retained non-GUI dependencies. | done | `make test-all` passed. |

## Phase 2: Replace Frontend Packaging Assumptions
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Rewrite release packaging so it ships the CLI-first executable and runtime assets without bundling `frontend-react/dist`. | done | `scripts/release/package-linux-x86_64.sh` now packages `chaos-bot` and `chaos-bot-backend` only, and the installed release root no longer contains `frontend/`. |
| 2.2 | Update installer and self-upgrade metadata so release manifests describe the CLI layout rather than a backend-plus-frontend bundle. | done | Release manifests now describe CLI binaries and launchers, and the installer/self-upgrade verifiers exercise `upgrade status/apply/relaunch` via the installed CLI. |
| 2.3 | Remove frontend/Tauri entries from version sync logic and define the retained source of truth for release versioning. | done | Version sync validation and `.githooks/pre-push` now enforce `VERSION` against `backend/Cargo.toml` only; packaging also supports `CARGO_TARGET_DIR` for sandbox-safe release builds. |
| 2.v1 | Verify: Packaging and versioning plans are internally consistent without `frontend-react` or `src-tauri`. | done | Version sync, release metadata generation, packaged runtime verification, GitHub installer verification, and self-upgrade verification all passed. |

## Phase 3: Update Project Documentation And Delivery Contracts
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Rewrite README and developer instructions around CLI startup, automation, and release verification. | done | `README.md` now describes the CLI-first runtime, CLI-only test gate, Linux bundle layout, and installed upgrade flow before any compatibility details. |
| 3.2 | Audit hooks, CI assumptions, and helper scripts for references to GUI artifacts or Node/Tauri setup. | done | CI no longer installs Node/Playwright for required gates, and the pre-push hook plus release helper scripts no longer require frontend/Tauri version sync. |
| 3.v1 | Verify: Docs and delivery commands describe a CLI-first project with no GUI requirement for development or testing. | done | README, CI, Makefile, and release scripts are aligned on a CLI-only required path. |

## Completion Record
- `cargo test --workspace --test cli_integration -- --nocapture`: passed (2/2)
- `make test-all`: passed
- `bash scripts/release/validate-version-sync.sh`: passed
- `bash scripts/release/generate-release-metadata.sh /tmp/chaos-bot-cli-release-check`: passed
- `CARGO_TARGET_DIR=/tmp/chaos-bot-cli-target bash scripts/release/verify-packaged-runtime.sh /tmp/chaos-bot-cli-release-check`: passed
- `CARGO_TARGET_DIR=/tmp/chaos-bot-cli-target bash scripts/release/verify-github-installer.sh /tmp/chaos-bot-cli-release-check`: passed
- `CARGO_TARGET_DIR=/tmp/chaos-bot-cli-target bash scripts/release/verify-self-upgrade.sh /tmp/chaos-bot-cli-release-check`: passed

## Notes
- Release outputs were directed to `/tmp/chaos-bot-cli-release-check` because this sandbox blocks creating a fresh `.tmp/` tree inside the CLI worktree.
- Installer and self-upgrade verification needed elevated execution so the staged local release endpoint and relaunched app listener could bind on `127.0.0.1`.
