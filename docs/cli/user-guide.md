# HyperTide CLI 新手用户指南

本指南面向第一次使用 `ht` 的用户，覆盖从安装、登录、初始化仓库，到文件同步、检出、暂存、提交、查看差异和处理常见问题的完整链路。审批、晋升和企业治理流程不在本指南范围内。

## 1. 安装 CLI

优先从 [GitHub Releases](https://github.com/openLYURA/HyperTide/releases) 下载与你的平台匹配的预编译 `ht` 包。解压后把 `ht` 放到 `PATH` 中，或在命令里使用完整路径。

如果暂时没有适合的平台包，可以从源码构建：

```bash
git clone https://github.com/openLYURA/HyperTide.git
cd HyperTide
cargo build --release -p hypertide-cli --bin ht
```

构建完成后 CLI 位于：

- Windows: `target/release/ht.exe`
- Linux/macOS: `target/release/ht`

## 2. 准备服务端

本地体验可以用 Docker Compose 启动服务端：

```bash
docker compose -f deploy/server/docker-compose.yml --env-file deploy/server/.env.example up -d
```

默认服务地址是 `http://localhost:3000`。启动后先确认健康状态：

```bash
curl http://localhost:3000/health/live
curl http://localhost:3000/health/ready
```

## 3. 登录

在你的工作目录执行：

```bash
ht login --server http://localhost:3000 --token dev-master-key
```

登录会创建 `.hypertide/profile.json`，保存服务地址、认证信息和默认配置。检查登录状态：

```bash
ht doctor
```

如果你只想直接使用 API Key，不走 JWT 刷新流程：

```bash
ht login --server http://localhost:3000 --token dev-master-key --api-key-direct
```

## 4. 创建或选择仓库

第一次使用某个资产仓库时，运行：

```bash
ht init --repo game-assets --branch main
```

`init` 会在服务端创建或选择仓库，并初始化本地 `.hypertide/` 工作区状态。它不会自动提交文件，也不会覆盖你的本地文件。

常用仓库命令：

```bash
ht repo list
ht repo info game-assets
ht repo use game-assets --branch main
```

## 5. 同步和检出文件

先同步服务端元数据：

```bash
ht sync
```

再把服务端资产检出到当前工作目录：

```bash
ht checkout
```

`checkout` 会保护本地未提交修改。若检测到可能被覆盖的文件，命令会拒绝继续；确认要覆盖时再显式使用 `--force`。

## 6. 添加文件

把本地文件加入暂存区：

```bash
ht add --file Content/Props/tree.uasset
```

如果本地路径和希望保存到服务端的资产路径不同，可以指定 `--asset-path`：

```bash
ht add --file ./exports/tree.uasset --asset-path Content/Props/tree.uasset
```

查看当前状态：

```bash
ht status
```

查看本地与服务端快照的哈希差异：

```bash
ht diff
```

## 7. 提交文件

确认暂存区后提交：

```bash
ht submit --message "update tree prop"
```

提交成功后，服务端会记录新的 changeset，分支头也会更新。其他用户随后可以通过 `ht sync` 和 `ht checkout` 获取你的修改。

## 8. 删除文件

删除服务端资产需要先暂存删除：

```bash
ht remove --path Content/Props/old-tree.uasset
ht status
ht submit --message "remove old tree prop"
```

`remove` 表示在 HyperTide 中记录资产删除，不等同于直接清理你的磁盘目录。

## 9. 拉取别人提交的更新

日常更新流程：

```bash
ht sync
ht checkout
ht status
```

如果本地有未提交修改，先用 `ht status` 和 `ht diff` 检查，再决定提交、清空暂存区或手动整理文件。

## 10. 分支

创建分支：

```bash
ht branch create --name feature/new-props
```

切换分支：

```bash
ht branch switch --name feature/new-props
ht sync
ht checkout
```

查看分支：

```bash
ht branch list
```

切换分支前建议保持暂存区干净，避免把一个分支的本地修改带到另一个分支。

## 11. 文件锁

多人协作修改二进制资产时，建议编辑前先加锁：

```bash
ht lock acquire --path Content/Props/tree.uasset
```

修改并提交后释放：

```bash
ht add --file Content/Props/tree.uasset
ht submit --message "update locked tree prop"
ht lock release --path Content/Props/tree.uasset
```

查看锁：

```bash
ht lock list
```

## 12. 历史和回滚

查看历史：

```bash
ht log --limit 10
ht log --graph --limit 10
```

回滚到指定 changeset：

```bash
ht rollback --to <changeset-id> --message "rollback bad asset update"
```

回滚会创建新的服务端记录，不会静默改写历史。

## 13. 暂存区维护

查看暂存区：

```bash
ht stage list
```

确认不需要当前暂存内容时清空：

```bash
ht stage clear
```

清空暂存区只影响 `.hypertide/` 中的待提交记录，不会删除你的工作目录文件。

## 14. 常见排错

```bash
ht doctor
```

优先看：

- `login`: 是否已登录，token 是否有效。
- `server`: 服务端是否可访问。
- `default repo`: 当前默认仓库是否存在。
- `workspace`: 当前目录是否是 HyperTide 工作区。
- `stage`: 是否有未提交暂存内容。

需要脚本集成时，给命令加 `--json`：

```bash
ht status --json
ht log --json --limit 5
ht stage list --json
```

下一步可以阅读 [CLI Reference](README.md) 了解完整命令参数。
