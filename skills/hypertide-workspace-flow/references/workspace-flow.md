# Workspace Flow Reference

This skill owns the safe pre-mutation read path.

## Default sequence

```powershell
ht sync --repo <repo> --branch <branch>
ht checkout --repo <repo> --branch <branch>
ht status --repo <repo> --branch <branch>
ht diff --repo <repo> --branch <branch>
```

## Meanings

- `sync`: refresh remote snapshot metadata and local baseline
- `checkout`: write blobs to the working directory
- `status`: asset-level state summary
- `diff`: base/local/staged hash comparison

## Escalation signals

If any of these show up, raise them before handing off:

- `locked_by_other`
- `stale_base`
- missing repo or branch context
- missing snapshot target for `--to`
