---
name: hypertide-trust-audit
description: Use when an external AI needs HyperTide governance, gate, witness, audit, retention, or replay operations and must apply stricter confirmation before sensitive trust actions.
---

# HyperTide Trust Audit

## Overview

Use this skill for governance and trust workflows around approval gates, promote decisions, witness topology, audit export, retention policy, and replay verification.

This skill is CLI-first where possible, but it may use documented `/v2/*` API fallback for gate, promote, trust, and audit endpoints that do not have first-class CLI coverage.

## When to Use

Use this skill when:

- the user asks to approve or promote a changeset
- gate status must be checked before release
- trust checkpoint, witness, audit, retention, or replay endpoints are relevant
- the user needs evidence or governance state rather than workspace state

Do not use this skill for ordinary file staging or submit flows.

## Command and API Workflow

### Gate-oriented release checks

If the user wants to promote a changeset, inspect governance state first. When CLI coverage is absent, use the documented API endpoints:

- `GET /v2/changesets/{changeset_id}/gate?repo_id=<repo>`
- `POST /v2/changesets/{changeset_id}/approve?repo_id=<repo>`
- `POST /v2/changesets/{changeset_id}/promote?repo_id=<repo>`

### History and rollback context

Use these documented interfaces when governance work needs release context:

- `GET /v2/history/{repo_id}`
- `POST /v2/rollback`

### Trust and audit surfaces

Relevant documented endpoints include:

- trust checkpoints
- witness summary / topology
- audit verify / export
- retention policy
- replay verify / readiness

## Safety Rules

Sensitive actions in this skill:

- `approve`
- `promote`
- retention writes
- replay or force-style administrative writes
- any trust or governance action that changes release state

Before any sensitive action, the agent must report:

- repo
- branch if applicable
- target changeset or checkpoint
- current gate or trust status
- one-sentence operational risk summary

Then it must get explicit confirmation.

Additional rules:

- Never promote without checking gate status first.
- Never assume a changeset is eligible because it exists in history.
- Keep audit and replay operations read-only unless the endpoint and user request clearly call for a write.

## Failure Handling

- If gate status is not ready, stop and explain which prerequisite is missing.
- If trust or audit endpoints return authorization failures, surface that as a governance permission issue rather than a generic network problem.
- If the request is ambiguous between a rollback and a promote, stop and restate the two possible intents before proposing commands.

## References

- [trust-audit.md](./references/trust-audit.md)
- [`docs/api/openapi.yaml`](../../../docs/api/openapi.yaml)
