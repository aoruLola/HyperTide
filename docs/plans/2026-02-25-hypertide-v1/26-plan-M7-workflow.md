# M7 Workflow Completion Plan (CLI First)

## MilestoneRef
- M7 Workflow Completion

## Task
- Deliver branch/history/rollback/sync end-to-end workflow with CLI-first UX and server parity.

## Files
- `src/core/versioning.rs`
- `src/api/versioning.rs`
- `src/main.rs`
- `src/bin/ht.rs`
- `Cargo.toml`

## Runtime Naming
- Rust package: `hypertide-cli`
- Server binary: `hypertide`
- CLI binary: `ht`

## Phase 1
1. Implement branch + head + history primitives.
2. Implement submit with `branch + base_changeset_id` CAS check.
3. Implement rollback as reverse submit with `rollback_of`.

## Phase 2
1. Expose `/v1/branches`, `/v1/changesets`, `/v1/history`, `/v1/rollback`, `/v1/sync`.
2. Preserve compatibility of existing `/api/*` routes.

## Phase 3
1. Implement `ht` CLI commands: `login`, `branch`, `add`, `submit`, `log`, `rollback`, `sync`.
2. Extend local profile and stage state with current branch and base changeset.

## Rollback
- Disable new `/v1/*` routes and keep `/api/*` behavior unchanged.

## Evidence
- `cargo test`
- `cargo check --bin hypertide --bin ht`
