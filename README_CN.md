# HyperTide

[English README](README.md)

HyperTide 是一个面向大型二进制资产工作流的中心化协作与版本系统。它把版本真相、分支状态、锁、审批、审计和回放能力放在服务端，由 CLI 和后续 UI 提供面向工作区的操作体验。

> 本项目基于 [MIT 许可证](LICENSE) 开源。

## 目录

- [产品定位](#产品定位)
- [核心能力](#核心能力)
- [架构概览](#架构概览)
- [仓库结构](#仓库结构)
- [快速开始](#快速开始)
- [CLI 工作流](#cli-工作流)
- [服务端](#服务端)
- [部署](#部署)
- [开发与验证](#开发与验证)
- [文档索引](#文档索引)
- [协作规则](#协作规则)
- [安全与许可](#安全与许可)

## 产品定位

HyperTide 不是 Git 的替代品，也不是为源代码的分布式开发而设计。它的重点是大型二进制资产，例如游戏资源、美术素材、场景文件、构建产物或其他不适合 Git 式本地 DAG 的文件。

HyperTide 的设计假设是：团队需要一个服务端维护的版本事实源，需要可审计的操作记录，需要工作区级别的同步、检出、暂存和提交能力，也需要围绕高风险变更设置审批、锁和治理流程。

当前产品方向：

- 服务端维护版本真相、branch head 和 changeset 状态。
- CLI 面向资产工作区，支持登录、同步、检出、上传、暂存、保存进度、检查点恢复和提交。
- 提供 changeset gate、approve、promote、lock、trust checkpoint、audit、replay 和 retention inspection 等治理命令面。
- 支持锁、审批、回滚、运行时校验和高风险操作签名。
- 不提供 Git 式本地仓库、本地 DAG、离线 commit、merge 或 rebase。

## 核心能力

### 资产版本

HyperTide 以资产路径和 blob hash 为基础描述版本状态。CLI 会把本地文件上传为 blob，再把资产路径到 blob 的变化写入 stage，最后由服务端生成正式 changeset。

服务端会在接受提交前校验 base changeset、锁冲突、权限和运行时规则。这样可以让资产历史保持中心化、可审计，并在多个客户端之间保持一致。

### 中心化分支

分支状态由服务端维护。客户端可以创建、列出、切换和同步分支，但不会在本地生成独立的分布式历史。

每次正式提交都以服务端认可的 base changeset 为基础。如果分支已经前进，服务端可以拒绝过期提交，而不是让客户端制造分叉的本地历史。

### 工作区与缓存

CLI 在当前目录下使用 `.hypertide/` 保存本地状态，包括登录 profile、stage、workspace metadata、session state 和对象缓存。

`checkout` 会把服务端快照 materialize 到当前工作目录。`sync` 只更新元数据和基线，不写入资产文件。本地对象缓存可以避免重复下载已经存在的 blob。

### 治理与审计

HyperTide 的治理面覆盖 changeset gate、approve、promote、asset lock、trust checkpoint、witness attestation、audit verification、replay verification 和 retention policy 查询。

这些命令用于把高风险版本操作从“单个客户端动作”提升为“可验证、可追踪、可回放”的服务端流程。

## 架构概览

HyperTide 当前由两个主要 Rust 包组成：`hypertide-server` 和 `hypertide-cli`。

服务端提供 HTTP API、鉴权、存储、锁、changeset、branch、audit 和 trust 能力。CLI 负责把本地 workspace 操作转换为 API 调用，并维护 `.hypertide/` 本地状态。后续 UI 工作区位于 `hypertide-ui/`，用于承载桌面或 Web 体验。

```text
workspace
├─ crates/server     -> hypertide-server, binary: hypertide
├─ crates/cli        -> hypertide-cli, binary: ht
├─ deploy            -> server and CLI deployment assets
├─ docs              -> specs, plans, validation notes, CLI docs
├─ migrations        -> SQL migrations used by the server crate
├─ hypertide-ui      -> desktop/web UI workspace
└─ skills            -> agent-facing operational workflows
```

维护中的 Rust 代码位于 `crates/`。根目录的 `src/` 是历史快照，不应作为当前 server 或 CLI 的维护入口。

## 仓库结构

| 路径 | 说明 |
| --- | --- |
| `crates/server` | 服务端包，binary 名称为 `hypertide`。 |
| `crates/cli` | CLI 包，binary 名称为 `ht`。 |
| `docs/cli/README.md` | CLI 使用说明和命令示例。 |
| `docs/` | 设计、计划、规格和验证记录。 |
| `deploy/server` | 服务端容器部署入口。 |
| `deploy/cli` | CLI 打包脚本和内部发布入口。 |
| `deploy/ubuntu-server-windows-cli.md` | Ubuntu server + Windows CLI walkthrough。 |
| `migrations/` | 服务端 SQL migration。 |
| `hypertide-ui/` | 后续桌面/Web UI 工作区。 |
| `skills/` | 面向 agent 的 HyperTide 操作技能。 |

生成文件、本地状态和运行时产物不应提交。当前会忽略 `target/`、`.tmp/`、`tmp/`、`storage/`、`.hypertide/`、`deploy/keys/`、`deploy/server/keys/` 和 `deploy/cli/dist/` 等目录。

## 快速开始

### 1. 环境要求

推荐使用 Rust workspace 直接运行 server 和 CLI。当前 workspace 使用 Rust 2021 edition，最低 Rust 版本声明为 `1.85.1`。

```powershell
rustc --version
cargo --version
```

如果需要运行容器化部署，请准备 Docker 和 Docker Compose。服务端持久化、数据库、密钥和 smoke test 细节见 `deploy/server/README.md`。

### 2. 运行测试

完整 workspace 验证：

```powershell
cargo test --workspace
```

只验证 CLI：

```powershell
cargo test -p hypertide-cli
```

### 3. 查看 CLI 帮助

```powershell
cargo run -p hypertide-cli --bin ht -- --help
```

CLI 的维护实现位于 `crates/cli/src/main.rs`，workspace binary 名称是 `ht`。旧的根目录源码不作为当前 CLI 发布入口。

### 4. 登录本地或测试服务

```powershell
cargo run -p hypertide-cli --bin ht -- login `
  --server http://localhost:3000 `
  --token dev-master-key `
  --repo demo-repo `
  --branch main
```

如果服务端配置为直接接受 API key，可以使用：

```powershell
cargo run -p hypertide-cli --bin ht -- login `
  --server http://localhost:3000 `
  --token dev-master-key `
  --api-key-direct `
  --repo demo-repo `
  --branch main
```

登录会写入当前目录下的 `.hypertide/profile.json`。非 `--api-key-direct` 模式下，CLI 会交换并刷新 JWT token；刷新失败时需要重新登录。

## CLI 工作流

### 支持的命令

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

### 常用资产流程

1. `login` 保存服务地址、凭据、默认 repo 和默认 branch。
2. `sync` 同步 branch metadata 和 base changeset，不写入资产文件。
3. `checkout` 从服务端 materialize 资产到当前工作目录。
4. `add` 上传本地文件并写入 `.hypertide/stage.json`。
5. `status` 和 `diff` 查看本地、stage、base 和 lock 状态。
6. `submit` 创建正式 changeset，并清空已提交 stage。

示例：

```powershell
cargo run -p hypertide-cli --bin ht -- sync --repo demo-repo --branch main
cargo run -p hypertide-cli --bin ht -- checkout --repo demo-repo --branch main
cargo run -p hypertide-cli --bin ht -- add `
  --file .\Content\Props\tree.uasset `
  --path Content/Props/tree.uasset `
  --branch main
cargo run -p hypertide-cli --bin ht -- status
cargo run -p hypertide-cli --bin ht -- submit `
  --repo demo-repo `
  --branch main `
  --message "update tree prop"
```

### 进度保存与检查点

`save` 和 `checkpoint` 用于 agent 或长流程恢复层。它们可以保存当前 workspace 的资产状态，但不会像 `submit` 一样推进正式 branch head。

```powershell
cargo run -p hypertide-cli --bin ht -- save --repo demo-repo --branch main --message "agent pass 1"
cargo run -p hypertide-cli --bin ht -- checkpoint create --repo demo-repo --branch main --message "before rewrite"
cargo run -p hypertide-cli --bin ht -- checkpoint list
cargo run -p hypertide-cli --bin ht -- checkpoint restore --id <checkpoint-id>
cargo run -p hypertide-cli --bin ht -- checkpoint branch --id <checkpoint-id> --name try/alt-plan
```

### 审批、锁和信任操作

正式版本晋升可以通过 `changeset gate`、`changeset approve` 和 `changeset promote` 进行检查与推进。资产编辑锁通过 `lock acquire`、`lock renew`、`lock release` 和 `lock list` 管理。信任与审计操作通过 `trust checkpoint`、`trust witness`、`trust audit`、`trust replay` 和 `trust retention` 进入。

```powershell
cargo run -p hypertide-cli --bin ht -- changeset gate --repo demo-repo --id <changeset-id>
cargo run -p hypertide-cli --bin ht -- changeset approve --repo demo-repo --id <changeset-id>
cargo run -p hypertide-cli --bin ht -- changeset promote --repo demo-repo --id <changeset-id>

cargo run -p hypertide-cli --bin ht -- lock acquire --path Content/Props/tree.uasset
cargo run -p hypertide-cli --bin ht -- lock list
cargo run -p hypertide-cli --bin ht -- lock release --path Content/Props/tree.uasset

cargo run -p hypertide-cli --bin ht -- trust audit verify
cargo run -p hypertide-cli --bin ht -- trust replay readiness
```

更完整的 CLI 使用说明见 [docs/cli/README.md](docs/cli/README.md)。

## 服务端

服务端包位于 `crates/server`，binary 名称为 `hypertide`。它负责 API 路由、鉴权、blob/manifest 存储、branch 和 changeset 状态、锁、审计、trust checkpoint、witness、replay 和 retention 相关能力。

服务端开发时请优先从 workspace 运行和测试，避免使用根目录历史 `src/` 作为入口。数据库 migration 位于 `migrations/`，部署环境变量示例位于 `deploy/server/.env.example`。

## 部署

部署入口已经按交付物拆分：

- [deploy/server](deploy/server/README.md): 服务端容器部署。
- [deploy/cli](deploy/cli/README.md): `ht` CLI 打包和内部发布。
- [deploy/ubuntu-server-windows-cli.md](deploy/ubuntu-server-windows-cli.md): Ubuntu server + Windows CLI walkthrough。

服务端容器示例：

```powershell
docker compose -f deploy/server/docker-compose.yml --env-file deploy/server/.env.example up -d --build
powershell -ExecutionPolicy Bypass -File .\deploy\server\smoke.ps1
```

CLI 打包示例：

```powershell
powershell -ExecutionPolicy Bypass -File .\deploy\cli\package.ps1
```

```bash
bash ./deploy/cli/package.sh
```

仓库仍保留早期顶层 `deploy/` 资产用于兼容，但新部署应优先使用 `deploy/server/` 和 `deploy/cli/`。

## 开发与验证

### 基线验证

```powershell
cargo test --workspace
```

当只修改 CLI 行为时，可以先跑：

```powershell
cargo test -p hypertide-cli
```

当修改部署、容器、启动脚本或运行时配置时，还应运行对应 smoke test：

```powershell
powershell -ExecutionPolicy Bypass -File .\deploy\server\smoke.ps1
```

### 开发约定

- 保持改动聚焦、可审阅，并在行为变化时更新文档。
- 不提交运行时产物、本地缓存、密钥、数据库文件或 `.hypertide/` 工作区状态。
- 优先维护 `crates/` 下的 Rust workspace。
- 保持产品定位：中心化、资产导向、服务端治理，而不是分布式 Git-style 工作流。

## 文档索引

- [README.md](README.md): 英文项目总览。
- [docs/cli/README.md](docs/cli/README.md): CLI 使用说明、命令示例和常见流程。
- [deploy/README.md](deploy/README.md): 部署入口总览。
- [deploy/server/README.md](deploy/server/README.md): 服务端容器部署。
- [deploy/cli/README.md](deploy/cli/README.md): CLI 打包。
- [deploy/ubuntu-server-windows-cli.md](deploy/ubuntu-server-windows-cli.md): Ubuntu server 与 Windows CLI walkthrough。
- [CONTRIBUTING.md](CONTRIBUTING.md): 当前协作模型。
- [SECURITY.md](SECURITY.md): 安全问题报告路径。

## 协作规则

当前仓库由维护者直接协调。默认流程是从 `main` 创建分支，做聚焦改动，补充对应文档和验证，再通过 GitHub pull request 合入 `main`。

在直接推主线前，需要确认本地 `main` 与 `origin/main` 已安全整合，并避免 force push。

最小提交预期：

- 代码或文档改动有清楚范围。
- 行为变化附带文档更新。
- 提交前运行相关测试或说明未运行原因。
- 不提交生成文件、密钥、本地运行状态或大型临时文件。

## 安全与许可

安全敏感问题应按 [SECURITY.md](SECURITY.md) 中的私有报告路径处理，不应公开披露。

本项目基于 [MIT 许可证](LICENSE) 开源。
