---
name: hypertide-workspace-flow
description: Use when an external AI needs to inspect or refresh a HyperTide workspace, sync snapshot metadata, check out files, or explain local stage and cache state without committing changes.
---

# HyperTide Workspace Flow

## Overview

Use this skill for read-heavy workspace operations. It owns the safe inspection path for snapshot metadata, local checkout state, cached blobs, and staged-but-unsubmitted assets.

This skill is CLI-first and should stay read-oriented by default.

## When to Use

Use this skill when:

- the user wants to pull a branch locally
- the user asks for current asset state
- the user needs `sync`, `checkout`, `status`, or `diff`
- an external AI should inspect a workspace before any mutation

Do not use this skill to submit or rollback changesets.

## Command Workflow

Preferred inspection sequence:

```powershell
ht sync --repo <repo> --branch <branch>
ht checkout --repo <repo> --branch <branch>
ht status --repo <repo> --branch <branch>
ht diff --repo <repo> --branch <branch>
```

Interpretation rules:

- `sync` updates local metadata and branch baseline
- `checkout` materializes files into the working directory
- `status` is the asset-level state summary
- `diff` is the asset-level base/local/staged hash comparison

If the user asks for a specific historical snapshot, add `--to <changeset-id>` to `sync` and `checkout`.

## Safety Rules

- Do not assume the current repo or branch if the command line omits them.
- Do not recommend `submit` from this skill; hand off to `hypertide-versioning-ops` instead.
- Before any mutation handoff, report the current `status` or explain why `status` could not be produced.
- Treat `locked_by_other` and `stale_base` as blocking conditions to raise, not background details to ignore.

## Failure Handling

- If `sync` fails, do not propose `checkout` until the baseline issue is understood.
- If `checkout` fails, report whether the issue looks like missing auth, missing blob data, or local filesystem trouble.
- If `status` or `diff` cannot be trusted because the workspace is uninitialized, explicitly say the workspace needs `sync` and `checkout` first.

## References

- [workspace-flow.md](./references/workspace-flow.md)
- [`docs/api/openapi.yaml`](../../../docs/api/openapi.yaml)
