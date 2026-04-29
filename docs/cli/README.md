# HyperTide CLI 使用说明

`ht` 是 HyperTide 的命令行客户端，用来和 `hypertide` 服务端交互，完成登录、分支切换、元数据同步、工作区检出、资产暂存、保存进度、检查点恢复、提交、审批晋升、锁、治理审计、回滚和分块上传。

按当前实现，HyperTide 的定位是中心化资产版本系统：

- 版本真相在服务端
- CLI 负责本地 workspace、对象缓存和提交流程
- 不提供 Git 式本地仓库、本地 DAG、离线 commit 或 merge/rebase

## 前置条件

在使用 CLI 之前，请先确认：

1. 你有一个可访问的 HyperTide 服务地址，例如 `http://localhost:3000`
2. 你有一个可用的 API Key 或 dev token
3. 目标服务已经初始化完成，并能响应 `/v2/*` 接口

在仓库根目录运行 CLI 的推荐方式：

```powershell
cargo run -p hypertide-cli --bin ht -- --help
```

CLI 的唯一维护实现位于 `crates/cli/src/main.rs`，workspace 二进制名是 `ht`。根目录历史源码不作为当前 CLI 发布入口。

## 当前支持的命令

```text
login
branch
add
remove
save
checkpoint
changeset
lock
trust
submit
log
rollback
sync
checkout
status
diff
chunk-upload
```

## 快速开始

### 1. 登录

JWT 交换模式：

```powershell
cargo run -p hypertide-cli --bin ht -- login `
  --server http://localhost:3000 `
  --token dev-master-key `
  --repo demo-repo `
  --branch main
```

直接使用 API Key：

```powershell
cargo run -p hypertide-cli --bin ht -- login `
  --server http://localhost:3000 `
  --token dev-master-key `
  --api-key-direct `
  --repo demo-repo `
  --branch main
```

`login` 会写入当前目录下的 `.hypertide/profile.json`。非 `--api-key-direct` 模式下，CLI 会自动刷新 token；刷新失效时需要重新登录。

### 2. 同步分支元数据

```powershell
cargo run -p hypertide-cli --bin ht -- sync --repo demo-repo --branch main
```

如果需要把分支同步到某个特定的 changeset，可以加上 `--to <changeset-id>`。

`sync` 只更新服务端快照元数据和本地基线，不会把文件写到工作目录。

### 3. 检出工作区

```powershell
cargo run -p hypertide-cli --bin ht -- checkout --repo demo-repo --branch main
```

如果需要检出到某个特定的 changeset，可以加上 `--to <changeset-id>`。

`checkout` 会读取服务端快照，把 blob materialize 到当前目录，并更新 `.hypertide/workspace.json`。检出时会优先命中 `.hypertide/cache/objects/<hash>` 本地缓存；缓存未命中时再调用下载接口。

### 4. 暂存本地文件

推荐直接使用 `--file`：

```powershell
cargo run -p hypertide-cli --bin ht -- add `
  --file .\Content\Props\tree.uasset `
  --path Content/Props/tree.uasset `
  --branch main
```

CLI 会自动读取文件、上传内容、拿到最终 `blob_hash`，并写入 `.hypertide/stage.json`。

兼容层仍保留底层方式：

```powershell
cargo run -p hypertide-cli --bin ht -- add `
  --path Content/Props/tree.uasset `
  --blob <blob-hash> `
  --branch main
```

### 5. 查看状态与差异

```powershell
cargo run -p hypertide-cli --bin ht -- status --repo demo-repo --branch main
cargo run -p hypertide-cli --bin ht -- diff --repo demo-repo --branch main
```

`status` 输出的是资产级状态，不做源码行级 diff。当前可能出现：

- `unmodified`
- `modified`
- `added`
- `deleted`
- `staged`
- `locked_by_other`
- `stale_base`

`diff` 输出 base、local、staged 三组 blob hash，对应资产级比较。

### 6. 提交

```powershell
cargo run -p hypertide-cli --bin ht -- submit `
  --repo demo-repo `
  --branch main `
  --message "update tree asset"
```

如果当前 `stage.json` 为空，提交会失败。提交错误会尽量区分：

- 锁冲突
- `BaseChangesetMismatch`
- 缺失 blob
- 鉴权失败

### 7. 保存进度与检查点

`save` 和 `checkpoint` 是 agent 高频恢复层，不会推进正式 branch head：

```powershell
cargo run -p hypertide-cli --bin ht -- save --repo demo-repo --branch main --message "agent pass 1"
cargo run -p hypertide-cli --bin ht -- checkpoint create --repo demo-repo --branch main --message "before texture rewrite"
cargo run -p hypertide-cli --bin ht -- checkpoint list
cargo run -p hypertide-cli --bin ht -- checkpoint restore --id <checkpoint-id>
cargo run -p hypertide-cli --bin ht -- checkpoint branch --id <checkpoint-id> --name try/alt-plan
```

从 checkpoint 提交候选版本：

```powershell
cargo run -p hypertide-cli --bin ht -- submit --repo demo-repo --branch main --from-checkpoint <checkpoint-id> --visibility draft
```

### 8. 正式版本晋升

`changeset` 分组用于 draft/approve/promote/gate 这条正式版本链：

```powershell
cargo run -p hypertide-cli --bin ht -- changeset gate --repo demo-repo --id <changeset-id>
cargo run -p hypertide-cli --bin ht -- changeset approve --repo demo-repo --id <changeset-id>
cargo run -p hypertide-cli --bin ht -- changeset promote --repo demo-repo --id <changeset-id>
```

如果服务端启用了高风险签名，`promote` 可传 `--high-risk-secret`，或设置 `HT_HIGH_RISK_SIGNING_SECRET` 环境变量。

### 9. 锁

`lock` 分组用于资产编辑锁：

```powershell
cargo run -p hypertide-cli --bin ht -- lock acquire --path Content/Props/tree.uasset
cargo run -p hypertide-cli --bin ht -- lock renew --path Content/Props/tree.uasset
cargo run -p hypertide-cli --bin ht -- lock list
cargo run -p hypertide-cli --bin ht -- lock release --path Content/Props/tree.uasset
```

管理员强制释放锁是高风险操作：

```powershell
cargo run -p hypertide-cli --bin ht -- lock force-release --path Content/Props/tree.uasset --high-risk-secret <secret>
```

### 10. Trust / Audit / Replay

`trust checkpoint` 是审计链/系统状态证明，和 agent session checkpoint 不是同一个概念：

```powershell
cargo run -p hypertide-cli --bin ht -- trust checkpoint generate
cargo run -p hypertide-cli --bin ht -- trust checkpoint latest
cargo run -p hypertide-cli --bin ht -- trust witness attest --checkpoint <trust-checkpoint-id> --witness witness-a
cargo run -p hypertide-cli --bin ht -- trust witness summary --checkpoint <trust-checkpoint-id>
cargo run -p hypertide-cli --bin ht -- trust witness topology
cargo run -p hypertide-cli --bin ht -- trust audit verify
cargo run -p hypertide-cli --bin ht -- trust audit export --limit 100
cargo run -p hypertide-cli --bin ht -- trust replay verify
cargo run -p hypertide-cli --bin ht -- trust replay readiness
cargo run -p hypertide-cli --bin ht -- trust retention policy
```

## 常见工作流

### 新建并切换分支

```powershell
cargo run -p hypertide-cli --bin ht -- branch create --repo demo-repo --name feature/props-cleanup
cargo run -p hypertide-cli --bin ht -- branch switch --repo demo-repo --name feature/props-cleanup
```

如果要从指定分支创建新分支，可以在 `branch create` 时加上 `--from <branch>`。

### 列出分支

```powershell
cargo run -p hypertide-cli --bin ht -- branch list --repo demo-repo
```

### 删除一个资产

```powershell
cargo run -p hypertide-cli --bin ht -- remove --path Content/Props/old-tree.uasset --branch main
```

删除会先写入 stage；真正的版本删除发生在 `submit` 时。

### 查看历史

```powershell
cargo run -p hypertide-cli --bin ht -- log --repo demo-repo --branch main --limit 20
```

### 回滚到指定 changeset

```powershell
cargo run -p hypertide-cli --bin ht -- rollback `
  --repo demo-repo `
  --branch main `
  --to cs_123456 `
  --author release-bot `
  --message "rollback broken asset update"
```

`--author` 可选，用于显式指定回滚作者。

### 分块上传

默认模式会上传 chunk、创建 manifest、在服务端 compose 出最终 blob，并打印 `blob_hash`：

```powershell
cargo run -p hypertide-cli --bin ht -- chunk-upload `
  --file .\LargeAsset.pak `
  --chunk-size 4194304 `
  --chunk-size-policy fixed-4m
```

如果只想做底层预上传，不进入版本流，可以使用：

```powershell
cargo run -p hypertide-cli --bin ht -- chunk-upload `
  --file .\LargeAsset.pak `
  --manifest-only
```

`chunk-upload` 不会自动提交 changeset；如果你想把文件纳入版本流，优先使用 `ht add --file ...`。

## 本地状态文件

CLI 会在当前工作目录下创建 `.hypertide`，而不是写入全局用户目录。

### `.hypertide/profile.json`

保存：

- `server`
- `api_key`
- JWT 交换得到的 token 信息
- 当前默认 `repo`
- 当前默认 `branch`

### `.hypertide/stage.json`

保存：

- 当前 branch
- 当前 staged 资产列表
- 当前提交基线 `base_changeset_id`

### `.hypertide/workspace.json`

保存：

- `repo_id`
- `branch`
- `workspace_root`
- `base_changeset_id`
- 已检出资产列表
- 最后同步时间

### `.hypertide/cache/objects/<hash>`

保存已下载或已上传对象的本地缓存，用于加速重复检出和重复上传。

## 命令速查

### `login`

```powershell
cargo run -p hypertide-cli --bin ht -- login --server http://localhost:3000 --token dev-master-key
```

关键参数：`--server`、`--token`、`--api-key-direct`、`--repo`、`--branch`

### `branch create`

```powershell
cargo run -p hypertide-cli --bin ht -- branch create --repo demo-repo --name feature/test
```

关键参数：`--repo`、`--name`、`--from`

### `branch list`

```powershell
cargo run -p hypertide-cli --bin ht -- branch list --repo demo-repo
```

### `branch switch`

```powershell
cargo run -p hypertide-cli --bin ht -- branch switch --repo demo-repo --name main
```

### `add`

```powershell
cargo run -p hypertide-cli --bin ht -- add --file .\asset.bin --path Content/asset.bin --branch main
```

或：

```powershell
cargo run -p hypertide-cli --bin ht -- add --path Content/asset.bin --blob <blob-hash> --branch main
```

### `remove`

```powershell
cargo run -p hypertide-cli --bin ht -- remove --path Content/asset.bin --branch main
```

兼容说明：旧参数名 `--asset-path` 仍可作为 `--path` 的别名使用，但新文档和帮助页统一使用 `--path`。

### `sync`

```powershell
cargo run -p hypertide-cli --bin ht -- sync --repo demo-repo --branch main
```

### `checkout`

```powershell
cargo run -p hypertide-cli --bin ht -- checkout --repo demo-repo --branch main
```

### `status`

```powershell
cargo run -p hypertide-cli --bin ht -- status --repo demo-repo --branch main
```

### `diff`

```powershell
cargo run -p hypertide-cli --bin ht -- diff --repo demo-repo --branch main
```

### `submit`

```powershell
cargo run -p hypertide-cli --bin ht -- submit --repo demo-repo --branch main --message "update asset"
```

### `log`

```powershell
cargo run -p hypertide-cli --bin ht -- log --repo demo-repo --branch main --limit 20
```

### `rollback`

```powershell
cargo run -p hypertide-cli --bin ht -- rollback --repo demo-repo --branch main --to cs_123456 --message "rollback bad release"
```

### `chunk-upload`

```powershell
cargo run -p hypertide-cli --bin ht -- chunk-upload --file .\LargeAsset.pak
```

## 常见问题

### `repo not set`

没有在 `login` 时写入默认 repo，也没有在命令里传 `--repo`。重新执行 `login --repo ...` 或者直接给命令补 `--repo`。

### token 过期

JWT 模式下，CLI 会自动刷新。如果 refresh token 已失效，请重新执行 `ht login`。

### `sync` 之后没有看到文件

这是当前设计。`sync` 只更新元数据基线；真正把文件写到工作目录的是 `checkout`。

### 提交时报 `stale base`

说明本地 `base_changeset_id` 已落后于当前分支 head。先执行 `sync` 或 `checkout`，确认最新基线后再重新暂存并提交。

### 为什么没有 merge / rebase / 本地 commit

按当前实现，HyperTide 是中心化资产版本系统，不是分布式源码 VCS。它依赖服务端维护版本真相，并通过锁、`BaseChangesetMismatch`、`draft/approve/promote` gate 做协作控制。
