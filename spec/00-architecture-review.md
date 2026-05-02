# HyperTide 架构评审与改进建议

> 基于 2026-05-02 深度评审
> 参与: GPT-4o, Claude (Hermes), aoruLola

---

## 一、产品定位

### 现状
HyperTide 当前被描述为 "asset version control system"，但代码和 CLI 已经超出了这个范围。

### 建议定位
> **"Replayable Development History — 可回放的开发历史系统"**

这不是：
- ❌ Git 替代品
- ❌ 分布式 SCM
- ❌ 代码审阅系统
- ❌ AI VCS（AI 只是用户之一）

这是：
- ✅ 新一代生产历史系统
- ✅ 工作空间状态生命周期管理
- ✅ Human + Agent 协作的可追溯执行记录
- ✅ Checkpoint 驱动的协作模型

### 叙事调整
| 当前 | 建议 |
|------|------|
| AI-native VCS | Replayable development history |
| AI 是产品定义 | AI 是用户类型之一 |
| 强调 AI 能力 | 强调 replay / lineage / checkpoint |

---

## 二、许可与商业模式

### 当前状态
未明确 License，代码全公开但无协议。

### 建议：MIT + Cloud SaaS

```
开源层 (MIT)
├── hypertide-core — 核心对象模型
├── hypertide-cli — CLI 工具
├── hypertide-server — 本地/自托管 Server
├── spec/ — 开放协议规范
└── sdk/ — 客户端 SDK

商业层 (SaaS)
├── 分布式存储 + dedup
├── Agent 协作历史可视化
├── 企业 SSO / RBAC
├── 审计追溯 / 合规导出
└── 托管 backup / HA
```

**不推荐 AGPL 的原因：**
- 游戏公司/引擎团队法务通常回避 AGPL
- VCS 类基础设施需要最大程度的生态开放
- 真正的护城河是数据模型和生态，不是 License

---

## 三、核心对象模型重构

### 当前问题
多个概念并存，用户认知负担高：

```
save / checkpoint / submit / changeset / witness / replay readiness / promotion / lineage
```

### 建议：统一为 Checkpoint

```rust
enum CheckpointState {
    Temporary,   // 原 save / draft
    Reviewable,  // 原 changeset / submit
    Promoted,    // 原 promoted changeset
}
```

### 术语对照表

| 当前术语 | 建议术语 | 理由 |
|---------|---------|------|
| checkpoint | ✅ 保留 | 核心对象，不变 |
| save | `ht checkpoint --temporary` | save 是创建 Temporary checkpoint |
| changeset | 合并入 Checkpoint | 本质是 Reviewable 状态的 Checkpoint |
| witness | attestation | witness 过于抽象 |
| witness receipt | attestation receipt | — |
| witness topology | attestation network | — |
| replay readiness | checkpoint health | readiness 含义模糊 |
| promotion | publish | promotion 太企业内部 |
| lineage | history | 开发者更熟悉 history |
| audit chain | event log | chain 暗示区块链，不准确 |
| lock | ✅ 保留 | 合适 |
| trust audit | audit | trust 前缀冗余 |
| trust replay | replay verify | — |
| compliance audit | ✅ 保留但降级 | 早期不需要 |

---

## 四、CLI 命令重构

### 当前状态
CLI 已经暴露完整的 production workflow model，但部分命令名有误导。

### 建议修改

| 当前 | 建议 | 理由 |
|------|------|------|
| `ht checkout` | `ht materialize` (或 `ht restore`) | checkout 让 Git 用户带入错误认知；本质是 workspace materialization |
| `ht save` | `ht checkpoint --temp` | save 太泛，统一到 checkpoint 概念下 |
| `ht submit` | `ht checkpoint --submit` | submit 是 checkpoint 状态转换 |
| `ht checkpoint branch` | `ht checkpoint --fork` | 更直观 |
| `ht sync` | ✅ 保留 | 设计合理，区分 sync 和 materialize |
| `ht status` | ✅ 保留 | 可以做得比 Git 更强 |
| `ht diff` | ✅ 保留但扩展 | 未来做 semantic diff |
| `ht rollback` | ✅ 保留但重新定义 | 不是反向 submit，是 workspace state recovery |

### CLI 命令分类（建议）

```
一、Workspace 生命周期
  ht login          接入 production authority
  ht sync           同步元数据（不 materialize）
  ht materialize    将指定 checkpoint 写入工作区
  ht status         查看工作区状态 vs server

二、Checkpoint 操作（统一入口）
  ht checkpoint           列出/管理 checkpoints
  ht checkpoint --temp    创建临时 checkpoint (原 save)
  ht checkpoint --submit  提交审阅 (原 submit)
  ht checkpoint --promote 发布 (原 changeset approve+promote)
  ht checkpoint --fork    从历史 checkpoint 分支
  ht checkpoint --log     查看 timeline

三、资产操作
  ht add            注册/上传资产
  ht remove         移除资产
  ht diff           semantic diff
  ht blob           底层 blob 操作

四、协作
  ht lock           文件锁定
  ht unlock         释放锁
  ht attest         验证 checkpoint (原 trust witness)

五、恢复与重建
  ht rollback       恢复工作区到历史状态
  ht replay         重建执行过程
```

---

## 五、Replay 重新定义

### 当前定义
暗示 deterministic re-execution（完全可重复执行）。

### 建议定义
> **Replay = Execution Reconstruction（执行过程重建）**

- 不是 bit-perfect reproduction
- 是 workflow reconstruction
- 记录实际发生的事件，重放时重建状态
- 不承诺 AI 模型/外部工具链的确定性重放
- 但对**审计和追溯**已经足够

### 关键区分
```rust
// Replay 做的事情：
read events from log → apply state transitions → verify state root

// Replay 不做的事情：
re-run AI model → re-import assets → re-build project
```

---

## 六、功能优先级

### 当前问题
代码库中存在许多 enterprise 功能（witness topology、compliance audit、high_risk、retention policy），过早复杂化。

### 建议优先级

```
P0 — 现在做
├── Object Model Spec ✅ (已开始)
├── 仓库拆分 (server / CLI)
├── 统一 Checkpoint 概念
├── CLI 命令重构
├── 可视化时间线 (ht log --graph / Web UI)
└── README 重新定位

P1 — 短期
├── Workspace Snapshot 模型完善
├── Replay 可视化
├── Partial sync / lazy fetch
├── Semantic diff (asset 级别)
└── Spec 系列文档 (graph / replay / attestation)

P2 — 中期
├── Cloud SaaS 原型
├── Agent execution unit 定义
├── Plugin / Bridge SDK
└── 生态工具 (exporter, viewer)

P3 — 冻结（不做深入研究）
├── Enterprise RBAC / SSO
├── Compliance audit
├── High-risk nonce tracking
├── Retention policy
└── Multi-environment witness topology
```

---

## 七、代码架构建议

### 当前结构
```
HyperTide/
├── src/           ← 核心逻辑（monolith）
├── crates/cli/    ← CLI
├── crates/server/ ← Server（新拆分）
└── hypertide-ui/  ← Desktop UI
```

### 建议结构
```
hypertide/                  ← server repo (MIT)
├── crates/
│   ├── hypertide-core/     ← 对象模型、trait 定义（不依赖 server）
│   ├── hypertide-server/   ← HTTP server
│   └── hypertide-graph/    ← checkpoint ancestry / lineage
├── spec/                   ← 协议规范
├── migrations/             ← DB schema
└── docs/

hypertide-cli/              ← 独立 repo (MIT)
├── src/
└── Cargo.toml              ← 依赖 hypertide-core

hypertide-ui/               ← 独立 repo (MIT)
├── src/
└── package.json

hypertide-cloud/            ← 私有 repo (闭源)
├── cloud-orchestration/
├── enterprise-auth/
└── ...
```

---

## 八、命名规范

### 避免术语爆炸

| 建议保留 | 建议删除/合并 |
|----------|-------------|
| checkpoint | changeset（合并入 checkpoint） |
| event | save（合并入 checkpoint） |
| asset | witness topology（→ attestation network） |
| workspace | replay readiness（→ checkpoint health） |
| lock | promotion（→ publish） |
| attestation | high-risk（暂时冻结） |
| replay | retention policy（暂时冻结） |

### 关键原则
- 复用开发者已有认知的术语
- 一个概念一个词
- 新术语必须有明确的不可替代性

---

## 九、下一步执行计划

### 第 1 步：钉死 Spec（本周）
- [x] 起头 `spec/01-object-model.md`
- [ ] 评审 + 修改 Object Model
- [ ] 补充 `spec/02-replay-protocol.md`
- [ ] 补充 `spec/03-checkpoint-lifecycle.md`

### 第 2 步：改 License + README（本周）
- [ ] 添加 MIT License
- [ ] 重写 README，按新定位
- [ ] 明确 "NOT" 列表

### 第 3 步：代码层对齐 Spec（2 周）
- [ ] 重构 Checkpoint 为唯一核心对象
- [ ] CLI 命令改名
- [ ] 合并 changeset/save/checkpoint
- [ ] 冻结 P3 功能模块

### 第 4 步：仓库拆分（2 周）
- [ ] `git filter-repo` 拆分 CLI 到独立仓库
- [ ] 创建 hypertide-core crate
- [ ] 更新 CI

### 第 5 步：可视化时间线（3 周）
- [ ] `ht log --graph` 增强
- [ ] Web UI 原型

---

## 附录：代码评审结论

### 代码质量评分

| 维度 | 分数 | 说明 |
|------|------|------|
| 核心概念成熟度 | ⭐⭐⭐⭐⭐ | Witness/Replay/Checkpoint 已钉死 |
| 代码质量 | ⭐⭐⭐⭐ | 干净、类型安全、无坏味道 |
| 架构 | ⭐⭐⭐ | Monolith→crate 过渡期 |
| 测试 | ⭐⭐⭐ | 有 CI，但核心逻辑缺单元测试 |
| 文档 | ⭐⭐⭐⭐ | 中英双语，有 AGENTS.md |
| 完成度 | ⭐⭐⭐ | 核心逻辑完成，UI/Cloud 早期 |

### 最乐观的一点
> 最硬的东西已经写完了：Witness、Replay、Checkpoint、Execution Log、Lineage Graph。
> 剩下的是工程问题、文档问题和产品问题——不是研究问题。

---

*文档版本: v1 | 2026-05-02 | 作者: Hermes Agent + aoruLola*
