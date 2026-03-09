# HyperTide CLI 使用说明

`ht` 是 HyperTide 的命令行客户端，用来和 `hypertide` 服务端交互，完成登录、分支切换、快照同步、工作区检出、资产暂存、提交、回滚和分块上传。

按当前实现，HyperTide 的定位是“中心化资产版本系统”：

- 版本真相在服务端
- CLI 负责本地 workspace、缓存和提交流程
- 不提供 Git 式本地仓库、本地 DAG、离线 commit 或 merge/rebase

## 1. 前置条件

使用 CLI 之前，请先确认：

1. 你有一个可访问的 HyperTide 服务地址，例如 `http://localhost:3000`
2. 你有一个可用的 API Key
3. 目标服务已经完成初始化，能够响应 `/v2/*` 接口

如果你在本地开发环境运行 CLI，推荐在仓库根目录通过 Cargo workspace 方式调用：

```powershell
cargo run -p hypertide-cli --bin ht -- --help
```

## 2. 当前支持的命令

```text
login
branch
add
remove
submit
log
rollback
sync
checkout
status
diff
chunk-upload
```

## 3. 快速开始

### 3.1 登录

推荐使用 JWT 交换模式：

```powershell
cargo run -p hypertide-cli --bin ht -- login `
  --server http://localhost:3000 `
  --token dev-master-key `
  --repo demo-repo `
  --branch main
```

如果目标环境要求直接使用 API Key，可以改为：

```powershell
cargo run -p hypertide-cli --bin ht -- login `
  --server http://localhost:3000 `
  --token dev-master-key `
  --api-key-direct `
  --repo demo-repo `
  --branch main
```

`login` 会写入当前目录下的 `.hypertide/profile.json`。非 `--api-key-direct` 模式下，CLI 会自动刷新 access token；刷新失败时需要重新登录。

### 3.2 同步当前分支元数据

`sync` 只同步服务端快照元数据，并把当前分支 head 写入本地 `stage.json` 作为基线，不会把文件检出到工作目录：

```powershell
cargo run -p hypertide-cli --bin ht -- sync --repo demo-repo --branch main
```

### 3.3 检出工作区

`checkout` 会读取 `/v2/sync/{repo_id}` 快照，把快照中的 blob materialize 到当前目录，并写入 `.hypertide/workspace.json`：

```powershell
cargo run -p hypertide-cli --bin ht -- checkout --repo demo-repo --branch main
```

检出时会优先命中 `.hypertide/cache/objects/<hash>` 本地对象缓存；缓存未命中时再调用 `/v2/storage/download/{hash}`。

### 3.4 暂存本地文件

高层工作流推荐直接使用 `--file`：

```powershell
cargo run -p hypertide-cli --bin ht -- add `
  --file .\Content\Props\tree.uasset `
  --asset-path Content/Props/tree.uasset
```

CLI 会自动：

1. 读取本地文件
2. 小文件走 `/v2/storage/upload`
3. 大文件走 `chunk -> manifest -> compose`
4. 把最终 `blob_hash` 写入 `.hypertide/stage.json`

兼容层仍保留低级接口：

```powershell
cargo run -p hypertide-cli --bin ht -- add `
  --path Content/Props/tree.uasset `
  --blob <blob-hash>
```

### 3.5 查看状态和差异

```powershell
cargo run -p hypertide-cli --bin ht -- status
cargo run -p hypertide-cli --bin ht -- diff
```

`status` 输出的是资产级状态，不做源码行级 diff。当前可能出现：

- `unmodified`
- `modified`
- `added`
- `deleted`
- `staged`
- `locked_by_other`
- `stale_base`

`diff` 输出 base/local/staged 三组 blob hash，对应资产级比较。

### 3.6 提交

```powershell
cargo run -p hypertide-cli --bin ht -- submit `
  --repo demo-repo `
  --branch main `
  --message "update tree asset"
```

如果当前 `stage.json` 为空，提交会失败。提交错误会尽量区分：

- 锁冲突
- `BaseChangesetMismatch`（基线过期）
- 缺失 blob
- 鉴权失败

## 4. 常见工作流

### 4.1 新建并切换分支

```powershell
cargo run -p hypertide-cli --bin ht -- branch create --repo demo-repo --name feature/props-cleanup
cargo run -p hypertide-cli --bin ht -- branch switch --repo demo-repo --name feature/props-cleanup
```

切换分支时，CLI 会重置当前目录 `.hypertide/stage.json` 的分支和基线，避免把旧分支的 staged 变更带到新分支。

### 4.2 删除一个资产

```powershell
cargo run -p hypertide-cli --bin ht -- remove --asset-path Content/Props/old-tree.uasset
```

删除会写入 stage；真正的版本删除发生在 `submit` 时。

### 4.3 查看历史

```powershell
cargo run -p hypertide-cli --bin ht -- log --repo demo-repo --branch main --limit 20
```

### 4.4 回滚到指定 changeset

```powershell
cargo run -p hypertide-cli --bin ht -- rollback `
  --repo demo-repo `
  --branch main `
  --to cs_123456 `
  --message "rollback broken asset update"
```

### 4.5 分块上传

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

`chunk-upload` 不会自动提交 changeset；如果你希望把文件纳入版本流，推荐直接用 `ht add --file ...`。

## 5. 本地状态文件

CLI 会在当前工作目录下创建 `.hypertide`，而不是写入全局用户目录。

### 5.1 `.hypertide/profile.json`

保存：

- `server`
- `api_key`
- JWT 交换得到的 token 信息
- 当前默认 `repo`
- 当前默认 `branch`

### 5.2 `.hypertide/stage.json`

保存：

- 当前 branch
- 当前 staged 资产列表
- 当前 submit 基线 `base_changeset_id`

### 5.3 `.hypertide/workspace.json`

保存：

- `repo_id`
- `branch`
- `workspace_root`
- `base_changeset_id`
- 已检出资产列表
- 最后同步时间

### 5.4 `.hypertide/cache/objects/<hash>`

保存已下载或已上传对象的本地缓存，用于加速重复检出和重复上传。

## 6. 命令速查

### 6.1 `login`

```powershell
cargo run -p hypertide-cli --bin ht -- login --server http://localhost:3000 --token dev-master-key
```

关键参数：

- `--server`
- `--token`
- `--api-key-direct`
- `--repo`
- `--branch`

### 6.2 `branch create`

```powershell
cargo run -p hypertide-cli --bin ht -- branch create --repo demo-repo --name feature/test
```

### 6.3 `branch list`

```powershell
cargo run -p hypertide-cli --bin ht -- branch list --repo demo-repo
```

### 6.4 `branch switch`

```powershell
cargo run -p hypertide-cli --bin ht -- branch switch --repo demo-repo --name main
```

### 6.5 `add`

高层模式：

```powershell
cargo run -p hypertide-cli --bin ht -- add --file .\asset.bin --asset-path Content/asset.bin
```

兼容模式：

```powershell
cargo run -p hypertide-cli --bin ht -- add --path Content/asset.bin --blob <blob-hash>
```

### 6.6 `remove`

```powershell
cargo run -p hypertide-cli --bin ht -- remove --asset-path Content/asset.bin
```

### 6.7 `sync`

```powershell
cargo run -p hypertide-cli --bin ht -- sync --repo demo-repo --branch main
```

### 6.8 `checkout`

```powershell
cargo run -p hypertide-cli --bin ht -- checkout --repo demo-repo --branch main
```

### 6.9 `status`

```powershell
cargo run -p hypertide-cli --bin ht -- status
```

### 6.10 `diff`

```powershell
cargo run -p hypertide-cli --bin ht -- diff
```

### 6.11 `submit`

```powershell
cargo run -p hypertide-cli --bin ht -- submit --message "update asset"
```

### 6.12 `log`

```powershell
cargo run -p hypertide-cli --bin ht -- log --repo demo-repo --branch main --limit 20
```

### 6.13 `rollback`

```powershell
cargo run -p hypertide-cli --bin ht -- rollback --to cs_123456 --message "rollback bad release"
```

### 6.14 `chunk-upload`

```powershell
cargo run -p hypertide-cli --bin ht -- chunk-upload --file .\LargeAsset.pak
```

## 7. 常见问题

### 7.1 `repo not set`

没有在 `login` 时写入默认 repo，也没有在命令里传 `--repo`。重新执行 `login --repo ...` 或直接给命令补 `--repo`。

### 7.2 token 过期

JWT 模式下，CLI 会自动刷新。若 refresh token 已失效，请重新执行 `ht login`。

### 7.3 `sync` 之后没有看到文件

这是当前设计。`sync` 只更新元数据基线；真正把文件写到工作目录的是 `checkout`。

### 7.4 提交时报 `stale base`

说明本地 `base_changeset_id` 落后于当前分支 head。先执行 `sync` 或 `checkout`，确认最新基线后再重新暂存并提交。

### 7.5 为什么没有 merge / rebase / 本地 commit

按当前实现，HyperTide 是中心化资产版本系统，不是分布式源码 VCS。它依赖服务端维护版本真相，并通过锁、`BaseChangesetMismatch`、`draft/approve/promote` gate 来做协作控制。
