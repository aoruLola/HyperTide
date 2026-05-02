# HyperTide 最新开发与改良计划

日期：2026-03-09

## Summary

基于 [2026-03-09-hypertide-review-report.md](/E:/Project/HyperTide/docs/plans/2026-03-09-hypertide-review-report.md) 与当前 M8-M11 进度，下一阶段采用双线并行：

1. 稳定化线：完成 M8-011 runtime smoke 与 M8-012 验证矩阵闭环
2. 工作流改良线：把 HyperTide 明确演进为中心化资产版本系统，补齐 workspace 操作、本地对象缓存和更自然的 CLI 工作流

产品定位固定为：

> HyperTide 不是 Git 替代品；它是服务端维护版本真相、客户端提供完整资产工作区体验的中心化版本系统。

## 当前落地点

本轮已落地的基础能力：

1. 服务端新增 `POST /v2/blobs/compose`
2. CLI 新增 `checkout`、`status`、`diff`、`remove`
3. CLI `add` 新增 `--file` 高层模式
4. CLI `chunk-upload` 新增默认 compose 与 `--manifest-only`
5. 本地新增 `.hypertide/workspace.json` 与 `.hypertide/cache/objects/<hash>`
6. `deploy/smoke.ps1` 已从健康检查扩展到最小 CLI 登录、上传、提交、同步、检出链路

## 工作流改良线

### A. Workspace 基础层

目标：

1. `sync` 保持元数据同步语义
2. `checkout` 负责把 snapshot materialize 到工作目录
3. `workspace.json` 记录当前 repo、branch、workspace_root、base_changeset_id、已检出资产和最近同步时间
4. `.hypertide/cache/objects/<hash>` 作为本地对象缓存，只承担性能职责，不承担版本真相职责

非目标：

1. 本地仓库
2. 本地 DAG
3. 离线 commit
4. 离线 branch

### B. 用户级 CLI 工作流

兼容层保留：

1. `ht add --path --blob`
2. `ht chunk-upload`

用户层主推：

1. `ht checkout`
2. `ht status`
3. `ht diff`
4. `ht add --file <local-file> [--asset-path <repo-path>]`
5. `ht remove --asset-path <repo-path>`

状态输出采用资产级而不是源码行级：

1. `unmodified`
2. `modified`
3. `added`
4. `deleted`
5. `staged`
6. `locked_by_other`
7. `stale_base`

### C. 上传与版本流整合

设计原则：

1. 版本层只认 `blob_hash`
2. `manifest_hash` 只属于传输/去重层，不进入 changeset 持久模型
3. 大文件工作流统一为 `chunk -> manifest -> compose -> stage`

服务端接口：

1. `POST /v2/blobs/compose`
2. 输入：`manifest_hash`
3. 输出：`blob_hash`、`size_bytes`

CLI 策略：

1. `ht add --file` 自动选择直传或 chunk 流程
2. `ht chunk-upload` 默认输出最终 `blob_hash`
3. `ht chunk-upload --manifest-only` 只做底层预上传

### D. 冲突模型

当前协作模型固定为：

1. 资源并发修改：锁冲突优先
2. 过期快照提交：`BaseChangesetMismatch`
3. 受保护主线：`draft -> approve -> promote -> visible`

明确不做：

1. Git 式 merge / rebase
2. 文本级三方合并
3. 本地分布式同步

## 稳定化线

### A. Runtime Smoke

`M8-011` 关闭条件：

1. Docker Compose 启动成功
2. 数据库迁移完成
3. `/health/live` 通过
4. `/health/ready` 通过
5. CLI 登录成功
6. 最小上传/提交/同步/检出链路通过

### B. 验证矩阵

`M8-012` 关闭条件：

1. Auth：exchange / refresh / revoke / replay
2. AuthZ：401 / 403 / 200 覆盖
3. Versioning：CAS conflict / rollback / sync
4. Upload：resumable transfer / missing-chunk-only / manifest integrity
5. Trust：audit chain / checkpoint / signed high-risk ops
6. Recovery：restart persistence / migration gate

## 下一个执行批次

1. 跑通 Docker Compose + `deploy/smoke.ps1`，确认 M8-011 实际状态
2. 补 CLI `--help` 与 README 的最终文字校对
3. 为 `checkout/status/diff/add --file/remove/chunk-upload --manifest-only` 增加更多回归测试
4. 把验证矩阵中的 workspace/CLI 场景并入现有 smoke 与后端测试基线

## PR 拆分补充：Legacy CLI 入口清理

### PR1：删除 legacy 入口 `src/bin/ht.rs`

目标：把 CLI 入口统一收敛到 `crates/cli/src/main.rs`，避免根目录旧入口与 workspace 结构并存。

#### 差异兜底校验（新增）

1. 除常规 diff 检查外，必须对 `src/bin/ht.rs` 做专项校验：
   - 代码中是否仍引用该文件路径
   - 文档中是否仍把该文件当作当前 CLI 入口
   - 脚本与发布流程是否仍依赖该路径
2. 全文检索至少覆盖：`src/bin/ht.rs`、`bin/ht.rs`、`CLI 入口` 相关说明。
3. 检查范围至少包含：`deploy/`、`docs/`、`README.md`、`CONTRIBUTING.md`。
4. 若发现“当前流程”引用旧入口，先迁移到 `crates/cli/src/main.rs` 对应入口再删除文件；仅历史归档文档可保留旧描述。

#### 删除前依赖证明（需写入 PR 描述）

PR 描述需显式给出以下证明项，确认删除 `src/bin/ht.rs` 不会破坏流程：

1. workspace members 与包定义已由 `crates/cli` 承载 CLI 二进制（`ht`）。
2. 构建脚本（如 `deploy/cli/package.ps1`、`deploy/cli/package.sh`）通过 `-p hypertide-cli --bin ht` 构建，不依赖 `src/bin/ht.rs` 路径。
3. 发布/验收脚本（如 smoke、部署文档中的运行命令）均通过 package + bin 名称调用 CLI，不依赖旧入口文件路径。

## Assumptions

1. HyperTide 下一阶段继续采用同仓库 workspace 结构：`hypertide-server` + `hypertide-cli`
2. 前端仍不在本阶段实现范围内
3. 中心化产品定位优先于 Git 兼容性追求
