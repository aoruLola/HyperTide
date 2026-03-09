# Trust Audit Reference

Use this reference to keep governance operations disciplined.

## Must-check sequence for promote

1. confirm repo and changeset id
2. read gate state
3. summarize release risk
4. ask for confirmation
5. only then approve or promote

## Relevant documented endpoints

- `GET /v2/changesets/{changeset_id}/gate`
- `POST /v2/changesets/{changeset_id}/approve`
- `POST /v2/changesets/{changeset_id}/promote`
- `GET /v2/history/{repo_id}`
- `POST /v2/rollback`
- trust checkpoint, witness, audit, retention, replay endpoints under `/v2/trust/*`

## Confirmation summary format

Before a sensitive governance action, summarize:

- repo
- branch if applicable
- target changeset or checkpoint
- gate or trust state
- one-sentence risk summary
