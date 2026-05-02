# HyperTide 四方会审 — 结论与执行方案

> 2026-05-02 · v3（整合 aoruLola 第二轮反馈）
> GPT · Hermes (Claude) · Claude Code · aoruLola

---

## 一、各方角色

| 角色 | 贡献 | 典型发言 |
|------|------|---------|
| 🧠 **GPT** | 战略、定位、生态 | "成为基础设施比卖 License 重要" |
| 🔧 **Hermes** | 架构、spec、执行计划 | "最硬的东西已经写完了" |
| 💻 **Claude Code** | 工程评审、代码级 critique | "Dead code is liability, not asset" |
| 👤 **aoruLola** | 产品直觉、CLI 分析、方向决策 | "Workspace state lifecycle 才是核心" |

---

## 二、共同确认的结论

### ✅ 产品定位
**"Replayable Development History — 可回放的开发历史系统"**

- 不是 "AI VCS"（AI 是用户类型之一）
- 不是 "Git replacement"
- 核心价值：记录生成过程，而非仅记录最终结果

### ✅ 核心对象

**Checkpoint 的定义（spec 第一章第一行）：**

> **A checkpoint is a recoverable workspace state.**
>
> Not a file diff.
> Not a commit.
> Not an upload batch.
>
> It captures what the workspace looked like at a point in time,
> including file tree, toolchain context, and pending outputs.

两个类型，一个关系：

| 对象 | 职责 | 创建者 | CLI 名 |
|------|------|--------|--------|
| `WorkspaceCheckpoint` | 用户级工作区快照 | 用户/AI | `checkpoint` |
| `TrustAttestation` | 密码学完整性证明 | 系统 | `attest` |

**重要：** CLI/UI 层面只用 `checkpoint`，不在命令里暴露 `WorkspaceCheckpoint` 完整名字。

**关系：** 每个 WorkspaceCheckpoint **可以有**一个可选的 Attestation，但不是同一类型的不同状态。

### ✅ 许可模型
**社区版 MIT + 企业版商业授权**

- 社区版：`hypertide-core` + `hypertide-cli` + `hypertide-server`（全部 MIT）
- 企业版：`hypertide-enterprise`（私有 crate，商业授权）
- 通过 trait interface 对接，不是 feature flag（防编译绕过）
- License **现在定** MIT，README 注明 "may adjust before v1"

### ✅ 商业模式
**Cloud SaaS 为主**

- 托管服务（协作、存储、可视化）— 自然付费意愿
- 企业 License 为辅（大公司内部部署）

### ✅ 术语精简

| 旧词 | 新词 | 理由 |
|------|------|------|
| changeset | → WorkspaceCheckpoint (Reviewable) | 统一到 checkpoint |
| save | → checkpoint --temp | 同上 |
| witness | → attestation | 更直观 |
| witness receipt | → attestation receipt | — |
| replay readiness | → checkpoint health | readiness 含义模糊 |
| promotion | → publish | 开发者更熟悉 |
| lineage | → history | — |
| audit chain | → event log | 去区块链暗示 |

### ✅ 企业功能策略
**不删、不冻、挪走**

瘦身后的 enterprise crate（只放确定性高的功能）：

```
hypertide-enterprise/
├── attestation/         ← witness / replay verify（核心）
├── enterprise-auth/     ← SSO / RBAC
└── compliance/          ← audit export

暂不放入 enterprise（还不够成熟）:
├── high-risk/           ← 以后再说
└── retention/           ← 以后再说
```

---

## 三、分歧与裁决

| 分歧 | 各方立场 | 最终决定 |
|------|---------|---------|
| License 现在定还是等 v1 | GPT+Hermes: 现在定 MIT | Claude Code: 等 v1 | ✅ **现在定 MIT，注明可能调整** |
| 一个 Checkpoint 还是两个 | GPT+Hermes: 一个 | Claude Code: 两个 | ✅ **两个：WorkspaceCheckpoint + TrustAttestation** |
| 删企业代码还是留着 | Claude Code: 删 | Hermes+aoruLola: 留 | ✅ **迁入 enterprise 私有 crate，不删** |
| CLI 改名时机 | GPT+Hermes: 现在 | Claude Code: P2 | ✅ **概念先统一，CLI 改名等 API 稳定后** |
| License 文件 | GPT+Hermes: 现在加 | Claude Code: 等 v1 | ✅ **现在加 MIT，注明可能调整** |

---

## 四、Replay Guarantee 分级

Replay **不承诺确定性重放**。以下是保证级别：

| 维度 | 保证级别 | 说明 |
|------|---------|------|
| 文件状态 | ✅ **Strong** | 文件内容和目录结构完全恢复 |
| 事件顺序 | ✅ **Strong** | 操作序列、时间顺序完全恢复 |
| AI 输出一致性 | ⚠️ **Weak** | 记录输出内容但不保证重新生成相同结果 |
| 外部工具链 | ❌ **None** | 不保证引擎/插件/编译器版本一致性 |

**Replay 的本质是 execution reconstruction，不是 bit-perfect reproduction。**

---

## 五、Killer Workflow（最关键的产品决策）

当前最缺的是一个"第一次用就 wow"的体验。

**最高优先级：Replay Timeline Visualization**

```
checkpoint abc1234
├── AI texture generation ────────→ texture_v2.tga
├── Imported to UE ───────────────→ /Game/Characters/...
├── Material adjustment ──────────→ roughness 0.2→0.5
├── [ROLLBACK] ←──────────────────┘
└── Regenerated variant ──────────→ texture_v3.tga ✅ promoted
```

这个视图一出，用户**瞬间理解"这不是 Git"**。

**实现路径：**
1. `ht log --graph` 命令行时间线（Phase 3）
2. Web UI 交互式时间线（Phase 4）

**比以下都优先：**
- 分布式存储
- 企业 auth
- Plugin system

---

## 六、修正后的执行计划

### Phase 0 — Spec 先行（本周）

| 任务 | 负责人 | 产出 |
|------|--------|------|
| Object Model v2 — WorkspaceCheckpoint + TrustAttestation 分离 | Hermes | spec/01-object-model.md v2 |
| Replay Protocol Spec（含 guarantee 分级表） | Hermes | spec/02-replay-protocol.md |
| Checkpoint Lifecycle — 状态转换 + fork/merge | Hermes | spec/03-checkpoint-lifecycle.md |
| 评审 + 定稿 | aoruLola | 三方绿 |

### Phase 1 — 架构清理（1-2 周）

| 任务 | 说明 |
|------|------|
| 创建 `hypertide-enterprise` 私有仓库 | GitHub 私有 repo，只含 attestation + enterprise-auth + compliance |
| 将 attestation/compliance 相关代码迁入 | 不删历史，只是搬走 |
| 定义 trait interface（AttestationProvider 等） | 在 hypertide-core 里 |
| 加 event attribution（agent_id 绑定到每个事件） | 当前 execution log 缺失，Ph0 确定的最高优先级 |
| 改 replay 为增量模式 | 按 checkpoint 差量，不全表扫描 |
| 写 SQL migration 路径 | 从当前 schema 到目标 schema |

### Phase 2 — 代码对齐（2-3 周）

| 任务 | 说明 |
|------|------|
| 合并 changeset / save 到 WorkspaceCheckpoint | 代码层对齐 spec |
| 统一 CLI 内部概念 | 命令行先不改名，但内部数据模型统一 |
| 修复 attestation 签名（用真正 HMAC） | 当前是 security theater |
| Replay end-to-end 测试 | 从 create events → replay → verify |
| 整理 `src/` vs `crates/` | 解决当前代码分裂 |

### Phase 3 — 对外发布（3-4 周）

| 任务 | 说明 |
|------|------|
| MIT License 文件 | ✅ |
| README 重写 — 新定位 + NOT 列表 | 明确不是 Git replacement |
| spec/ 目录对外公开 | 开放协议规范 |
| `ht log --graph` 时间线增强 | 可视化 checkpoint 谱系（**killer feature**） |
| CLI 改名（如果需要） | 等 API 稳定后再做 |

### Phase 4 — 产品化（Q3）

| 任务 | 说明 |
|------|------|
| Web UI 交互式时间线 | 真正的 killer feature |
| Cloud SaaS 原型 | 托管版 |
| Plugin / Bridge SDK | 生态建设 |
| 仓库拆分（CLI 独立 repo） | 有需要时再做 |

---

## 七、仓库结构（最终目标）

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
├── src/
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

## 八、产品时间线（一句话版）

```
Phase 0 (这周):   Spec 钉死 ✓
Phase 1 (2 周):   代码对齐 + enterprise 私有 crate
Phase 2 (3 周):   Checkpoint 统一 + Replay 可工作 + 测试
Phase 3 (1 月):   开源发布 + ht log --graph 时间线
Phase 4 (Q3):     Web UI 可视化 + Cloud SaaS
```

---

## 九、如果你只记住三件事

1. **产品定位**：Replayable Development History，不是 AI VCS
2. **许可策略**：MIT 社区版 + Enterprise 私有 crate + Cloud SaaS
3. **第一步**：先钉 Object Model Spec，再动代码

---

## 十、四方性格总结

| 角色 | 擅长 | 盲区 |
|------|------|------|
| 🧠 **GPT** | 看方向、看定位、看叙事 | 不碰代码实现细节 |
| 🔧 **Hermes** | 把战略翻译成文档、模型、计划 | 偶尔 oversimplify |
| 💻 **Claude Code** | 抓代码级坑、性能、迁移路径 | 对产品战略不太敏感 |
| 👤 **aoruLola** | 产品直觉、抽象收敛、决策 | — |

**互补关系：**
GPT 说往哪走 → Hermes 画地图 → Claude Code 检查路况 → aoruLola 拍板

---

*文档版本: v3 | 2026-05-02 | 四方会审结论*
