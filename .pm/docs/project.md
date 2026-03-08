# Project Context

- Project: cicd
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/cicd
- Base Branch: master
- Worktree Mode: git worktree
- Updated At: 2026-03-08T17:18:06+08:00

## Requirements

- Build a GitHub CI/CD pipeline for this repository.
- Publish release artifacts whenever `master` is updated, with explicit versioning.
- Produce installable binary distribution that includes frontend and backend deliverables.
- Support self-upgrade so installed binaries can move to the latest released version safely.

## Technical Constraints

- Respect the frozen backend architecture: `application/`, `domain/`, `infrastructure/`, `interface/`, `runtime/`, and `lib.rs` only.
- Preserve `README.md` as the single maintained architecture/runtime/testing document entry.
- CI full gate must remain `make test-all`, with failure artifacts aligned to `.github/workflows/ci.yml`.
- Every new feature must include a dedicated e2e testing phase before task completion.
- Packaging must account for the existing Rust backend, React frontend, and Tauri shell layout in this mono-repo.
- Versioning and publish automation should be compatible with GitHub Actions and the repository's `master` release branch policy.

## Assumptions To Validate During Execution

- Release version source is the repository-owned `VERSION` manifest, with derived release tags and artifact stems generated from task-1 scripts.
- The initial "binary installation with fe and be" delivery is a user-installable `linux-x86_64` archive containing the backend binary, bundled frontend `dist`, installer script, and release manifest/checksum assets.
- Native Tauri desktop bundles remain a follow-on option once the desktop shell can launch or embed the packaged backend consistently.
- The implemented self-upgrade flow now targets installed Linux release bundles, consumes GitHub Release metadata plus bundle checksums, installs into a new versioned release directory, and requires a manual relaunch rather than an in-process binary swap.
- Cross-platform self-upgrade remains a follow-on concern if additional artifact targets or native desktop installers are introduced.
