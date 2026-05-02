# Contributing

This repository is currently operated as a private primary codebase during the commercial incubation phase of HyperTide.

## Current Contribution Model

At this stage, contributions are coordinated directly by the maintainers.

Expected workflow:

1. Create a branch from `main`
2. Make focused changes with matching docs and verification
3. Run the relevant checks before requesting review
4. Open a GitHub pull request against `main`

## Minimum Expectations

- Keep changes scoped and reviewable
- Update documentation when behavior or workflows change
- Do not commit runtime artifacts such as `storage/`, `tmp/`, or local state directories
- Preserve the current product positioning: centralized, asset-oriented collaboration rather than distributed Git-style workflows

## Verification

Baseline verification:

```powershell
cargo test --workspace
```

When deployment behavior changes, also run:

```powershell
powershell -ExecutionPolicy Bypass -File .\deploy\smoke.ps1
```

## Security

For security-sensitive findings, use the private reporting path described in [SECURITY.md](SECURITY.md) instead of opening a public issue.
