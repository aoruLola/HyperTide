# HyperTide

[中文](README_CN.md)

**Centralized version control for large binary assets.**

HyperTide is purpose-built for game content, art files, build outputs, and other large binary assets that don't work well with Git. It provides server-maintained version truth, file locking, approval workflows, and full audit trails — all through a simple CLI.

## Why HyperTide

| | Git | Git LFS | Perforce | **HyperTide** |
|---|---|---|---|---|
| Large binary files | Poor | OK | Good | **Excellent** |
| File locking | No | No | Yes | **Yes** |
| Approval workflows | No | No | Limited | **Full (gate → approve → promote)** |
| Audit chain | No | No | Basic | **BLAKE3 hash chain + witnesses** |
| Agent support | No | No | No | **Native (sessions, checkpoints)** |
| Self-hosted | Yes | Yes | Expensive | **Yes (Docker)** |
| Open source | Yes | Yes | No | **Yes (MIT)** |

## Quick Start

### Prerequisites

- Rust 1.85.1+
- PostgreSQL 15+
- Docker (optional, for container deployment)

### Install

```bash
# Clone
git clone https://github.com/openLYURA/HyperTide.git
cd HyperTide

# Build
cargo build --release

# The CLI binary is at target/release/ht
# The server binary is at target/release/hypertide
```

### Run with Docker

```bash
docker compose -f deploy/server/docker-compose.yml --env-file deploy/server/.env.example up -d
```

### Login and start working

```bash
# Login to server
ht login --server http://localhost:3000 --token dev-master-key

# Create or select a repo for this workspace
ht init --repo my-repo

# Check health
ht doctor

# Pull latest assets
ht sync
ht checkout

# Edit files, then stage and submit
ht add --file Content/Props/tree.uasset
ht status
ht submit --message "update tree prop"
```

## CLI Commands

### Core Workflow

| Command | Description |
|---|---|
| `ht login` | Save server credentials and defaults |
| `ht init` | Create/select a repo and initialize the local workspace |
| `ht repo` | Create, list, inspect, or select repositories |
| `ht sync` | Sync local metadata to server snapshot |
| `ht checkout` | Pull server assets into workspace |
| `ht add` | Stage a local file for the next submit |
| `ht remove` | Stage an asset removal |
| `ht submit` | Create a formal changeset |
| `ht status` | Show asset status (modified/staged/locked) |
| `ht diff` | Show hash differences |
| `ht log` | Show changeset history (`--graph` for visual) |
| `ht rollback` | Roll back to a previous changeset |

### Staging

| Command | Description |
|---|---|
| `ht stage list` | Show staged assets |
| `ht stage clear` | Clear all staged assets |

### Branching

| Command | Description |
|---|---|
| `ht branch create` | Create a branch |
| `ht branch list` | List branches |
| `ht branch switch` | Switch default branch |

### Locking

| Command | Description |
|---|---|
| `ht lock acquire` | Lock a file for editing |
| `ht lock release` | Release a lock |
| `ht lock renew` | Extend lock lease |
| `ht lock list` | List all active locks |
| `ht lock force-release` | Admin: force release a lock |

### Checkpoints (Agent Workflows)

| Command | Description |
|---|---|
| `ht save` | Save workspace progress (no branch advance) |
| `ht checkpoint create` | Create a recoverable workspace checkpoint |
| `ht checkpoint restore` | Restore workspace to a checkpoint |
| `ht checkpoint branch` | Create a branch from a checkpoint |
| `ht checkpoint list` | List checkpoints |

### Governance

| Command | Description |
|---|---|
| `ht changeset gate` | Check changeset promotion readiness |
| `ht changeset approve` | Approve a changeset |
| `ht changeset promote` | Promote changeset to visible head |
| `ht trust checkpoint` | Generate/inspect system state attestations |
| `ht trust witness` | Witness attestation operations |
| `ht trust audit` | Audit chain verification and export |
| `ht trust replay` | Event replay verification |
| `ht trust retention` | Retention policy inspection |

### Utilities

| Command | Description |
|---|---|
| `ht doctor` | Check login, connectivity, and workspace health |
| `ht completions` | Generate shell completion scripts |
| `ht chunk-upload` | Upload large files through chunk storage |

All commands support `--json` for structured output and `--help` for detailed usage.

## Architecture

```
HyperTide
├── crates/server    → hypertide-server (binary: hypertide)
├── crates/cli       → hypertide-cli (binary: ht)
├── migrations/      → PostgreSQL schema migrations
├── deploy/          → Docker and packaging scripts
└── docs/            → Design notes and specifications
```

The open source repository contains the server and CLI. The server provides REST APIs for all version operations. The CLI translates workspace actions into API calls and manages local state under `.hypertide/`. The server is the single source of truth — there is no local DAG or offline commits. Desktop or web UI work is outside the public Community Edition repository.

## Key Design Decisions

- **Content-addressable storage** — Files stored by BLAKE3 hash, automatic deduplication
- **Server-side branch heads** — No divergent local histories, stale submissions rejected
- **Four-stage changeset lifecycle** — draft → approve → promote → visible
- **Audit chain** — BLAKE3 hash chain with witness attestations, fully verifiable
- **High-risk operation signing** — HMAC-SHA256 with nonce + timestamp, anti-replay

## Deployment

See [deploy/server/README.md](deploy/server/README.md) for Docker deployment and [deploy/cli/README.md](deploy/cli/README.md) for CLI packaging.

```bash
# Docker Compose
docker compose -f deploy/server/docker-compose.yml --env-file deploy/server/.env.example up -d

# Smoke test
powershell -ExecutionPolicy Bypass -File deploy/server/smoke.ps1
```

## Documentation

- [Getting Started](docs/getting-started.md) — 5 分钟上手指南
- [CLI Reference](docs/cli/README.md) — 所有命令的详细参数和示例
- [Server Guide](docs/server/README.md) — 服务端配置、API Key 管理、生产部署
- [Architecture](docs/architecture.md) — 系统架构和设计决策
- [API Spec](docs/api/openapi.yaml) — OpenAPI 规格
- [Docker Deployment](deploy/server/README.md) — Docker 部署指南
- [Contributing](CONTRIBUTING.md) — 贡献指南
- [Security](SECURITY.md) — 安全问题报告

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). In short:

1. Fork and create a branch from `main`
2. Make focused changes with tests
3. Submit a pull request

## Security

Report security issues privately via [SECURITY.md](SECURITY.md). Do not disclose publicly.

## License

[MIT](LICENSE)
