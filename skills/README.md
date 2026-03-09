# HyperTide Skills

This directory is the versioned source of truth for HyperTide external AI skills.

Current skill set:

- `hypertide-auth-bootstrap`
- `hypertide-workspace-flow`
- `hypertide-versioning-ops`
- `hypertide-trust-audit`

Each skill follows the same layout:

- `SKILL.md`
- `agents/openai.yaml`
- `references/`

Use the sync script to copy these skills into local agent install directories without deleting anything:

```powershell
powershell -ExecutionPolicy Bypass -File .\skills\sync-skills.ps1
```

Targets supported by the sync script:

- `~/.codex/skills`
- `~/.agents/skills`

The sync is non-destructive:

- It creates missing directories
- It copies and overwrites files present in this repo
- It does not delete extra files from the destination
