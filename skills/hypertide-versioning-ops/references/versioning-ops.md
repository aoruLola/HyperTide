# Versioning Ops Reference

Use this reference to keep mutation guardrails short and consistent.

## Submit preflight

Always run and summarize before submit:

```powershell
ht status --repo <repo> --branch <branch>
ht diff --repo <repo> --branch <branch>
```

## Required confirmation format

Before `submit` or `rollback`, report:

- repo and branch
- target asset or changeset
- current blocking signals, if any
- one-sentence risk summary

Then ask for explicit confirmation.

## Blocking conditions

- `locked_by_other`
- `stale_base`
- unknown target changeset
- missing repo or branch context
