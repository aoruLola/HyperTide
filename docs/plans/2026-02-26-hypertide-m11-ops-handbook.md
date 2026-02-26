# HyperTide M11 Ops Handbook (Backend)

## Scope
- Service: `hypertide` backend (`hypertide-cli` package)
- API surface: `/v2/*`
- Storage and control plane: Postgres + blob store + trust/audit modules

## SLO/SLA (Initial)
1. API availability:
- SLO: `>= 99.5%` monthly for `/health/live`, `/health/ready`, and core `/v2/*` workflows.

2. Auth success latency:
- SLO: `p95 < 150ms` for `/v2/auth/exchange-key` and `/v2/auth/refresh`.

3. Versioning consistency:
- SLO: `0` tolerated silent branch-head divergence.
- Enforced by: replay verification + changeset gate checks.

4. Trust integrity:
- SLO: audit chain verification passes in periodic checks.

## Readiness Gates Before Serving
1. Postgres pool initialized.
2. DB migrations completed.
3. `/health/ready` returns `200 READY`.
4. For trust-enabled deployments:
- replay readiness check has no mismatch blockers.

## Runtime Verification Endpoints
1. Health:
- `GET /health/live`
- `GET /health/ready`

2. Trust:
- `POST /v2/trust/audit/verify`
- `POST /v2/trust/replay/verify`
- `GET /v2/trust/replay/readiness`
- `GET /v2/trust/witness/topology`

3. Compliance:
- `GET /v2/trust/audit/export`
- `GET /v2/trust/retention/policy`

## Incident Runbook
1. Auth outage (401 spike):
- Check JWT signing env and API key validation path.
- Verify refresh endpoint health and DB connectivity.

2. Promote failures:
- Query `GET /v2/changesets/{id}/gate?repo_id=...`.
- Resolve required status or base/head mismatch before retry.

3. Trust mismatch:
- Run replay/audit verify APIs.
- If mismatch exists, switch/keep `dual-write` mode and stop event-led cutover.

4. Witness quorum not met:
- Check topology and configured witness IDs/quorum.
- Re-attest latest checkpoint and verify receipt freshness.

## Rollback Strategy
1. Prefer feature rollback over schema rollback:
- Disable trust/cutover features first.

2. Data safety:
- Keep append-only audit/event data.
- Use visible pointer rollback (`rollback` workflow) for business recovery.

3. Service recovery:
- Restore from known-good image + DB snapshot.
- Validate with health + trust endpoints before traffic restore.

## Change Management
1. Every milestone batch requires:
- `cargo check --locked --bin hypertide --bin ht`
- targeted regression tests for changed routes/modules

2. Any API contract change:
- update `docs/api/openapi.yaml`
- update `docs/plans/...-todo.md` status
