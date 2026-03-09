# HyperTide M8-M11 Production Roadmap

## Summary
- Goal: productionize backend from M8 to M11 with Postgres persistence, JWT auth, content-addressed asset collaboration, and trust/governance primitives.
- Scope: backend + CLI only. No frontend implementation work in this roadmap.
- Route policy: v2-only APIs as the target contract.

## Locked Decisions
1. Persistence backend: Postgres 15.x.
2. Auth: API Key + JWT (RS256), access token 15m, refresh token 7d.
3. Delivery shape: Docker Compose (single-node) for M8 baseline.
4. Event model: dual-write transition in M8-M10; evaluate event-led in M11.
5. Witness topology: same-environment witnesses in M10, cross-environment in M11.

## Public APIs / Interfaces / Types
1. Route namespace: `/v2/*` only.
2. Auth endpoints:
- `POST /v2/auth/exchange-key`
- `POST /v2/auth/refresh`
- `POST /v2/auth/revoke-refresh`
3. Health endpoints:
- `GET /health/live`
- `GET /health/ready`
4. Blob/Manifest endpoints:
- `POST /v2/blobs/missing`
- `PUT /v2/blobs/chunks/{chunk_hash}`
- `POST /v2/blobs/compose`
- `POST /v2/manifests`
5. ChangeSet endpoints:
- `POST /v2/changesets`
- `POST /v2/changesets/{id}/approve`
- `POST /v2/changesets/{id}/promote`
6. Trust endpoints:
- `GET /v2/trust/checkpoints/latest`
- `POST /v2/trust/audit/verify`
7. Response envelopes:
- `SuccessEnvelope<T> { success, data, request_id }`
- `ErrorEnvelope { success, error: { code, message, details?, request_id } }`

## Milestone Plan

### M8 Production Baseline
1. Add SQLx, DB pool, migration runner, and readiness gate.
2. Move auth to DB and implement JWT access/refresh flow.
3. Unify auth middleware and error model with request_id.
4. Persist lock/versioning states to DB.
5. Adapt CLI for token refresh and v2 contract.
6. Deliver compose deployment + smoke tests.

### M9 Git + P4 Core Mechanics
1. Introduce content-addressed chunks and manifest hashing.
2. Implement missing-chunk query + idempotent chunk upload.
3. Add asset snapshots and changeset asset mapping.
4. Implement atomic promote transaction for visibility.
5. Strengthen lock lease + renew semantics.
6. Add CLI resumable/chunked transfer workflow.

### M10 Trust and Governance
1. Add append-only event store and dual-write from key actions.
2. Add hash-chained audit log with prev_hash continuity.
3. Add checkpoint generation (`log_head_hash`, `log_size`, `state_root`).
4. Add witness receipts (2-of-3) for checkpoint attestation.
5. Add signed high-risk actions with nonce + time window.
6. Add replay interception and trust verification APIs.

### M11 Completion and Scale
1. Validate event-replay reconstruction in canary environment.
2. Add Git staging refs + approval gate to protected mainline.
3. Implement storage tiering and studio cache protocol.
4. Add compliance export/retention and operations handbook.
5. Expand witness topology to cross-environment.

## Dependency Order (Hard Constraints)
1. Finish M8-001..M8-003 before domain persistence migration.
2. Do not switch global auth middleware before JWT refresh flow is stable.
3. M9 promote transaction depends on chunk/manifest/snapshot model readiness.
4. M10 checkpoint/witness depends on event store + audit chain online.
5. M11 starts only after one stable M10 cycle.

## Validation Matrix
1. Auth: exchange/refresh/revoke/replay.
2. AuthZ: full 401/403/200 coverage.
3. Versioning: CAS conflict, rollback, sync.
4. Upload: resumable transfer + missing chunk only + manifest integrity.
5. Trust: audit chain continuity + checkpoint verification + signed admin ops.
6. Recovery: restart persistence + migration gate.

## Rollback Strategy
1. Feature-flag trust and governance modules by milestone.
2. Keep schema down migrations for non-destructive rollback windows.
3. Use readiness gate to block partial initialization serving traffic.

## Assumptions
1. Workspace packages are `hypertide-server` and `hypertide-cli`; binaries remain `hypertide` and `ht`.
2. Frontend remains out of implementation scope.
3. API compatibility for `/api` and `/v1` is not guaranteed under v2-only policy.
