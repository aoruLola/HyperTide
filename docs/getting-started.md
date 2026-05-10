# HyperTide 快速入门

本指南帮助你在 5 分钟内从零开始使用 HyperTide。

## 第一步：安装

### 预编译 CLI

如果 [GitHub Releases](https://github.com/openLYURA/HyperTide/releases) 已提供与你的平台匹配的预编译包，请优先下载并把 `ht` 加入 `PATH`。

### 从源码构建

```bash
git clone https://github.com/openLYURA/HyperTide.git
cd HyperTide
cargo build --release
```

构建完成后：
- CLI：`target/release/ht`
- 服务端：`target/release/hypertide`

### Docker 部署服务端

```bash
docker compose -f deploy/server/docker-compose.yml --env-file deploy/server/.env.example up -d
```

服务端启动后监听 `http://localhost:3000`。

## 第二步：登录

```bash
ht login --server http://localhost:3000 --token dev-master-key --repo my-repo --branch main
```

这会在当前目录创建 `.hypertide/profile.json`，保存你的凭据和默认配置。

验证登录状态：

```bash
ht doctor
```

应该看到 `[ok] login` 和 `[ok] server`。

## 第三步：拉取资产

```bash
ht sync
ht checkout
```

- `sync` 从服务端获取最新的分支元数据
- `checkout` 把资产文件下载到当前工作目录

现在你可以看到服务端的资产文件了。

## 第四步：修改并提交

编辑任意资产文件后：

```bash
# 查看哪些文件被修改
ht status

# 暂存修改
ht add --file Content/Props/tree.uasset

# 提交
ht submit --message "update tree prop"
```

提交成功后，你的修改就正式记录在服务端了。

## 第五步：协作

### 文件锁定

编辑前先锁定，防止冲突：

```bash
ht lock acquire --path Content/Props/tree.uasset
# 编辑文件...
ht add --file Content/Props/tree.uasset
ht submit --message "update tree"
ht lock release --path Content/Props/tree.uasset
```

### 分支

```bash
# 创建新分支
ht branch create --name feature/new-textures

# 切换分支
ht branch switch --name feature/new-textures

# 在新分支上工作...
ht add --file Content/Textures/grass.png
ht submit --message "add grass texture"
```

### 审批流程

```bash
# 检查是否可以晋升
ht changeset gate --id <changeset-id>

# 审批
ht changeset approve --id <changeset-id>

# 晋升为可见版本
ht changeset promote --id <changeset-id>
```

## 进阶用法

### 查看历史

```bash
# 文本格式
ht log --limit 10

# 图形格式
ht log --graph --limit 10
```

### 检查点（Agent 工作流）

```bash
# 保存进度
ht save --message "pass 1 complete"

# 创建检查点
ht checkpoint create --message "before risky change"

# 出错后恢复
ht checkpoint restore --id <checkpoint-id>
```

### JSON 输出

所有命令支持 `--json` 便于脚本处理：

```bash
ht status --json
ht log --json --limit 5
ht stage list --json
```

### Shell 补全

```bash
# Bash
ht completions bash > ~/.bash_completion.d/ht

# Zsh
ht completions zsh > ~/.zfunc/_ht

# Fish
ht completions fish > ~/.config/fish/completions/ht.fish

# PowerShell
ht completions powershell > $PROFILE
```

## 下一步

- [CLI 新手用户指南](cli/user-guide.md) — 文件同步、检出、暂存、提交、差异和排错全链路
- [CLI 完整参考](cli/README.md) — 所有命令的详细参数
- [架构概览](architecture.md) — 了解 HyperTide 的设计
- [部署指南](../deploy/server/README.md) — 生产环境部署
