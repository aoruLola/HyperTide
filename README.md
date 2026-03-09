# HyperTide

HyperTide is a centralized asset collaboration and versioning system for large binary workflows.

This repository is the private primary codebase for the commercial incubation phase of HyperTide. It is not published as an open source repository, and no open source license is granted at this stage.

## What HyperTide Is

HyperTide is designed for teams that need server-governed collaboration around large binary assets, workflow controls, and auditable version operations. It is not intended to be a Git replacement for distributed source-code workflows.

Current product direction:

- Server-maintained version truth and branch state
- Asset-oriented CLI workflows for sync, checkout, upload, staging, and submit
- Locking, gating, rollback, and runtime validation for operational control

## Repository Layout

- `crates/server`: `hypertide-server` package, binary name `hypertide`
- `crates/cli`: `hypertide-cli` package, binary name `ht`
- `docs/`: design notes, roadmap, validation evidence, and CLI usage docs
- `deploy/`: Docker and smoke-test assets

## Key Docs

- CLI guide: [docs/cli/README.md](docs/cli/README.md)
- Deployment notes: [deploy/README.md](deploy/README.md)

## Development Baseline

Primary verification command:

```powershell
cargo test --workspace
```

Runtime smoke entrypoint:

```powershell
powershell -ExecutionPolicy Bypass -File .\deploy\smoke.ps1
```
