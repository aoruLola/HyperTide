# HyperTide M8-012 Validation Matrix

日期：2026-03-09

## Summary

本矩阵把路线图中的 `M8-012` 从“目标要求”落成“可执行验证清单 + 当前证据”。

当前结论：

1. runtime smoke 已完成，并且在 fresh compose 项目与默认 compose 项目上都拿到了成功证据
2. 代码级验证已经按 Auth、AuthZ、Versioning、Upload、Trust、Recovery 六类显式重跑
3. 基于本页记录，`M8-012` 可以关闭

## Runtime Smoke Evidence

已验证命令：

1. `docker compose -p hypertidefresh -f deploy/docker-compose.yml --env-file deploy/.env.example up -d --build`
2. `powershell -ExecutionPolicy Bypass -File .\deploy\smoke.ps1`
3. `$env:APP_PORT='3001'; $env:POSTGRES_PORT='5433'; docker compose -f deploy/docker-compose.yml --env-file deploy/.env.example up -d --build`
4. `powershell -ExecutionPolicy Bypass -File .\deploy\smoke.ps1 -BaseUrl http://localhost:3001`

Smoke 覆盖的链路：

1. `/health/live`
2. `/health/ready`
3. `ht login`
4. `ht add --file`
5. `ht submit`
6. `ht sync`
7. `ht checkout`
8. `ht status`
9. `ht diff`

## Categorized Rerun Evidence

2026-03-09 的分类回归命令：

1. `cargo test -p hypertide-server core::auth::tests -- --nocapture`
2. `cargo test -p hypertide-server tests::lock_route_rejects_missing_api_key -- --exact`
3. `cargo test -p hypertide-server core::versioning::tests::stale_base_is_rejected -- --exact`
4. `cargo test -p hypertide-server tests::compose_blob_reassembles_uploaded_chunks -- --exact`
5. `cargo test -p hypertide-server tests::witness_topology_reports_cross_environment_groups -- --exact`
6. `cargo test -p hypertide-server core::versioning::tests::persists_state_across_manager_restarts -- --exact`
7. `powershell -ExecutionPolicy Bypass -File .\deploy\smoke.ps1 -BaseUrl http://localhost:3001`

结果：

1. 全部通过
2. runtime smoke 输出 `Smoke passed.`

## Matrix

| Category | Required by roadmap | Current evidence | Status |
|---|---|---|---|
| Auth | exchange / refresh / revoke / replay | `core::auth::tests::*`；runtime smoke 验证 `login` 与 `/v2/auth/exchange-key` | done |
| AuthZ | 401 / 403 / 200 coverage | `tests::lock_route_rejects_missing_api_key`；既有 admin/forbidden 路由测试 | done |
| Versioning | CAS conflict / rollback / sync | `stale_base_is_rejected`、`rollback_plan_targets_existing_history`、runtime smoke 的 `submit/sync/checkout` | done |
| Upload | resumable / missing-chunk-only / manifest integrity | `compose_blob_reassembles_uploaded_chunks`；runtime smoke 的 `add --file`；`chunk/missing/manifest/compose` 实装 | done |
| Trust | audit / checkpoint / witness / signed admin ops | `witness_topology_reports_cross_environment_groups`；既有 replay/audit/retention 路由测试 | done |
| Recovery | restart persistence / migration gate | `persists_state_across_manager_restarts`；`ready_returns_503_without_db_pool`；fresh/default compose 启动验证 | done |

## Concrete Evidence

### Auth

1. `core::auth::tests::test_dev_master_key`
2. `core::auth::tests::test_generate_and_validate_key`
3. `core::auth::tests::test_invalid_key`
4. `core::auth::tests::test_revoke_key`
5. runtime smoke:
   - `ht login --api-key-direct`
   - `/v2/auth/exchange-key`
   - `/v2/auth/verify`

### AuthZ

1. `tests::lock_route_rejects_missing_api_key`
2. `tests::lock_route_rejects_spoofed_owner_id`
3. `tests::changeset_gate_rejects_missing_api_key`
4. `tests::retention_policy_returns_ok_with_admin_key`
5. `tests::audit_export_rejects_missing_api_key`

### Versioning

1. `core::versioning::tests::stale_base_is_rejected`
2. `core::versioning::tests::rollback_plan_targets_existing_history`
3. `core::versioning::tests::submit_with_head_match_advances_branch_head`
4. runtime smoke:
   - `submit`
   - `sync`
   - `checkout`

### Upload

1. `tests::compose_blob_reassembles_uploaded_chunks`
2. runtime smoke:
   - `add --file`
3. CLI/Server integration path:
   - `/v2/storage/upload`
   - `/v2/blobs/missing`
   - `/v2/blobs/chunks/{chunk_hash}`
   - `/v2/manifests`
   - `/v2/blobs/compose`

### Trust

1. `tests::replay_verify_rejects_missing_api_key`
2. `tests::replay_readiness_returns_503_when_service_unavailable`
3. `tests::audit_export_returns_503_when_service_unavailable`
4. `tests::retention_policy_returns_ok_with_admin_key`
5. `tests::witness_topology_reports_cross_environment_groups`

### Recovery

1. `core::versioning::tests::persists_state_across_manager_restarts`
2. `tests::ready_returns_503_without_db_pool`
3. fresh/default compose project startup succeeded after migration version fix

## Closure

1. `M8-011`：已完成，runtime smoke 已通过
2. `M8-012`：已完成，六类验证均有明确执行记录
3. 后续如果继续扩展验证项，应在本页追加时间戳和命令记录，而不是只改路线图摘要
