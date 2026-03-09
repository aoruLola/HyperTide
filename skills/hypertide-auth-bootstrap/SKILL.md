---
name: hypertide-auth-bootstrap
description: Use when an external AI needs to establish HyperTide CLI login, choose the right authentication mode, or confirm repo and branch context before any workspace or version operation.
---

# HyperTide Auth Bootstrap

## Overview

Use this skill to establish a safe HyperTide starting point before doing any workspace, upload, or versioning work.

This skill is CLI-first:

- prefer `ht`
- only mention direct `/v2/auth/*` API fallback when CLI coverage is missing or the user explicitly asks for HTTP integration

## When to Use

Use this skill when:

- the user needs to log into HyperTide
- repo or branch context is unclear
- an external AI is about to run its first HyperTide command in a workspace
- authentication mode must be chosen between JWT exchange and `--api-key-direct`

Do not use this skill for submit, rollback, or governance actions. Hand those off to the dedicated versioning or trust skills.

## Command Workflow

1. Gather the minimum context before proposing commands:
   - `server`
   - `token`
   - `repo`
   - `branch`
2. Prefer the JWT exchange path:

```powershell
ht login --server <server> --token <api-key> --repo <repo> --branch <branch>
```

3. Use `--api-key-direct` only when:
   - the user explicitly requests it, or
   - the environment is a development/emergency path where JWT exchange is intentionally unavailable

```powershell
ht login --server <server> --token <api-key> --api-key-direct --repo <repo> --branch <branch>
```

4. After login, restate the active repo and branch before handing off to another HyperTide skill.

## Safety Rules

- Do not assume `main`.
- Do not assume the default repo from prior context unless the user confirmed it in this session.
- Do not recommend `--api-key-direct` as the normal production path.
- Treat all tokens and keys as secrets: never echo them back in summaries or logs.
- If a login command would replace the user's local default repo or branch, state that explicitly before suggesting it.

## Failure Handling

- If login fails due to invalid credentials, ask for a fresh server/token pair rather than retrying blindly.
- If the user only gives a repo or branch name, stop and ask for the missing server/token inputs before proposing a login command.
- If the user wants to inspect state without mutating anything, pivot to `hypertide-workspace-flow` after login.

## References

- [auth-bootstrap.md](./references/auth-bootstrap.md)
- [`docs/api/openapi.yaml`](../../../docs/api/openapi.yaml)
