---
name: hypertide-versioning-ops
description: Use when an external AI needs to change HyperTide version state through branch, stage, submit, log, or rollback operations and must apply explicit preflight and confirmation rules.
---

# HyperTide Versioning Ops

## Overview

Use this skill for version-state mutations. It covers the CLI workflows that change branch state, stage assets, create changesets, and roll back history.

This skill is allowed to drive real mutations, but only after it performs the required preflight checks and gets explicit confirmation for sensitive actions.

## When to Use

Use this skill when:

- the user wants to create or switch a branch
- assets need to be staged with `add` or `remove`
- the user wants to submit a changeset
- history or rollback operations are requested

Do not use this skill for trust/gate approval flows; hand those off to `hypertide-trust-audit`.

## Command Workflow

### Branch operations

```powershell
ht branch list --repo <repo>
ht branch create --repo <repo> --name <branch> [--from <base-branch>]
ht branch switch --repo <repo> --name <branch>
```

### Stage operations

Preferred high-level staging:

```powershell
ht add --file <local-file> --asset-path <repo-path> [--branch <branch>]
ht remove --asset-path <repo-path> [--branch <branch>]
```

Compatibility path when a blob already exists:

```powershell
ht add --path <repo-path> --blob <blob-hash> [--branch <branch>]
```

### Submit and history

Required preflight before submit:

```powershell
ht status --repo <repo> --branch <branch>
ht diff --repo <repo> --branch <branch>
```

Submit only after reporting those results and receiving confirmation:

```powershell
ht submit --repo <repo> --branch <branch> --message "<message>"
```

History and rollback:

```powershell
ht log --repo <repo> --branch <branch> --limit <n>
ht rollback --repo <repo> --branch <branch> --to <changeset-id> --message "<message>"
```

## Safety Rules

Sensitive actions in this skill:

- `branch switch`
- `submit`
- `rollback`
- any staging action that overwrites or deletes a tracked asset

Before a sensitive action, the agent must report:

- current repo
- current branch
- target branch / asset / changeset
- current `status` or `diff` signal
- one-sentence risk summary

Then it must explicitly ask the user to confirm before proceeding.

Additional rules:

- Do not skip `status` and `diff` before `submit`.
- Do not assume the branch target for `add` or `remove`.
- Do not suggest rollback without first showing enough `log` context to identify the target changeset.

## Failure Handling

- If submit hits `BaseChangesetMismatch`, stop and send the user back through `sync -> checkout -> status -> diff`.
- If submit or staging reports a lock conflict, surface the locked asset path and do not continue automatically.
- If blob upload fails, keep the failure scoped to the asset and avoid implying the whole repo is corrupted.

## References

- [versioning-ops.md](./references/versioning-ops.md)
- [`docs/api/openapi.yaml`](../../../docs/api/openapi.yaml)
