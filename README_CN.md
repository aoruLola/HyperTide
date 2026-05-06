# HyperTide

[English](README.md)

**面向大型二进制资产的中心化版本控制系统。**

HyperTide 专为游戏资源、美术文件、构建产物等不适合 Git 的大型二进制资产而设计。它提供服务端维护的版本真相、文件锁、审批工作流和完整审计链，通过简洁的 CLI 操作。

## 为什么选择 HyperTide

| | Git | Git LFS | Perforce | **HyperTide** |
|---|---|---|---|---|
| 大文件支持 | 差 | 一般 | 好 | **优秀** |
| 文件锁 | 无 | 无 | 有 | **有** |
| 审批工作流 | 无 | 无 | 有限 | **完整（gate → approve → promote）** |
| 审计链 | 无 | 无 | 基础 | **BLAKE3 哈希链 + 见证者** |
| Agent 支持 | 无 | 无 | 无 | **原生（session、checkpoint）** |
| 自托管 | 是 | 是 | 昂贵 | **是（Docker）** |
| 开源 | 是 | 是 | 否 | **是（MIT）** |

## 快速开始

### 环境要求

- Rust 1.85.1+
- PostgreSQL 15+
- Docker（可选，用于容器部署）

### 构建

```bash
git clone https://github.com/openLYURA/HyperTide.git
cd HyperTide
cargo build --release
```

CLI 二进制：`target/release/ht`，服务端二进制：`target/release/hypertide`

### Docker 部署

```bash
docker compose -f deploy/server/docker-compose.yml --env-file deploy/server/.env.example up -d
```

### 开始使用

```bash
# 登录
ht login --server http://localhost:3000 --token dev-master-key

# 为当前工作区创建或选择仓库
ht init --repo my-repo

# 健康检查
ht doctor

# 拉取最新资产
ht sync
ht checkout

# 编辑文件后暂存并提交
ht add --file Content/Props/tree.uasset
ht status
ht submit --message "update tree prop"
```

## CLI 命令

### 核心工作流

| 命令 | 说明 |
|---|---|
| `ht login` | 保存服务端凭据和默认配置 |
| `ht init` | 创建/选择仓库并初始化本地工作区 |
| `ht repo` | 创建、列出、查看或选择仓库 |
| `ht sync` | 同步本地元数据到服务端快照 |
| `ht checkout` | 拉取服务端资产到工作区 |
| `ht add` | 暂存本地文件 |
| `ht remove` | 暂存资产删除 |
| `ht submit` | 创建正式 changeset |
| `ht status` | 查看资产状态（修改/暂存/锁定） |
| `ht diff` | 查看哈希差异 |
| `ht log` | 查看提交历史（`--graph` 可视化） |
| `ht rollback` | 回滚到历史版本 |

### 暂存区

| 命令 | 说明 |
|---|---|
| `ht stage list` | 查看暂存内容 |
| `ht stage clear` | 清空暂存区 |

### 分支

| 命令 | 说明 |
|---|---|
| `ht branch create` | 创建分支 |
| `ht branch list` | 列出分支 |
| `ht branch switch` | 切换默认分支 |

### 锁

| 命令 | 说明 |
|---|---|
| `ht lock acquire` | 锁定文件 |
| `ht lock release` | 释放锁 |
| `ht lock renew` | 续期锁 |
| `ht lock list` | 列出所有锁 |
| `ht lock force-release` | 管理员强制释放锁 |

### 检查点（Agent 工作流）

| 命令 | 说明 |
|---|---|
| `ht save` | 保存工作进度（不推进分支） |
| `ht checkpoint create` | 创建可恢复的工作区检查点 |
| `ht checkpoint restore` | 恢复到检查点 |
| `ht checkpoint branch` | 从检查点创建分支 |
| `ht checkpoint list` | 列出检查点 |

### 治理

| 命令 | 说明 |
|---|---|
| `ht changeset gate` | 检查 changeset 晋升就绪状态 |
| `ht changeset approve` | 审批 changeset |
| `ht changeset promote` | 晋升 changeset 为可见版本 |
| `ht trust checkpoint` | 生成/查看系统状态证明 |
| `ht trust witness` | 见证者证明操作 |
| `ht trust audit` | 审计链验证和导出 |
| `ht trust replay` | 事件回放验证 |
| `ht trust retention` | 保留策略查询 |

### 工具

| 命令 | 说明 |
|---|---|
| `ht doctor` | 检查登录、连通性和工作区健康 |
| `ht completions` | 生成 Shell 补全脚本 |
| `ht chunk-upload` | 大文件分块上传 |

所有命令支持 `--json` 结构化输出和 `--help` 详细帮助。

## 架构

```
HyperTide
├── crates/server    → hypertide-server (binary: hypertide)
├── crates/cli       → hypertide-cli (binary: ht)
├── migrations/      → PostgreSQL 数据库迁移
├── deploy/          → Docker 和打包脚本
└── docs/            → 设计文档和规格说明
```

开源仓库只包含 Server 和 CLI。服务端提供 REST API 处理所有版本操作。CLI 将工作区操作转换为 API 调用，本地状态保存在 `.hypertide/` 目录。服务端是唯一的版本真相源——没有本地 DAG 或离线提交。桌面或 Web UI 不在公开 Community Edition 仓库范围内。

## 核心设计

- **内容寻址存储** — 文件按 BLAKE3 哈希存储，自动去重
- **服务端分支头** — 无分叉本地历史，过期提交被拒绝
- **四阶段 changeset 生命周期** — draft → approve → promote → visible
- **审计链** — BLAKE3 哈希链 + 见证者证明，完全可验证
- **高风险操作签名** — HMAC-SHA256 + nonce + 时间戳，防重放

## 部署

详见 [deploy/server/README.md](deploy/server/README.md)（Docker 部署）和 [deploy/cli/README.md](deploy/cli/README.md)（CLI 打包）。

```bash
docker compose -f deploy/server/docker-compose.yml --env-file deploy/server/.env.example up -d
```

## 文档

- [快速入门](docs/getting-started.md) — 5 分钟上手指南
- [CLI 参考](docs/cli/README.md) — 所有命令的详细参数和示例
- [服务端指南](docs/server/README.md) — 服务端配置、API Key 管理、生产部署
- [架构概览](docs/architecture.md) — 系统架构和设计决策
- [API 规格](docs/api/openapi.yaml) — OpenAPI 规格
- [Docker 部署](deploy/server/README.md) — Docker 部署指南
- [贡献指南](CONTRIBUTING.md) — 如何参与贡献
- [安全报告](SECURITY.md) — 安全问题报告

## 贡献

详见 [CONTRIBUTING.md](CONTRIBUTING.md)。简而言之：Fork → 创建分支 → 提交 PR。

## 安全

安全问题请通过 [SECURITY.md](SECURITY.md) 私下报告，不要公开披露。

## 许可证

[MIT](LICENSE)
