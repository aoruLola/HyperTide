# Roadmap

HyperTide is moving toward a stable open-core `v0.1.0` release.

## v0.1.0 Community Edition

- Clean-clone OSS build without private repository dependencies.
- MIT licensing and public contribution/security process.
- Server health, readiness, metrics, graceful shutdown, and rate limiting.
- CLI-first asset workflow: login, sync, checkout, add, status, submit, log,
  rollback, lock, and doctor.
- Community witness/audit/replay trust layer with structured configuration.
- Docker Compose local deployment and release artifacts with checksums.

## After v0.1.0

- Stronger OpenAPI contract tests.
- More backend contract tests across in-memory and Postgres implementations.
- Safer CLI preview and dry-run flows for materializing large workspaces.
- UI workspace for browsing repositories, locks, changesets, and audit state.
- Public extension documentation for commercial Enterprise providers.

## Enterprise Boundary

HyperTide Enterprise is a commercial distribution built on the public core. It
may include advanced identity, RBAC/ABAC, compliance exports, cloud KMS or
hardware-backed witness integrations, multi-tenant governance, SLA support, and
managed deployment assistance.
