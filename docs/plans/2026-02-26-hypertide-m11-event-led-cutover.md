# HyperTide M11-002 Event-led Cutover Plan

## Goal
- Move from `dual-write` (state tables + event/audit append) to an `event-led` operating mode in a controlled canary rollout.
- Keep rollback path explicit: any consistency risk immediately falls back to `dual-write`.

## Readiness Gate
- Source of truth: `GET /v2/trust/replay/readiness`
- Required recommendation: `canary_event_led_candidate`
- Required conditions:
1. `consistency_ok = true`
2. `replay_mismatch_count = 0`
3. `checkpoint_count > 0`
4. `audit_entry_count > 0`

## Rollout Stages
1. Stage A (Observe only)
- Keep production in `dual-write`.
- Run readiness check on schedule (for example every 5 minutes) and alert on regression.

2. Stage B (Canary repos)
- Select low-risk repos/branches.
- Keep `dual-write` as fallback, but enable event-led read-path on canary scope only.
- Compare read outputs between state-table path and replay path.

3. Stage C (Expand)
- Gradually increase canary coverage by repo/team.
- Require zero mismatch window before each expansion.

4. Stage D (Default event-led)
- Switch default read-path to event-led.
- Keep shadow state-table checks and automated rollback trigger.

## Rollback Rules
1. Any replay mismatch in canary scope.
2. Branch head divergence detected by replay verification.
3. Audit/checkpoint pipeline interruption that blocks trust verification.

Rollback action:
- Disable event-led read-path for affected scope.
- Return to `dual-write` baseline.
- Keep event/audit append enabled for postmortem replay.

## Operational Metrics
1. Replay verification pass rate.
2. Replay mismatch count and category (locks/visible changesets/branch heads).
3. Read-path latency delta (event-led vs state-table).
4. Checkpoint freshness and witness receipt lag.

## Current Implementation Mapping
1. Replay consistency check API:
- `POST /v2/trust/replay/verify`
2. Event-led readiness report API:
- `GET /v2/trust/replay/readiness`
