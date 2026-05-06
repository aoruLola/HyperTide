# Contributing

Thanks for helping improve HyperTide Community Edition. HyperTide is an
open-core project: this public repository contains the self-hostable asset
versioning core, while Enterprise-only capabilities are developed separately.

## How To Contribute

1. Open an issue for bugs, usability problems, docs gaps, or feature proposals.
2. Fork the repository and create a focused branch from `main`.
3. Keep changes scoped; avoid unrelated refactors in the same pull request.
4. Update docs when behavior, configuration, CLI output, or workflows change.
5. Run the relevant verification commands before opening the pull request.

## Development Setup

```powershell
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

Server integration tests expect a local Postgres compatible with:

```text
postgres://hypertide:hypertide@localhost/hypertide
```

The Docker Compose assets under `deploy/server/` provide the recommended local
runtime for smoke testing.

## Pull Request Expectations

- Explain the user-visible behavior change.
- Include tests for new behavior or bug fixes.
- Include docs updates for new configuration or commands.
- Do not commit secrets, runtime state, object caches, database files, or local
  `.hypertide/` workspace state.
- Do not introduce private repository dependencies into the public workspace.

## Open Core Boundary

Community Edition changes should keep clean-clone builds working without any
private token. Enterprise work belongs in the commercial distribution and should
depend on public HyperTide crates through stable extension points rather than
making this repository depend on private crates.

## Security

Please do not disclose vulnerabilities in public issues. Follow
[SECURITY.md](SECURITY.md) for private reporting.
