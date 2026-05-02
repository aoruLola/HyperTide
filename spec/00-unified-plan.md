# HyperTide 综合评审与统一执行方案

> 2026-05-02 · 最终版
>
> 评审参与方：
> - 🧠 **GPT (ChatGPT)** — 产品经理视角，17 页深度评估
> - 🧠 **GPT (MiMo)** — 架构 + 产品双视角评估
> - 🔧 **Hermes (Claude)** — 架构师，代码评审 + spec 输出
> - 💻 **Claude Code** — 工程师，代码级 critique
> - 👤 **aoruLola** — 产品决策者

---

## 第一部分：现状共识

### 1.1 产品定位（五方一致）

**"Replayable Development History — 可回放的开发历史系统"**

这不是：
- ❌ Git 替代品
- ❌ 分布式 SCM
- ❌ 仅面向 AI 的工具
- ❌ 普通 SaaS

这是：
- ✅ 大型二进制资产的中心化版本与协作系统
- ✅ 记录生成过程的 work-in-progress VCS
- ✅ Human + Agent 协作的可追溯执行记录
- ✅ Checkpoint 驱动的协作模型

> **MiMo 原文：** HyperTide 的定位非常明确——面向大文件二进制资产的集中式版本协作系统。它不试图替代 Git，而是填补了一个真实的市场空白。
>
> **ChatGPT 原文：** HyperTide 当前已经具备一个较完整的"中心化大二进制资产版本系统"内核。

### 1.2 代码成熟度（五方一致）

HyperTide 当前不是 MVP，也不是完整产品——它是**工程可用的基础设施内核**。

| 维度 | 评分 | ChatGPT | MiMo | Hermes | Claude Code |
|------|:----:|:-------:|:----:|:------:|:-----------:|
| 核心版本模型 | 8/10 | ✅ | ✅ | ✅ ✅ | ✅ |
| 安全治理 | 8/10 | ✅ | ✅ ✅ | ✅ | ⚠️ HMAC 需修 |
| 架构设计 | 7/10 | — | ✅ | ✅ | ⚠️ 存储抽象缺 |
| CLI 功能覆盖 | 7/10 | ✅ | ✅ | ✅ | — |
| 用户体验 | 5/10 | ✅ P0 风险 | ✅ | ⚠️ | — |
| 文档 | 7/10 | — | ✅ | ✅ | — |
| 运维准备度 | 5/10 | — | ⚠️ | — | — |

> **ChatGPT 原文：** 当前状态更接近"工程可用的基础设施内核"，还不是"资产团队可以低摩擦采用的协作产品"。
>
> **MiMo 原文：** 内核强、外壳薄。核心版本模型和安全治理已经是企业级水准，但存储抽象、CLI 体验、运维工具链还需要一轮集中打磨。

---

## 第二部分：六方意见汇总

### 2.1 🧠 GPT (ChatGPT) — 产品经理视角

**定位：** 17 页详细产品评估，从代码和用户双视角出发。

**核心发现：**

| 维度 | 评价 |
|------|------|
| Server 版本模型 | 方向正确，状态机已成型。draft→approve→promote 对企业场景有价值 |
| 回滚能力 | 实现方式正确（回滚是一次可追踪变更），但缺 dry-run |
| 存储模型 | 内容寻址 + chunk 上传适合大资产，但缺 streaming 和对象存储后端 |
| 锁模型 | 基本可用，但 key 只有 file_path，缺少 repo/branch 维度 |
| 鉴权 | 比 MVP 完整（JWT + API key + refresh rotation），但权限粒度不足 |
| CLI 主链路 | 完整跑通，从 login 到 rollback，但心智偏工程 |

**P0 风险（最重要）：**

1. **checkout 会覆盖本地未提交修改** — 没有预检，没有确认
2. **sync / branch switch 会清空 stage** — 没有提示
3. **add --file 的默认 asset path 可能误导** — 路径规范化缺乏文档
4. **status 信息表达不够** — 单值状态掩盖了组合状态

**差异化亮点：**
- Checkpoint 系统（save/restore/branch from checkpoint）是真正的差异化
- Agent session/checkpoint 适合 AI 长任务
- 但这些能力被命名冲突拖累（save vs checkpoint vs trust checkpoint）

### 2.2 🧠 GPT (MiMo) — 架构 + 产品双视角

**定位：** 侧重架构评审，但也覆盖产品定位、竞品分析。

**架构优点：**
- ✅ BLAKE3 内容寻址存储，两级目录防爆
- ✅ changeset 四阶段门禁流（draft→approve→promote→visible）
- ✅ 生产环境强制高风险签名
- ✅ SQLx 编译时校验，advisory lock 保审计链并发安全
- ✅ 13 个 migration，结构清晰

**架构问题（P0）：**

| 问题 | 详情 | 建议 |
|------|------|------|
| 存储层无抽象 | `StorageManager` 直接操作本地文件系统 | 定义 `StorageBackend` trait |
| AppState 太多 Option | 7 个 `Option` 字段 | builder pattern 或明确启动路径 |
| 内存锁持久化问题 | DashMap + PG 双写，重启可能丢锁 | 事务性锁持久化 |
| CLI main.rs 过大 | 3500+ 行 | 按命令分模块 |
| 错误信息可读性差 | 透传 HTTP 响应体 | CLI 层错误映射 |
| 缺速率限制 | 无 middleware | tower::limit |
| 缺指标/可观测性 | 无 metrics，无 trace ID | Prometheus + tracing |

**P0 优先级建议：**
1. 存储后端抽象（`StorageBackend` trait）
2. CLI 模块化拆分
3. 错误信息用户友好化
4. 危险操作确认（`--yes` / 交互确认）

### 2.3 🔧 Hermes (Claude) — 架构师

**贡献：** 代码评审、规格输出、执行计划。

**关键结论：**

1. **最硬的东西已经写完了** — Witness、Replay、Checkpoint、Execution Log、Lineage Graph
2. **Checkpoint 应是唯一核心对象** — 但需要区分 WorkspaceCheckpoint（用户级）和 TrustAttestation（系统级）
3. **License 策略** — MIT 社区版 + Enterprise 私有 crate + Cloud SaaS
4. **术语收缩** — witness→attestation, lineage→history, audit chain→event log
5. **Killer feature 是可视化时间线** — `ht log --graph`

### 2.4 💻 Claude Code — 工程师

**核心批评（最尖锐）：**

| 批评 | 详情 |
|------|------|
| Checkpoint 统一是过度修正 | 信任 checkpoint 和用户 checkpoint 是两个不同生命周期，强行合并会出问题 |
| Event attribution 是空的 | 代码里有 `actor_id` 字段，但 replay 处理事件时不追踪 agent |
| Replay 性能 O(n) | 全表扫描所有 events，不是 checkpoint 级增量 |
| Witness 签名是 security theater | `blake3(secret \|\| data)` 不是真正 HMAC |
| 迁移路径没写 | 现有 PostgreSQL schema 要改，但没有 SQL migration 方案 |
| 没有讨论 src/ vs crates/ | 核心逻辑散落在两个目录 |

**他给的优先级：**
- P0: 删 dead weight → 加 event attribution → 增量 replay
- P1: 统一概念但不统一类型 → 写 migration 路径 → replay e2e 测试
- P2: CLI 改名 → 拆仓库 → 可视化

### 2.5 👤 aoruLola — 产品决策者

**核心观点：**

1. **Checkpoint 是核心，但用户 checkpoint 和 trust checkpoint 应分离**
2. **企业功能不删、不冻、挪走** — 迁入 `hypertide-enterprise` 私有 crate
3. **CLI 不缺功能，缺安全保护** — checkout/sync/stage 的保护比加新命令更重要
4. **可视化时间线是 killer feature** — 让用户第一次看就明白"这不是 Git"
5. **Replay 不承诺 deterministic** — 定义为 execution reconstruction
6. **AI 从主叙事降级** — 但 event attribution 必须先做实

---

## 第三部分：分歧与裁决

| 分歧 | GPT(C) | GPT(M) | Hermes | CC | aoruLola | **裁决** |
|------|:------:|:------:|:------:|:--:|:--------:|:---------:|
| License 现在定？ | 可定 | 可定 | MIT 现在 | 等 v1 | ✅ MIT | **现在定 MIT** |
| 统一 Checkpoint？ | ⚠️ 需分清 | — | 统一 | 2 个类型 | 2 个类型 | **2 个类型** |
| 删企业代码？ | 冻结 | — | 冻结 | 删 | 挪走 | **挪进私有 crate** |
| CLI 改名时机？ | 不急 | — | P2 | P2 | 等稳定 | **P2 再说** |
| 存储抽象优先级 | — | P0 | — | — | — | **P1（不是 P0）** |
| 工作区安全 vs 架构 | **P0** | P0 架构 | — | — | ✅ P0 | **P0 安全 + P1 架构** |
| Event attribution | — | — | ✅ | ✅ | ✅ ✅ | **P0（基础数据）** |

---

## 第四部分：统一执行方案

### Phase 0 — 立即启动（当前）

| # | 任务 | 来源 | 预期产出 |
|---|------|------|---------|
| 0.1 | 写 MIT License 文件 | Hermes | `LICENSE` 文件 |
| 0.2 | 创建工作区安全检查 — checkout 预检 | ChatGPT P0 | checkout 时检测本地修改，默认拒绝覆盖 |
| 0.3 | 创建 `hypertide-enterprise` 私有仓库 | aoruLola | GitHub 私有 repo |
| 0.4 | 将 attestation/compliance 迁入私有 crate | aoruLola | 社区版编译不包含 |

### Phase 1 — 架构清理（1-2 周）

| # | 任务 | 来源 | 说明 |
|---|------|------|------|
| 1.1 | 加 event attribution | Claude Code + 所有人 | 所有事件绑定 `actor_id`、`workflow_id`、`tool_id` |
| 2.1 | 定义 trait interface | MiMo + Hermes | `StorageBackend`、`AttestationProvider`、`CheckpointStore` |
| 3.1 | CLI 模块化拆分 | MiMo | main.rs 3500+ 行 → `cmd/*.rs` |
| 4.1 | sync/stage 安全保护 | ChatGPT P0 | sync 和 branch switch 不清空 stage |
| 5.1 | 写 Object Model 落地代码 | Hermes + aoruLola | `WorkspaceCheckpoint` struct，单元测试 |
| 6.1 | 写 SQL migration 路径 | Claude Code | 从当前 schema 到目标 schema |

### Phase 2 — 核心对齐（2-3 周）

| # | 任务 | 来源 | 说明 |
|---|------|------|------|
| 2.1 | 统一 save / changeset / checkpoint | 所有人 | 代码层对齐 spec |
| 2.2 | 改 replay 为增量模式 | Claude Code | 按 checkpoint 差量，不全表扫描 |
| 2.3 | 修复 attestation 签名 | Claude Code | `blake3(secret || data)` → 真正 HMAC |
| 2.4 | Replay end-to-end 测试 | Claude Code + ChatGPT | 从 create events → replay → verify |
| 2.5 | 锁模型升级 repo-aware | ChatGPT | `locks.file_path` → `(repo_id, scope, asset_id)` |
| 2.6 | 错误信息用户友好化 | MiMo | HTTP 错误码 → CLI 友好提示 |
| 2.7 | 危险操作加确认 | ChatGPT + MiMo | `--yes` / 交互确认 |

### Phase 3 — 对外发布（3-4 周）

| # | 任务 | 来源 | 说明 |
|---|------|------|------|
| 3.1 | MIT License + README 重写 | 所有人 | 新定位 + NOT 列表 |
| 3.2 | spec/ 目录公开 | Hermes | 开放协议规范 |
| 3.3 | `ht log --graph` 增强 | aoruLola | 可视化 checkpoint 谱系 |
| 3.4 | `ht doctor` | ChatGPT | 一键检查登录/server/repo/stage |
| 3.5 | `ht stage list/clear` | ChatGPT | 显式 stage 管理 |
| 3.6 | 存储后端抽象（LocalFS + S3） | MiMo | `StorageBackend` trait + 2 个实现 |
| 3.7 | JSON 输出模式 | MiMo | 所有命令支持 `--json` |
| 3.8 | Shell 补全 | MiMo | `ht completions bash/zsh/fish` |

### Phase 4 — 产品化（Q3）

| # | 任务 | 来源 | 说明 |
|---|------|------|------|
| 4.1 | Web UI 可视化时间线 | aoruLola + ChatGPT | Killer feature |
| 4.2 | Cloud SaaS 原型 | aoruLola | 托管版 |
| 4.3 | 增量快照 | MiMo P3 | 避免每次全量计算 |
| 4.4 | Git 桥接 | MiMo P3 | Git 仓库作为 HyperTide 分支 |
| 4.5 | 速率限制 + 指标 | MiMo | tower::limit + Prometheus |
| 4.6 | CLI 改名（如需） | 所有人 | 等 API 稳定后再做 |

---

## 第五部分：仓库结构（最终目标）

### 公开（MIT）
```
github.com/aoruLola/HyperTide
├── crates/
│   ├── hypertide-core/     ← 对象模型、trait、不依赖 server
│   ├── hypertide-server/   ← HTTP server
│   └── hypertide-graph/    ← checkpoint ancestry / lineage
├── spec/                   ← 协议规范
├── migrations/             ← DB schema
└── docs/

github.com/aoruLola/hypertide-cli (MIT)
├── src/ (cmd/*.rs 拆分)
└── Cargo.toml              ← 依赖 hypertide-core

github.com/aoruLola/hypertide-ui (MIT)
├── src/ (Tauri + React)
└── package.json
```

### 私有（商业授权）
```
github.com/aoruLola/hypertide-enterprise (🔒 私有)
├── attestation/            ← witness / replay verify
├── enterprise-auth/        ← SSO / RBAC
└── compliance/             ← audit export
```

---

## 第六部分：风险登记

| 风险 | 概率 | 影响 | 缓解措施 |
|------|:----:|:----:|---------|
| Checkpoint 统一导致数据模型膨胀 | 中 | 高 | 区分 WorkspaceCheckpoint + TrustAttestation |
| 存储抽象过早引入复杂度 | 低 | 中 | P1 再做，先用本地 FS 验证 |
| 工作区安全改动破坏现有工作流 | 中 | 高 | `--force` 保留逃生口 |
| CLI 模块化拆分破坏命令注册 | 中 | 中 | 分步做，每步保留测试 |
| License 从 MIT 改不动 | 低 | 低 | README 注明 "may adjust before v1" |
| Replay 性能问题导致不可用 | 中 | 高 | Phase 2 优先做增量 replay |

---

## 第七部分：一句话时间线

```
Phase 0 (现在):  License + enterprise repo + checkout 安全
Phase 1 (1-2 周): event attribution + trait 接口 + CLI 模块化 + stage 保护
Phase 2 (2-3 周): checkpoint 统一 + 增量 replay + 锁模型升级 + 错误友好化
Phase 3 (3-4 周): 开源发布 + ht log --graph + 存储抽象 + JSON 输出
Phase 4 (Q3):     Web UI 可视化 + Cloud SaaS
```

---

## 第八部分：你只需要记住的三件事

1. **产品是 Replayable Development History**，不是 AI VCS，不是 Git 替代品
2. **许可证是 MIT 社区版 + Enterprise 私有 crate + Cloud SaaS**
3. **第一步是工作区安全（不丢文件）和 event attribution（谁做了什么）**

---

*文档版本: final | 2026-05-02 | 五方综合评审*

*参与评审：GPT (ChatGPT 产品视角) · GPT (MiMo 架构视角) · Hermes (Claude) · Claude Code · aoruLola*
