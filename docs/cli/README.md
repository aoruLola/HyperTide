# HyperTide CLI 使用说明

`ht` 是 HyperTide 的命令行客户端，用来和 `hypertide` 服务端交互，完成登录、分支切换、元数据同步、工作区检出、资产暂存、保存进度、检查点恢复、提交、审批晋升、锁、治理审计、回滚和分块上传。

HyperTide 的定位是中心化资产版本系统：

- 版本真相在服务端
- CLI 负责本地 workspace、对象缓存和提交流程
- 不提供 Git 式本地仓库、本地 DAG、离线 commit 或 merge/rebase

## 前置条件

1. 一个可访问的 HyperTide 服务地址，例如 `http://localhost:3000`
2. 一个可用的 API Key 或 dev token
3. 目标服务已初始化完成，并能响应 `/v2/*` 接口

第一次使用建议先读 [CLI 新手用户指南](user-guide.md)，再把本文作为完整命令参考。

```powershell
# 查看帮助
cargo run -p hypertide-cli --bin ht -- --help
```

## 命令总览

```text
login           保存服务端凭据和默认配置
init            为当前工作区创建或选择仓库
repo            创建、列出、查看或选择仓库
branch          创建、列出、切换分支
sync            同步本地元数据到服务端快照
checkout        拉取服务端资产到工作区
add             暂存本地文件
remove          暂存资产删除
submit          创建正式 changeset
status          查看资产状态
diff            查看哈希差异
log             查看提交历史（支持 --graph）
rollback        回滚到历史版本
stage           查看或清空暂存区
save            保存工作进度（不推进分支）
checkpoint      创建、恢复、分支检查点
changeset       审批、晋升 changeset
lock            锁管理
trust           审计、见证、回放、保留策略
doctor          健康检查
completions     生成 Shell 补全脚本
chunk-upload    大文件分块上传
```

## 全局选项

所有命令支持以下全局选项：

| 选项 | 说明 |
|---|---|
| `--json` | 以 JSON 格式输出（便于脚本处理） |
| `--help` | 显示命令帮助 |

---

## 快速开始

### 1. 登录

```powershell
# JWT 模式（推荐）
ht login --server http://localhost:3000 --token dev-master-key

# 直接 API Key 模式
ht login --server http://localhost:3000 --token dev-master-key --api-key-direct
```

`login` 写入 `.hypertide/profile.json`。JWT 模式下 CLI 自动刷新 token；刷新失败需重新登录。`login` 不创建仓库，仓库初始化由 `ht init` 或 `ht repo create` 完成。

### 2. 初始化仓库

```powershell
# 如果仓库不存在则创建；如果已存在则选择它
ht init --repo demo-repo --branch main

# 也可以显式管理仓库
ht repo create game-assets --default-branch main --use
ht repo list
ht repo info game-assets
ht repo use game-assets --branch main
```

`init` 需要先登录。它只创建/选择远端仓库并初始化本地 `.hypertide` 状态，不会检出文件、提交文件或替代登录。

### 3. 健康检查

```powershell
ht doctor
```

输出示例：
```
[ok]   login: server=http://localhost:3000, mode=jwt
[ok]   server: http://localhost:3000 responding
[ok]   default repo: demo-repo
[ok]   default branch: main
[ok]   token: valid (3500s remaining)
[ok]   workspace: 42 assets checked out (branch=main)
[warn] stage: 3 asset(s) pending — run 'ht submit' or 'ht stage clear'

doctor: 6 ok, 1 warning(s), 0 error(s)
```

### 4. 同步与检出

```powershell
# 同步元数据（不写文件）
ht sync --repo demo-repo --branch main

# 检出资产到工作目录
ht checkout --repo demo-repo --branch main
```

`checkout` 会检测本地未提交修改。如果有修改，拒绝覆盖（除非 `--force`）。

### 5. 编辑、暂存、提交

```powershell
# 暂存文件
ht add --file Content/Props/tree.uasset

# 查看状态
ht status

# 提交
ht submit --repo demo-repo --branch main --message "update tree prop"
```

---

## 核心工作流

### 基本资产流程

```powershell
ht login --server http://localhost:3000 --token my-key
ht init --repo game-assets --branch main
ht sync
ht checkout
# ... 编辑文件 ...
ht add --file Content/Props/chair.fbx
ht status
ht submit --message "update chair model"
```

### 分支工作流

```powershell
# 创建分支
ht branch create --repo game-assets --name feature/new-props

# 切换分支
ht branch switch --name feature/new-props

# 列出分支
ht branch list --repo game-assets
```

切换分支时，如果暂存区有未提交的修改，CLI 会拒绝（除非 `--force`）。

### 锁工作流

```powershell
# 锁定文件（防止他人同时编辑）
ht lock acquire --path Content/Props/tree.uasset

# 编辑文件...
ht add --file Content/Props/tree.uasset
ht submit --message "update tree"

# 释放锁
ht lock release --path Content/Props/tree.uasset

# 查看所有锁
ht lock list
```

### 审批工作流

```powershell
# 检查 changeset 是否可以晋升
ht changeset gate --repo game-assets --id <changeset-id>

# 审批
ht changeset approve --repo game-assets --id <changeset-id>

# 晋升为可见版本（高风险操作）
ht changeset promote --repo game-assets --id <changeset-id>
```

### Agent 检查点工作流

```powershell
# 保存进度（不推进分支）
ht save --repo game-assets --branch main --message "agent pass 1"

# 创建检查点
ht checkpoint create --repo game-assets --branch main --message "before rewrite"

# 列出检查点
ht checkpoint list

# 恢复到检查点
ht checkpoint restore --id <checkpoint-id>

# 从检查点创建新分支
ht checkpoint branch --id <checkpoint-id> --name try/alt-plan

# 从检查点提交
ht submit --repo game-assets --branch main --from-checkpoint <checkpoint-id> --visibility draft
```

---

## 命令详解

### `login`

保存服务端凭据和默认配置。

```powershell
ht login --server <url> --token <key> [--api-key-direct] [--repo <repo>] [--branch <branch>]
```

| 参数 | 说明 | 默认值 |
|---|---|---|
| `--server` | 服务端 URL | 必填 |
| `--token` | API Key 或 dev token | 必填 |
| `--api-key-direct` | 直接使用 token 作为 API Key | false |
| `--repo` | 默认仓库 | 无 |
| `--branch` | 默认分支 | `main` |

### `init`

为当前工作区创建或选择仓库。需要先执行 `login`，不会检出文件或提交文件。

```powershell
ht init --repo <repo> [--branch <branch>] [--force]
```

| 参数 | 说明 | 默认值 |
|---|---|---|
| `--repo` | 要创建或选择的仓库 | 必填 |
| `--branch` | 要选择的分支 | `main` |
| `--force` | 暂存区非空时清空暂存区并切换 | false |

### `repo`

管理远端仓库并设置本地默认 repo/branch。

```powershell
ht repo create <repo> [--default-branch main] [--use] [--force]
ht repo list
ht repo info [repo]
ht repo use <repo> [--branch main] [--force]
```

`repo use` 和 `repo create --use` 会验证远端仓库与分支存在。暂存区非空时默认拒绝切换；使用 `--force` 会清空暂存区。

### `sync`

同步本地元数据到服务端快照，不写入资产文件。保留暂存区内容。

```powershell
ht sync [--repo <repo>] [--branch <branch>] [--to <changeset-id>]
```

### `checkout`

拉取服务端资产到工作目录。检测本地修改，拒绝覆盖（除非 `--force`）。

```powershell
ht checkout [--repo <repo>] [--branch <branch>] [--to <changeset-id>] [--force]
```

### `add`

暂存本地文件或已有 blob。

```powershell
# 上传文件并暂存
ht add --file <local-file> [--asset-path <repo-path>] [--branch <branch>]

# 暂存已有 blob
ht add --blob <hash> --asset-path <repo-path> [--branch <branch>]
```

文件超过 8MB 自动走分块上传路径。

### `remove`

暂存资产删除。真正的删除发生在 `submit` 时。

```powershell
ht remove --asset-path <repo-path> [--branch <branch>]
```

### `submit`

创建正式 changeset。

```powershell
ht submit [--repo <repo>] [--branch <branch>] [--message <msg>] [--visibility <vis>] [--from-checkpoint <id>]
```

| 参数 | 说明 | 默认值 |
|---|---|---|
| `--message` | 提交消息 | `submit` |
| `--visibility` | 可见性（`visible`/`draft`） | 自动 |
| `--from-checkpoint` | 从检查点提交 | 无 |

### `status`

查看资产状态。支持 `--json` 输出。

```powershell
ht status [--repo <repo>] [--branch <branch>] [--json]
```

状态类型：
- `unmodified` — 未修改
- `modified` — 本地修改
- `added` — 新增
- `deleted` — 已删除
- `staged` — 已暂存
- `locked_by_other` — 被他人锁定
- `stale_base` — 基线过期

### `log`

查看提交历史。

```powershell
ht log [--repo <repo>] [--branch <branch>] [--limit <n>] [--graph]
```

`--graph` 显示 ASCII 图形化的父子关系：

```
* 5d7e4fb 2026-05-02 fix: CI format fixes
* 8a9f179 2026-05-02 feat: CLI unified
|
* 05235f9 2026-05-01 docs: split readme
```

### `rollback`

回滚到指定 changeset（高风险操作，需确认）。

```powershell
ht rollback --to <changeset-id> [--repo <repo>] [--branch <branch>] [--author <author>] [--message <msg>] [--yes]
```

### `stage`

管理暂存区。

```powershell
# 查看暂存内容
ht stage list [--json]

# 清空暂存区
ht stage clear [--yes]
```

### `doctor`

一键检查登录状态、服务端连通性、默认配置、工作区和暂存区状态。

```powershell
ht doctor
```

### `completions`

生成 Shell 补全脚本。

```powershell
ht completions bash > ~/.bash_completion.d/ht
ht completions zsh > ~/.zfunc/_ht
ht completions fish > ~/.config/fish/completions/ht.fish
ht completions powershell > $PROFILE
ht completions elvish > ~/.elvish/lib/ht.elv
```

### `chunk-upload`

大文件分块上传。

```powershell
ht chunk-upload --file <file> [--chunk-size <bytes>] [--chunk-size-policy <policy>] [--manifest-only]
```

| 参数 | 说明 | 默认值 |
|---|---|---|
| `--chunk-size` | 分块大小（字节） | 4MB |
| `--chunk-size-policy` | 分块策略标签 | `fixed-4m` |
| `--manifest-only` | 只创建 manifest，不 compose | false |

### `trust`

审计、见证、回放和保留策略操作。

```powershell
# 系统状态证明
ht trust checkpoint generate
ht trust checkpoint latest

# 见证者
ht trust witness attest --checkpoint <id> [--witness <id>]
ht trust witness summary --checkpoint <id>
ht trust witness topology

# 审计链
ht trust audit verify
ht trust audit export [--limit <n>] [--before-seq <n>] [--action <action>] [--actor <id>]

# 回放
ht trust replay verify [--from-checkpoint <id>]
ht trust replay readiness

# 保留策略
ht trust retention policy
```

---

## 本地状态

CLI 在当前目录下创建 `.hypertide/`：

| 文件 | 说明 |
|---|---|
| `profile.json` | 服务端地址、凭据、默认 repo/branch |
| `stage.json` | 暂存的资产列表和基线 |
| `workspace.json` | 已检出的资产列表和元数据 |
| `session.json` | 当前 agent session ID |
| `cache/objects/<hash>` | 已下载/上传对象的本地缓存 |

---

## 常见问题

### `repo not set`

没有在 `login` 时设置默认 repo，也没有传 `--repo`。重新 `login --repo ...` 或给命令补 `--repo`。

### token 过期

JWT 模式下 CLI 自动刷新。如果 refresh token 失效，重新 `ht login`。

### `sync` 之后没有文件

这是设计如此。`sync` 只更新元数据；`checkout` 才写文件。

### `stale base`

本地基线落后于分支 head。先 `sync` 或 `checkout` 更新基线。

### `nothing staged`

暂存区为空。先 `ht add --file <file>` 暂存资产。

### checkout 被拒绝

工作区有未提交修改。用 `ht add` 暂存修改，或用 `--force` 强制覆盖。

### branch switch 被拒绝

暂存区有未提交修改。先 `ht submit` 保存，或用 `--force` 强制切换（会清空暂存区）。

### 为什么没有 merge / rebase

HyperTide 是中心化资产版本系统，不是分布式源码 VCS。通过锁、`BaseChangesetMismatch`、`draft/approve/promote` 门禁做协作控制。
