# HyperTide VCS 设计问题评估报告复核

> 复核口径：`当前实现 + 已写入路线图的设计意图`
>
> 核心结论：按当前实现，HyperTide 应被描述为中心化、资产协作导向的版本控制系统，而不是分布式源码 VCS；因此应避免直接拿 Git/Fork/Rebase 模型做一一对标。

## 结论表

| 主题 | 结论 | 一句话理由 |
| --- | --- | --- |
| 本地仓库缺失 | 准确 | 当前 CLI 只维护 `.hypertide/profile.json` 和 `.hypertide/stage.json`，没有本地对象库、完整历史或本地 DAG。 |
| `add` 命令设计 | 准确 | `ht add` 仍要求显式传入 `--blob`，CLI 不负责读取文件、计算哈希和自动上传。 |
| merge 机制缺失 | 部分准确 | 当前确实没有 merge/rebase，但系统已实现 `draft/approve/promote/gate` 这条服务端门禁流，不能简单等同于“只有线性裸提交”。 |
| 工作区机制不明确 | 准确 | CLI 没有 `checkout/status/diff/reset/clean`，也不直接管理本地文件树，当前工作区能力仅限本地 stage 元数据。 |
| 冲突处理机制缺失 | 部分准确 | 当前没有传统 merge conflict 机制，但已经有文件锁、lease 和 `base_changeset_id` CAS 冲突校验。 |
| `chunk-upload` 与版本提交流程割裂 | 准确 | `chunk-upload` 负责分块上传和 manifest 创建，但不会自动 stage 或 submit。 |

## 1. 本地仓库缺失

### 结论

`准确`

### 现状证据

当前 CLI 状态模型非常轻量，仅包含：

- `CliProfile`：服务地址、API key、token、默认 repo/branch
- `StageFile`：当前分支、`base_changeset_id`、`assets`

这些状态都保存在当前目录下的 `.hypertide` 中，代码位置见 `crates/cli/src/main.rs` 里的 `CliProfile`、`StageFile`、`load_profile`、`save_stage`、`current_dir`。  
`sync` 的行为是请求 `/v2/sync/{repo_id}` 拉取服务端快照，再把返回的 `changeset_id` 写成 stage 基线；`log`、`branch list/create/switch` 也都直接走 HTTP API，而不是读取本地提交图。

服务端虽然维护了 `changesets`、`branches`、`snapshots` 和 `parent_changeset_id`，也就是服务端侧 DAG，但这些结构存在于 `crates/server/src/core/versioning.rs` 的 `RepoState` 中，不在客户端本地。

### 路线图修正

路线图明确聚焦 backend + CLI 生产化，没有任何条目承诺要实现 Git 式本地仓库或分布式对象库。  
M9/M10/M11 的重点是内容寻址、chunk/manifest、事件、审核、staging refs 与 promote gate，而不是本地离线提交能力。

### 改写建议

把原报告中的“HyperTide 当前实现并不是分布式版本控制系统，而是中心化版本控制系统”保留。  
更准确的写法是：

> HyperTide 当前实现采用服务端维护历史与快照、客户端仅维护轻量 stage 元数据的中心化模型，因此不具备 Git 式本地提交、离线分支和本地对象库能力。

## 2. `add` 命令设计

### 结论

`准确`

### 现状证据

`crates/cli/src/main.rs` 中 `AddArgs` 的参数就是：

- `--path`
- `--blob`
- `--branch`

`add()` 的行为只是把 `{ path, blob_hash }` 写入本地 `stage.json`。  
真正会读取本地文件、切 chunk、计算 BLAKE3、查询缺失 chunk、上传分块、创建 manifest 的逻辑在 `chunk_upload()`；但这个命令执行完不会自动调用 `add()` 或 `submit()`。

因此，当前体验确实要求用户或脚本先拿到 blob/hash，再执行 `ht add --path --blob`，这和 `git add file` 的心智模型差异很大。

### 路线图修正

路线图的 M9 明确写了 “Add CLI resumable/chunked transfer workflow”，说明系统设计已经承认要补强上传体验。  
但截至当前实现，CLI 仍然停留在“上传”和“stage”两个分离动作。

### 改写建议

原报告对 UX 的批评是成立的。  
建议把措辞从“设计不合理”改成更技术中性的表达：

> 当前 `add` 更像“写入版本元数据”而不是“把文件纳入版本控制”。这对自动化流水线友好，但对普通开发者不友好，是当前 CLI 最明显的体验缺口之一。

## 3. merge 机制缺失

### 结论

`部分准确`

### 现状证据

当前 CLI 没有 `merge`、`rebase`、`conflict resolve` 等命令；服务端 API 也没有对应接口。  
已实现的分支主流程是：

- `branch create`
- `submit_changeset`
- `approve_changeset`
- `promote_changeset`
- `changeset_gate`
- `rollback`

在 `crates/server/src/core/versioning.rs` 中，`ChangesetVisibility`、`ChangesetStatus`、`staging_ref()`、`visible_ref()`、`approve_changeset()`、`promote_changeset()` 说明当前系统不是“功能分支 merge 回主线”的 Git 模型，而是“草稿 changeset -> 审批 -> 推广为可见 head”的门禁流模型。  
也就是说，系统已经存在非线性开发意图，但它体现为 draft/promote gate，不是 merge/rebase。

### 路线图修正

M11 路线图明确提到 “Add Git staging refs + approval gate to protected mainline”。  
这表明项目当前是有意朝“受保护主线 + 草稿/推广”演进，而不是优先做 Git 式 merge。

### 改写建议

原报告里“如果没有 merge，则分支只能线性推进”的说法过于绝对。  
更准确的写法是：

> 当前系统尚未提供 Git 式 merge/rebase 能力，因此传统功能分支合并流程并不存在；取而代之的是以 draft/approve/promote 为核心的服务端门禁流。这种模型可以支持受控并行开发，但不等价于源码 VCS 的分支合并语义。

## 4. 工作区（Workspace）机制不明确

### 结论

`准确`

### 现状证据

当前 CLI 命令集中没有：

- `checkout`
- `status`
- `diff`
- `reset`
- `clean`

`sync` 只同步服务端快照元数据，不会把文件 materialize 到本地工作树。  
CLI 本地状态只记录在 `.hypertide/profile.json` 和 `.hypertide/stage.json`，这说明当前客户端更像“版本元数据提交器”和“资源上传器”，而不是完整工作区管理器。

从 CLI README 的描述也可以看出，当前典型流程是：

- 上传 chunk / 创建 manifest
- 用 `add --path --blob` 写入 stage
- 用 `submit` 提交 changeset

而不是“改本地文件 -> 看 diff -> add -> commit”。

### 路线图修正

现有路线图没有承诺要做完整的本地工作区模型，也没有提 `checkout/status/diff`。  
这意味着“工作区机制不明确”不仅是 README 漏写，而是当前产品能力本身就尚未进入这一层。

### 改写建议

建议把原文中的“更像资产数据库”修正为：

> 就当前 CLI 形态而言，HyperTide 更像“中心化资产版本服务 + 轻量提交客户端”，而不是带完整本地工作区语义的开发者 VCS 工具。

## 5. 冲突处理机制缺失

### 结论

`部分准确`

### 现状证据

如果把“冲突处理”限定为传统 VCS 的 merge conflict / rebase conflict，那么当前系统确实没有对应命令和流程。  
但若据此推导“系统缺乏并发控制”，这个结论不成立，因为当前已经实现了两类冲突控制：

1. 锁冲突  
   `crates/server/src/core/lock.rs` 中有 `try_lock`、`renew_lock`、`unlock`、`force_unlock`，并且会返回 `File is already locked by ...`。

2. 基线冲突  
   `crates/server/src/core/versioning.rs` 与 `crates/server/src/api/versioning.rs` 中存在 `BaseChangesetMismatch`；`submit_changeset` 前会校验 `base_changeset_id` 是否仍等于分支 head。  
   这本质上是服务端 CAS，能阻止“基于过期快照提交覆盖新 head”。

此外，`ensure_lock_access()` 会在提交或回滚前检查资源路径是否被别人持锁。

### 路线图修正

路线图的验证矩阵明确包含 `CAS conflict`、`rollback`、`sync`。  
这说明并发修改问题在产品设计里不是空白，而是用“锁 + CAS + gate”路线在处理，而不是用 merge/rebase。

### 改写建议

建议把原报告中的“如果缺乏冲突机制，可能导致最后提交覆盖前提交”改成：

> 当前系统没有传统 merge conflict 机制，但已经实现锁冲突和基线 CAS 冲突两类服务端保护。它解决的是“并发提交覆盖”问题，而不是“自动合并分叉历史”问题。

## 6. `chunk-upload` 与版本提交流程割裂

### 结论

`准确`

### 现状证据

当前 `chunk-upload` 的职责在 `crates/cli/src/main.rs` 中非常清楚：

- 读取本地文件
- 计算 chunk hash
- 查询 `/v2/blobs/missing`
- 上传缺失 chunk
- 调用 `/v2/manifests` 创建 manifest

完成后只会打印：

`chunk-upload done: chunks=... uploaded=... manifest=...`

它不会：

- 自动生成 `add` 所需的 path/blob stage 记录
- 自动把 manifest 绑定到某个 asset path
- 自动执行 `submit`

所以报告指出它和版本流转割裂，这点是成立的。

### 路线图修正

M9 已经把“CLI resumable/chunked transfer workflow”列为目标，说明项目方向上是希望把上传体验做得更完整。  
但截至当前实现，这仍然是“资源上传层”和“版本提交层”分离的两阶段流程。

### 改写建议

建议保留原报告的大方向，但改成更具体的说法：

> 当前流程对 CI、预上传和内容寻址管线友好，但对人工日常使用偏繁琐。对于普通开发者，`chunk-upload -> add -> submit` 的三段式流程是明显的体验成本。

## 综合复核结论

这份“系统缺点”报告的大方向是成立的，尤其是在以下方面判断准确：

1. HyperTide 当前不是分布式 VCS。
2. CLI 没有完整本地工作区能力。
3. `add --path --blob` 的体验不符合主流 VCS 心智模型。
4. `chunk-upload` 与版本提交之间存在明显割裂。

需要修正的关键点有两处：

1. “没有 merge/rebase”不等于“没有并发控制”。  
   HyperTide 已有 `lock + lease + base_changeset_id CAS + draft/approve/promote gate`。
2. 当前系统不应被直接拿 Git 做镜像对标。  
   更准确的参照系是：`P4/LFS/资产库 + 服务端快照版本流 + 锁与审批门禁` 的混合模型。

因此，按当前实现与路线图综合判断，HyperTide 的真实定位应当是：

> 一个中心化、资产协作导向、服务端维护版本历史与快照的版本控制系统；它已经开始具备门禁、审核与内容寻址能力，但尚未实现分布式源码 VCS 所要求的本地仓库、工作区、merge/rebase 与冲突解决体验。
