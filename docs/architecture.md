# Architecture

HyperTide Community Edition is a centralized asset-versioning system for large
binary workflows.

## Core Runtime

- `hypertide-server` owns version truth, branch heads, changesets, locks,
  storage, auth, witness, audit, replay, and retention APIs.
- `hypertide-cli` turns local workspace operations into authenticated API calls
  and persists local state under `.hypertide/`.
- Postgres stores authoritative metadata and event/audit records.
- Local filesystem storage currently backs blob and manifest objects.

## Open Core Extension Points

The public server crate exposes community/default provider traits for:

- authentication providers
- policy engines
- attestation providers
- audit sinks
- witness providers

Enterprise builds should depend on the public crates and provide commercial
implementations without making the public repository depend on private crates.

## Operational Surface

The server exposes:

- `/health/live` for liveness
- `/health/ready` for database readiness
- `/metrics` for Prometheus-compatible counters
- bounded request body sizes for regular and upload routes
- global request rate limiting from `RATE_LIMIT_REQUESTS_PER_MINUTE`
- graceful shutdown on Ctrl+C and SIGTERM where supported

## Trust Model

Community Edition includes audit-chain verification, trust checkpoints, witness
receipts, replay verification, and retention policy inspection. Enterprise may
extend this with external KMS, hardware-backed signatures, advanced quorum
policy, and compliance exports.
