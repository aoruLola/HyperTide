# HyperTide

[Chinese README](README_CN.md)

HyperTide Community Edition is an open-core, self-hostable collaboration and versioning system for large binary asset workflows. It keeps version truth, branch state, locks, approvals, audit, witness checkpoints, and replay controls on the server, while the CLI and future UI provide workspace-oriented workflows.

> HyperTide Community Edition is open source under the [MIT License](LICENSE). HyperTide Enterprise is a separate commercial offering that builds on the public open-core extension points for advanced identity, policy, attestation, compliance, governance, and support needs.

## Contents

- [Product Positioning](#product-positioning)
- [Core Capabilities](#core-capabilities)
- [Community And Enterprise](#community-and-enterprise)
- [Architecture Overview](#architecture-overview)
- [Repository Layout](#repository-layout)
- [Quick Start](#quick-start)
- [CLI Workflows](#cli-workflows)
- [Server](#server)
- [Deployment](#deployment)
- [Development And Verification](#development-and-verification)
- [Documentation Index](#documentation-index)
- [Contribution Model](#contribution-model)
- [Security And Licensing](#security-and-licensing)

## Product Positioning

HyperTide is not a Git replacement and is not designed for distributed source-code development. Its focus is large binary assets such as game content, art files, scene files, build outputs, or other files that do not fit a Git-style local DAG well.

HyperTide assumes that teams need a server-maintained source of truth, auditable operations, workspace-level sync and checkout flows, staging and submit flows, plus governance controls around high-risk changes.

Current product direction:

- Server-maintained version truth, branch heads, and changeset state.
- Asset-oriented CLI workflows for login, sync, checkout, upload, staging, save, checkpoint restore, and submit.
- Governance command surface for changeset gates, approval, promotion, locks, trust checkpoints, audit, replay, and retention inspection.
- Locks, approvals, rollback, runtime validation, and high-risk operation signing.
- No Git-style local repository, local DAG, offline commits, merge, or rebase.

## Core Capabilities

### Asset Versioning

HyperTide models version state through asset paths and blob hashes. The CLI uploads local files as blobs, writes path-to-blob changes into the local stage, and asks the server to create formal changesets.

The server validates base changesets, lock conflicts, permissions, and runtime rules before accepting a submit. This keeps the asset history centralized, auditable, and consistent across clients.

### Centralized Branches

Branch state is maintained by the server. Clients can create, list, switch, and sync branches, but they do not create an independent distributed history locally.

Every formal submit is based on a server-accepted base changeset. If the branch has advanced, the server can reject stale submissions instead of letting clients invent divergent local history.

### Workspace And Cache

The CLI stores local state under `.hypertide/`, including login profile, stage, workspace metadata, session state, and object cache.

`checkout` materializes server snapshots into the current working directory. `sync` updates metadata and base state without writing asset files. The local object cache avoids repeated downloads when the same blob is already present.

### Governance And Audit

HyperTide governance covers changeset gates, approvals, promotion, asset locks, trust checkpoints, witness attestation, audit verification, replay verification, and retention policy inspection.

These commands turn high-risk version operations from isolated client actions into verifiable, traceable, replayable server workflows.

## Community And Enterprise

HyperTide follows an Open Core model: the public repository contains the trusted asset-versioning core, and Enterprise extends it in a separate commercial distribution. Public builds do not depend on private repositories or private tokens.

| Area | Community Edition | Enterprise |
| --- | --- | --- |
| Asset versioning | CAS storage, BLAKE3 hashes, branches, changesets, rollback, sync | Advanced deployment guidance and support |
| CLI workflows | login, doctor, add, stage, submit, status, log, checkout, lock, sync | Managed rollout and organization policies |
| Auth | API key and JWT flows | SSO/OIDC/SAML/SCIM and enterprise identity lifecycle |
| Governance | locks, basic gates, witness, audit, replay, retention inspection | RBAC/ABAC, multi-tenant policy, compliance exports |
| Witness and attestation | Community witness configuration and topology | Cloud KMS, hardware-backed attestation, cross-environment quorum policy |
| Operations | Docker Compose, health checks, metrics, graceful shutdown, rate limiting | Commercial SLA, private deployment assistance, advanced observability |

## Architecture Overview

HyperTide currently consists of two main Rust packages: `hypertide-server` and `hypertide-cli`.

The server provides HTTP APIs, authentication, storage, locks, changesets, branches, audit, and trust features. The CLI turns local workspace operations into API calls and maintains `.hypertide/` local state. The future UI workspace lives under `hypertide-ui/` for desktop or web experiences.

```text
workspace
├─ crates/server     -> hypertide-server, binary: hypertide
├─ crates/cli        -> hypertide-cli, binary: ht
├─ deploy            -> server and CLI deployment assets
├─ docs              -> specs, plans, validation notes, CLI docs
├─ migrations        -> SQL migrations used by the server crate
├─ hypertide-ui      -> desktop/web UI workspace
└─ skills            -> agent-facing operational workflows
```

Maintained Rust code lives under `crates/`. The root-level `src/` tree is a legacy snapshot and should not be treated as the active server or CLI entrypoint.

## Repository Layout

| Path | Description |
| --- | --- |
| `crates/server` | Server package, binary name `hypertide`. |
| `crates/cli` | CLI package, binary name `ht`. |
| `docs/cli/README.md` | CLI usage guide and command examples. |
| `docs/` | Design notes, plans, specs, and validation evidence. |
| `deploy/server` | Server container deployment entrypoint. |
| `deploy/cli` | CLI packaging scripts and internal distribution entrypoint. |
| `deploy/ubuntu-server-windows-cli.md` | Ubuntu server and Windows CLI walkthrough. |
| `migrations/` | Server SQL migrations. |
| `hypertide-ui/` | Future desktop/web UI workspace. |
| `skills/` | Agent-facing HyperTide operational skills. |

Generated files, local state, and runtime artifacts should not be committed. Ignored outputs include `target/`, `.tmp/`, `tmp/`, `storage/`, `.hypertide/`, `deploy/keys/`, `deploy/server/keys/`, and `deploy/cli/dist/`.

## Quick Start

### 1. Prerequisites

The recommended local workflow is to run the server and CLI directly from the Rust workspace. The workspace uses Rust 2021 edition and declares `1.85.1` as the minimum Rust version.

```powershell
rustc --version
cargo --version
```

For container deployment, prepare Docker and Docker Compose. Server persistence, database, secrets, and smoke-test details are documented in `deploy/server/README.md`.

### 2. Start A Local Server

For the fastest self-hosted trial, use the server deployment compose file:

```powershell
docker compose -f deploy/server/docker-compose.yml --env-file deploy/server/.env.example up -d --build
```

Then verify readiness:

```powershell
curl http://localhost:3000/health/live
curl http://localhost:3000/metrics
```

### 3. Run Tests

Full workspace verification:

```powershell
cargo test --workspace
```

CLI-only verification:

```powershell
cargo test -p hypertide-cli
```

### 4. Inspect CLI Help

```powershell
cargo run -p hypertide-cli --bin ht -- --help
```

The maintained CLI implementation is in `crates/cli/src/main.rs`, and the workspace binary name is `ht`. Legacy root-level source is not the current CLI release entrypoint.

### 5. Log In To A Server

```powershell
cargo run -p hypertide-cli --bin ht -- login `
  --server http://localhost:3000 `
  --token dev-master-key `
  --repo demo-repo `
  --branch main
```

If the server is configured for direct API-key usage:

```powershell
cargo run -p hypertide-cli --bin ht -- login `
  --server http://localhost:3000 `
  --token dev-master-key `
  --api-key-direct `
  --repo demo-repo `
  --branch main
```

Login writes `.hypertide/profile.json` in the current directory. Without `--api-key-direct`, the CLI exchanges and refreshes JWT tokens; if refresh fails, log in again.

### 6. Complete The First Asset Loop

```powershell
New-Item -ItemType Directory -Force .\Content\Props
Set-Content .\Content\Props\tree.txt "hello hypertide"
cargo run -p hypertide-cli --bin ht -- sync --repo demo-repo --branch main
cargo run -p hypertide-cli --bin ht -- add --file .\Content\Props\tree.txt --path Content/Props/tree.txt --branch main
cargo run -p hypertide-cli --bin ht -- status
cargo run -p hypertide-cli --bin ht -- submit --repo demo-repo --branch main --message "add tree asset"
cargo run -p hypertide-cli --bin ht -- log --repo demo-repo --branch main
```

## CLI Workflows

### Supported Commands

```text
login
branch
add
remove
save
checkpoint
changeset
lock
trust
submit
log
rollback
sync
checkout
status
diff
chunk-upload
```

### Common Asset Flow

1. `login` stores server URL, credentials, default repo, and default branch.
2. `sync` updates branch metadata and base changeset without writing asset files.
3. `checkout` materializes server assets into the current working directory; use `--dry-run` to preview writes first.
4. `add` uploads local files and writes `.hypertide/stage.json`.
5. `status` and `diff` inspect local, staged, base, and lock state.
6. `submit` creates a formal changeset and clears the submitted stage.

Example:

```powershell
cargo run -p hypertide-cli --bin ht -- sync --repo demo-repo --branch main
cargo run -p hypertide-cli --bin ht -- checkout --repo demo-repo --branch main --dry-run
cargo run -p hypertide-cli --bin ht -- checkout --repo demo-repo --branch main
cargo run -p hypertide-cli --bin ht -- add `
  --file .\Content\Props\tree.uasset `
  --path Content/Props/tree.uasset `
  --branch main
cargo run -p hypertide-cli --bin ht -- status
cargo run -p hypertide-cli --bin ht -- submit `
  --repo demo-repo `
  --branch main `
  --message "update tree prop"
```

### Save And Checkpoints

`save` and `checkpoint` are recovery-layer commands for agents or long-running workflows. They can preserve current workspace asset state without advancing the formal branch head like `submit`.

```powershell
cargo run -p hypertide-cli --bin ht -- save --repo demo-repo --branch main --message "agent pass 1"
cargo run -p hypertide-cli --bin ht -- checkpoint create --repo demo-repo --branch main --message "before rewrite"
cargo run -p hypertide-cli --bin ht -- checkpoint list
cargo run -p hypertide-cli --bin ht -- checkpoint restore --id <checkpoint-id>
cargo run -p hypertide-cli --bin ht -- checkpoint branch --id <checkpoint-id> --name try/alt-plan
```

### Approval, Locks, And Trust

Formal version promotion can be checked and advanced through `changeset gate`, `changeset approve`, and `changeset promote`. Asset editing locks are managed with `lock acquire`, `lock renew`, `lock release`, and `lock list`. Trust and audit operations are exposed through `trust checkpoint`, `trust witness`, `trust audit`, `trust replay`, and `trust retention`.

```powershell
cargo run -p hypertide-cli --bin ht -- changeset gate --repo demo-repo --id <changeset-id>
cargo run -p hypertide-cli --bin ht -- changeset approve --repo demo-repo --id <changeset-id>
cargo run -p hypertide-cli --bin ht -- changeset promote --repo demo-repo --id <changeset-id>

cargo run -p hypertide-cli --bin ht -- lock acquire --path Content/Props/tree.uasset
cargo run -p hypertide-cli --bin ht -- lock list
cargo run -p hypertide-cli --bin ht -- lock release --path Content/Props/tree.uasset

cargo run -p hypertide-cli --bin ht -- trust audit verify
cargo run -p hypertide-cli --bin ht -- trust replay readiness
```

For the full CLI guide, see [docs/cli/README.md](docs/cli/README.md).

## Server

The server package lives under `crates/server`, with binary name `hypertide`. It owns API routing, authentication, blob and manifest storage, branch and changeset state, locks, audit, trust checkpoints, witness flows, replay, and retention features.

For server development, run and test from the workspace and avoid using the legacy root-level `src/` tree as an entrypoint. Database migrations live in `migrations/`, and deployment environment examples live in `deploy/server/.env.example`.

## Deployment

Deployment is split by deliverable:

- [deploy/server](deploy/server/README.md): server container deployment.
- [deploy/cli](deploy/cli/README.md): `ht` CLI packaging and internal distribution.
- [deploy/ubuntu-server-windows-cli.md](deploy/ubuntu-server-windows-cli.md): Ubuntu server and Windows CLI walkthrough.

Server container example:

```powershell
docker compose -f deploy/server/docker-compose.yml --env-file deploy/server/.env.example up -d --build
powershell -ExecutionPolicy Bypass -File .\deploy\server\smoke.ps1
```

CLI packaging examples:

```powershell
powershell -ExecutionPolicy Bypass -File .\deploy\cli\package.ps1
```

```bash
bash ./deploy/cli/package.sh
```

Older top-level `deploy/` assets are kept for compatibility, but new deployments should prefer `deploy/server/` and `deploy/cli/`.

## Development And Verification

### Baseline Verification

```powershell
cargo test --workspace
```

When changing only CLI behavior, start with:

```powershell
cargo test -p hypertide-cli
```

When changing deployment, containers, startup scripts, or runtime configuration, also run the relevant smoke test:

```powershell
powershell -ExecutionPolicy Bypass -File .\deploy\server\smoke.ps1
```

### Development Conventions

- Keep changes focused and reviewable, and update documentation when behavior changes.
- Do not commit runtime artifacts, local cache, secrets, database files, or `.hypertide/` workspace state.
- Prefer maintaining the Rust workspace under `crates/`.
- Preserve the product positioning: centralized, asset-oriented, server-governed workflows rather than distributed Git-style workflows.

## Documentation Index

- [README_CN.md](README_CN.md): Chinese project overview.
- [ROADMAP.md](ROADMAP.md): public release and product roadmap.
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md): community participation standards.
- [docs/architecture.md](docs/architecture.md): server, CLI, open-core, and operational architecture.
- [docs/cli/README.md](docs/cli/README.md): CLI usage, command examples, and common flows.
- [deploy/README.md](deploy/README.md): deployment entrypoint overview.
- [deploy/server/README.md](deploy/server/README.md): server container deployment.
- [deploy/cli/README.md](deploy/cli/README.md): CLI packaging.
- [deploy/ubuntu-server-windows-cli.md](deploy/ubuntu-server-windows-cli.md): Ubuntu server and Windows CLI walkthrough.
- [CONTRIBUTING.md](CONTRIBUTING.md): open source contribution workflow.
- [SECURITY.md](SECURITY.md): security reporting path.

## Contribution Model

This repository accepts community issues and pull requests. The default flow is to create a branch from `main`, make focused changes, update matching docs and validation, then open a GitHub pull request into `main`.

Minimum expectations:

- Code or documentation changes have a clear scope.
- Behavior changes include documentation updates.
- Relevant tests are run before submission, or skipped checks are explained.
- Generated files, secrets, local runtime state, and large temporary files are not committed.
- Public workspace builds remain independent from private Enterprise repositories.

## Security And Licensing

Security-sensitive findings should follow the private reporting path in [SECURITY.md](SECURITY.md) and should not be disclosed publicly.

This project is licensed under the [MIT License](LICENSE).
