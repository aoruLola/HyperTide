# HyperTide 统一执行计划

> 日期：2026-05-02
> 基于：五方评审综合方案（00-unified-plan.md）
> 产品定位：Replayable Development History — 可回放的开发历史系统

---

## 总览时间线

```
Phase 0 (今天)     → License + checkout 安全 + enterprise 仓库
Phase 1 (第 1-2 周) → event attribution + trait 接口 + CLI 模块化 + stage 保护
Phase 2 (第 3-4 周) → checkpoint 对齐 + 增量 replay + 锁模型升级 + 错误友好化
Phase 3 (第 5-6 周) → 开源发布 + ht log --graph + 存储抽象 + JSON 输出
Phase 4 (Q3)       → Web UI 可视化 + Cloud SaaS
```

---

## Phase 0 — 立即启动（今天）

### 0.1 MIT License

**目标**：在仓库根目录添加 MIT 许可证文件，明确开源立场。

**实施步骤**：
1. 创建 `LICENSE` 文件，使用标准 MIT 模板
2. 版权持有人填写 `aoruLola`
3. 年份填写 `2026`
4. 在 README.md 中添加 License 段落引用

**验收标准**：
- `LICENSE` 文件存在于仓库根目录
- README 中有 License badge 或段落
- `cargo deny` 或类似工具能识别

**工作量**：5 分钟

---

### 0.2 Checkout 预检（P0 安全）

**目标**：`ht checkout` 执行前检测本地未提交修改，拒绝覆盖（除非 `--force`）。

**问题描述**：
当前 `checkout` 直接将服务端快照写入工作目录，如果本地有未暂存的修改，会被静默覆盖，导致用户数据丢失。这是五方一致认定的 P0 安全风险。

**实施步骤**：

**Step 1：在 checkout 命令中添加预检逻辑**

修改文件：`crates/cli/src/main.rs`（checkout 函数）

```rust
// 伪代码
fn checkout(args, profile) {
    // 1. 读取当前 workspace state
    let workspace = load_workspace()?;

    // 2. 如果 --force 未设置，执行预检
    if !args.force {
        let conflicts = detect_local_modifications(&workspace)?;
        if !conflicts.is_empty() {
            eprintln!("错误：工作区有未提交的修改，checkout 会覆盖以下文件：");
            for c in &conflicts {
                eprintln!("  {}", c.path);
            }
            eprintln!("使用 'ht add --file <file>' 暂存修改，或使用 '--force' 强制覆盖。");
            std::process::exit(1);
        }
    }

    // 3. 执行原有 checkout 逻辑
    // ...
}
```

**Step 2：实现本地修改检测**

```rust
fn detect_local_modifications(workspace: &WorkspaceState) -> Result<Vec<ConflictEntry>> {
    let mut conflicts = Vec::new();
    for asset in &workspace.checked_out_assets {
        let local_path = Path::new(&workspace.workspace_root).join(&asset.path);
        if local_path.exists() {
            let local_hash = hash_local_file(&local_path)?;
            if local_hash != asset.blob_hash {
                conflicts.push(ConflictEntry {
                    path: asset.path.clone(),
                    base_hash: asset.blob_hash.clone(),
                    local_hash,
                });
            }
        }
    }
    Ok(conflicts)
}
```

**Step 3：为 CheckoutArgs 添加 --force 参数**

```rust
struct CheckoutArgs {
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    branch: Option<String>,
    #[arg(long)]
    to: Option<String>,
    #[arg(long, help = "Force checkout, overwriting local modifications")]
    force: bool,  // 新增
}
```

**Step 4：编写回归测试**

```rust
#[test]
fn checkout_refuses_overwrite_without_force() {
    // 1. 创建 workspace state，包含一个已检出文件
    // 2. 在本地修改该文件内容
    // 3. 执行 checkout（不带 --force）
    // 4. 断言：命令失败，输出包含冲突文件路径
    // 5. 断言：本地文件内容未被覆盖
}

#[test]
fn checkout_force_overwrites() {
    // 1. 创建 workspace state，包含一个已检出文件
    // 2. 在本地修改该文件内容
    // 3. 执行 checkout --force
    // 4. 断言：命令成功，本地文件被覆盖为服务端版本
}
```

**验收标准**：
- `ht checkout` 检测到本地修改时拒绝执行并输出冲突文件列表
- `ht checkout --force` 跳过预检正常执行
- 所有现有测试仍然通过
- 新增 2+ 回归测试

**工作量**：1-2 小时

---

### 0.3 创建 hypertide-enterprise 私有仓库

**目标**：在 GitHub 创建私有仓库，为后续企业功能迁移做准备。

**实施步骤**（需要 aoruLola 手动操作）：

1. 在 GitHub 创建私有仓库 `hypertide-enterprise`
2. 初始化 `Cargo.toml`，定义 workspace 成员：
   - `crates/attestation/` — witness 签名、replay verify
   - `crates/enterprise-auth/` — SSO、RBAC
   - `crates/compliance/` — audit export
3. 在公开仓库的 `Cargo.toml` 中添加 feature flag：`enterprise = ["hypertide-enterprise"]`

**验收标准**：
- 私有仓库存在且可访问
- 公开仓库可以通过 feature flag 引用企业 crate

**工作量**：30 分钟（手动）

---

### 0.4 删除旧 LICENSE 引用（如有）

**目标**：清理旧的 review-only license 引用，统一为 MIT。

**实施步骤**：
1. 检查是否存在 `LICENSE.md` 或其他非 MIT 许可证文件
2. 如果有，删除或替换为 MIT
3. 更新 README 中的 License 段落

**验收标准**：
- 仓库中只有一个 `LICENSE` 文件，且为 MIT

---

## Phase 1 — 架构清理（第 1-2 周）

### 1.1 Event Attribution（P0 基础数据）

**目标**：所有服务端事件绑定 `actor_id`、`workflow_id`、`tool_id`，实现"谁在什么时候用什么工具做了什么"的完整追溯。

**问题描述**：
当前 `EventStore::append` 已有 `actor_id` 参数，但：
- 很多调用方传空字符串或硬编码值
- 缺少 `workflow_id` 和 `tool_id` 字段
- replay 时不追踪 agent 归属

**实施步骤**：

**Step 1：扩展 event_store 表 schema**

创建新 migration：`migrations/202605020001_event_attribution.up.sql`

```sql
ALTER TABLE event_store
    ADD COLUMN IF NOT EXISTS workflow_id TEXT,
    ADD COLUMN IF NOT EXISTS tool_id TEXT,
    ADD COLUMN IF NOT EXISTS session_id TEXT;

CREATE INDEX IF NOT EXISTS idx_event_store_session
    ON event_store(session_id) WHERE session_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_event_store_actor
    ON event_store(actor_id) WHERE actor_id IS NOT NULL;
```

对应 down migration：

```sql
ALTER TABLE event_store
    DROP COLUMN IF EXISTS workflow_id,
    DROP COLUMN IF EXISTS tool_id,
    DROP COLUMN IF EXISTS session_id;
DROP INDEX IF EXISTS idx_event_store_session;
DROP INDEX IF EXISTS idx_event_store_actor;
```

**Step 2：更新 EventStore API**

```rust
pub struct EventMetadata {
    pub actor_id: String,
    pub workflow_id: Option<String>,
    pub tool_id: Option<String>,
    pub session_id: Option<String>,
}

impl EventStore {
    pub async fn append(
        &self,
        event_type: &str,
        repo_id: Option<&str>,
        changeset_id: Option<&str>,
        payload: Value,
        meta: &EventMetadata,  // 新增
    ) -> Result<(), sqlx::Error> { ... }
}
```

**Step 3：更新所有 API handler 的事件记录调用**

需要审查并更新以下文件中的 `event_store.append` 调用：
- `crates/server/src/api/versioning.rs` — submit、approve、promote、rollback
- `crates/server/src/api/storage.rs` — upload
- `crates/server/src/api/lock.rs` — acquire、release、force-release
- `crates/server/src/api/session.rs` — create、save、checkpoint
- `crates/server/src/api/trust.rs` — generate checkpoint、attest

从请求头中提取 `X-HT-Workflow-Id`、`X-HT-Tool-Id`、`X-HT-Session-Id`。

**Step 4：CLI 端发送 attribution header**

在 `crates/cli/src/client.rs` 的 `with_auth` 函数中添加：

```rust
pub fn with_workflow(request: RequestBuilder, profile: &CliProfile) -> RequestBuilder {
    let mut req = with_auth(request, profile);
    if let Some(session) = &profile.current_session {
        req = req.header("X-HT-Session-Id", session);
    }
    req.header("X-HT-Tool-Id", "ht-cli")
}
```

**Step 5：编写测试**

- 测试 event 记录包含正确的 attribution 字段
- 测试按 session_id 查询事件
- 测试按 actor_id 查询事件

**验收标准**：
- 所有 API 事件都包含 `actor_id`（从认证信息提取）
- CLI 发送的事件包含 `tool_id = "ht-cli"`
- 支持按 session_id 和 actor_id 查询事件
- 新 migration 有对应的 down 脚本

**工作量**：3-4 小时

---

### 1.2 Sync/Switch Stage 安全保护（P0 安全）

**目标**：`ht sync` 和 `ht branch switch` 不再静默清空 stage。

**问题描述**：
当前 `sync` 和 `branch switch` 会重置 `stage.json`，导致用户已暂存的修改丢失。这是 ChatGPT 评审指出的 P0 风险。

**实施步骤**：

**Step 1：sync 保留 stage**

修改文件：`crates/cli/src/main.rs`（sync 函数）

```rust
fn sync(args, profile) {
    // 1. 获取服务端快照
    let snapshot = fetch_snapshot(&profile, &repo, &branch)?;

    // 2. 更新 workspace 的 base_changeset_id
    let mut workspace = load_workspace()?;
    workspace.base_changeset_id = Some(snapshot.changeset_id.clone());
    workspace.last_synced_at = now_unix();
    save_workspace(&workspace)?;

    // 3. 更新 stage 的 base_changeset_id（但不清空 assets）
    let mut stage = load_stage()?;
    stage.base_changeset_id = Some(snapshot.changeset_id.clone());
    save_stage(&stage)?;

    // 注意：不再执行 stage = StageFile::default_for_branch(&branch)
}
```

**Step 2：branch switch 检查 stage**

```rust
fn branch_switch(args, profile) {
    let stage = load_stage()?;
    if !stage.assets.is_empty() && !args.force {
        eprintln!("警告：当前 branch 有 {} 个暂存的修改。", stage.assets.len());
        eprintln!("切换分支会清空暂存区。使用 '--force' 强制切换。");
        eprintln!("或者先执行 'ht submit' 保存修改。");
        std::process::exit(1);
    }
    // ... 原有逻辑
}
```

**Step 3：为 BranchSwitchArgs 添加 --force**

```rust
struct BranchSwitchArgs {
    #[arg(long)]
    repo: Option<String>,
    #[arg(long)]
    name: String,
    #[arg(long, help = "Force switch, clearing staged changes")]
    force: bool,  // 新增
}
```

**Step 4：编写测试**

```rust
#[test]
fn sync_preserves_stage() {
    // 1. 创建 stage，包含 2 个暂存资产
    // 2. 执行 sync
    // 3. 断言：stage 中仍有 2 个资产
    // 4. 断言：stage.base_changeset_id 已更新
}

#[test]
fn branch_switch_refuses_with_stage() {
    // 1. 创建 stage，包含暂存资产
    // 2. 执行 branch switch（不带 --force）
    // 3. 断言：命令失败
}

#[test]
fn branch_switch_force_clears_stage() {
    // 1. 创建 stage，包含暂存资产
    // 2. 执行 branch switch --force
    // 3. 断言：命令成功，stage 被清空
}
```

**验收标准**：
- `ht sync` 更新 base_changeset_id 但保留 stage assets
- `ht branch switch` 在 stage 非空时拒绝执行（除非 `--force`）
- 新增 3+ 回归测试

**工作量**：1-2 小时

---

### 1.3 CLI 模块化拆分

**目标**：将 `main.rs`（3500+ 行）按命令拆分为独立模块。

**目标目录结构**：

```
crates/cli/src/
├── main.rs              # 入口 + 命令分发（< 200 行）
├── commands.rs          # clap 参数定义（保留）
├── client.rs            # HTTP 客户端 + 认证（保留）
├── models.rs            # 数据模型（保留）
├── workspace.rs         # 工作区状态管理（保留）
├── cmd/
│   ├── mod.rs           # 模块导出
│   ├── login.rs         # login 命令
│   ├── branch.rs        # branch create/list/switch
│   ├── add.rs           # add 命令
│   ├── remove.rs        # remove 命令
│   ├── submit.rs        # submit 命令
│   ├── checkout.rs      # checkout 命令（含预检）
│   ├── sync.rs          # sync 命令
│   ├── status.rs        # status 命令
│   ├── diff.rs          # diff 命令
│   ├── log.rs           # log 命令
│   ├── rollback.rs      # rollback 命令
│   ├── save.rs          # save 命令
│   ├── checkpoint.rs    # checkpoint create/restore/branch/list
│   ├── changeset.rs     # changeset gate/approve/promote
│   ├── lock.rs          # lock acquire/release/renew/list/force-release
│   ├── trust.rs         # trust checkpoint/witness/audit/replay/retention
│   └── chunk_upload.rs  # chunk-upload 命令
```

**实施步骤**：

**Step 1：创建 cmd/ 目录和 mod.rs**

**Step 2：逐个命令迁移**

每个命令迁移的模式：
1. 将命令函数从 main.rs 移到 `cmd/<name>.rs`
2. 将相关的辅助函数一起迁移
3. 在 `cmd/mod.rs` 中导出
4. 在 main.rs 中通过 `cmd::<name>::execute(args, profile)` 调用

**迁移顺序**（从简单到复杂）：
1. `login.rs` — 最简单，独立
2. `branch.rs` — 3 个子命令
3. `add.rs` + `remove.rs` — 暂存操作
4. `status.rs` + `diff.rs` — 只读查询
5. `sync.rs` + `checkout.rs` — 工作区操作
6. `submit.rs` + `rollback.rs` — 提交操作
7. `save.rs` + `checkpoint.rs` — 会话操作
8. `changeset.rs` + `lock.rs` + `trust.rs` — 治理操作
9. `chunk_upload.rs` — 大文件上传

**Step 3：每步验证**

每迁移一个命令，执行：
- `cargo check -p hypertide-cli`
- `cargo test -p hypertide-cli`
- `cargo clippy -p hypertide-cli`

**验收标准**：
- main.rs < 200 行（仅入口 + 命令分发）
- 每个 cmd/*.rs 文件 < 300 行
- 所有现有测试通过
- `cargo clippy` 无警告

**工作量**：4-6 小时（可分多天完成）

---

### 1.4 危险操作确认

**目标**：对高风险操作添加交互式确认或 `--yes` 跳过。

**涉及命令**：
- `lock force-release` — 管理员强制释放他人锁
- `rollback` — 回滚到历史版本
- `changeset promote` — 推广到 visible head
- `branch switch --force` — 强制切换分支（清空 stage）

**实施模式**：

```rust
fn confirm_dangerous(action: &str, args_yes: bool) -> Result<()> {
    if args_yes {
        return Ok(());
    }
    eprint!("危险操作：{}。确认执行？[y/N] ", action);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim().to_lowercase() != "y" {
        eprintln!("已取消。");
        std::process::exit(0);
    }
    Ok(())
}
```

每个命令添加 `--yes` 参数：

```rust
#[arg(long, help = "Skip confirmation prompt")]
yes: bool,
```

**验收标准**：
- 4 个命令都有 `--yes` 参数
- 不带 `--yes` 时显示确认提示
- 输入非 `y` 时取消操作

**工作量**：1 小时

---

## Phase 2 — 核心对齐（第 3-4 周）

### 2.1 Checkpoint 概念对齐

**目标**：区分 WorkspaceCheckpoint（用户级）和 TrustAttestation（系统级），消除命名冲突。

**当前混乱**：
- `ht save` — agent 保存进度
- `ht checkpoint create` — agent 检查点
- `ht trust checkpoint generate` — 系统信任检查点
- 三个概念共享 "checkpoint" 一词

**对齐方案**：

| CLI 命令 | 含义 | 服务端概念 |
|---------|------|-----------|
| `ht save` | 保存当前工作进度 | Agent Session Save |
| `ht checkpoint create` | 创建可恢复的检查点 | WorkspaceCheckpoint |
| `ht trust checkpoint generate` | 生成系统状态证明 | TrustAttestation |

**实施步骤**：
1. 服务端 API 路径不变（避免 breaking change）
2. CLI help 文本统一术语
3. `ht save --help` 明确说明"保存进度，不推进 branch head"
4. `ht checkpoint --help` 明确说明"创建可恢复的工作区检查点"
5. `ht trust checkpoint --help` 明确说明"生成系统状态证明，用于审计"

**工作量**：2 小时（文档 + help 文本）

---

### 2.2 Replay 增量模式

**目标**：将 replay 从全表扫描 O(n) 改为按 checkpoint 差量增量模式。

**当前问题**：
`ReplayService` 的 `verify` 方法扫描整个 `event_store` 表，对于大型仓库不可接受。

**实施步骤**：

**Step 1：添加 checkpoint 到 event 的映射**

```sql
CREATE TABLE IF NOT EXISTS replay_checkpoints (
    checkpoint_id TEXT PRIMARY KEY,
    event_seq BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Step 2：实现增量 replay**

```rust
impl ReplayService {
    pub async fn verify_incremental(
        &self,
        from_checkpoint: Option<&str>,
    ) -> Result<ReplayVerification, HyperTideError> {
        let start_seq = if let Some(cp_id) = from_checkpoint {
            self.get_checkpoint_seq(cp_id).await?
        } else {
            0
        };

        // 只扫描 start_seq 之后的事件
        let events = sqlx::query_as::<_, EventRow>(
            "SELECT * FROM event_store WHERE seq > $1 ORDER BY seq ASC"
        )
        .bind(start_seq)
        .fetch_all(&self.pool)
        .await?;

        // ... replay 逻辑
    }
}
```

**Step 3：添加 CLI 参数**

```powershell
ht trust replay verify --from-checkpoint <checkpoint-id>
```

**验收标准**：
- 增量 replay 只处理指定 checkpoint 之后的事件
- 性能提升：10000 条事件时，增量 replay < 1 秒

**工作量**：3-4 小时

---

### 2.3 修复 Attestation 签名

**目标**：将 `blake3(secret || data)` 替换为真正的 HMAC-SHA256。

**当前问题**（Claude Code 指出）：
`blake3(secret || data)` 不是标准 HMAC，存在长度扩展攻击风险。

**实施步骤**：

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

fn compute_attestation_signature(secret: &[u8], data: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret)
        .expect("HMAC accepts any key length");
    mac.update(data);
    hex::encode(mac.finalize().into_bytes())
}
```

**依赖更新**：`Cargo.toml` 添加 `hmac = "0.12"` 和 `sha2 = "0.10"`

**验收标准**：
- attestation 签名使用 HMAC-SHA256
- 现有签名验证兼容（需要 migration 或兼容层）
- 测试向量验证通过

**工作量**：2-3 小时

---

### 2.4 锁模型升级

**目标**：锁 key 从 `file_path` 升级为 `(repo_id, scope, asset_id)`。

**当前问题**：
锁只以 `file_path` 为 key，如果同一文件在不同 repo/branch 中存在，会产生冲突。

**实施步骤**：

**Step 1：扩展锁表 schema**

```sql
ALTER TABLE locks
    ADD COLUMN IF NOT EXISTS repo_id TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS scope TEXT NOT NULL DEFAULT 'asset';

-- 更新唯一约束
ALTER TABLE locks DROP CONSTRAINT IF EXISTS locks_file_path_key;
ALTER TABLE locks ADD CONSTRAINT locks_repo_scope_path
    UNIQUE (repo_id, scope, file_path);
```

**Step 2：更新 LockManager**

```rust
pub struct LockKey {
    pub repo_id: String,
    pub scope: String,  // "asset", "branch", "repo"
    pub file_path: String,
}

pub async fn try_lock(&self, key: LockKey, owner_id: String) -> Result<FileLock, HyperTideError> { ... }
```

**Step 3：更新 CLI lock 命令**

```powershell
ht lock acquire --path Content/Props/tree.uasset --repo demo-repo
```

**验收标准**：
- 锁支持 repo_id 维度
- 同一文件在不同 repo 中可以独立锁定
- 向后兼容（repo_id 默认为空字符串）

**工作量**：3-4 小时

---

### 2.5 错误信息用户友好化

**目标**：HTTP 错误码映射为 CLI 友好提示。

**实施步骤**：

在 CLI 中添加错误映射层：

```rust
fn friendly_error(err: &ApiError) -> String {
    match err.code.as_str() {
        "conflict" if err.message.contains("locked by") => {
            let owner = extract_lock_owner(&err.message);
            format!(
                "文件已被 {} 锁定。\n\
                 使用 'ht lock list' 查看所有锁。\n\
                 使用 'ht lock release --path <path>' 释放锁。",
                owner
            )
        }
        "conflict" if err.message.contains("BaseChangesetMismatch") => {
            "本地基线已过期。请先执行 'ht sync' 更新基线。".to_string()
        }
        "validation_error" if err.message.contains("nothing staged") => {
            "暂存区为空。请先执行 'ht add --file <file>' 暂存文件。".to_string()
        }
        _ => format!("错误：{}", err.message),
    }
}
```

**验收标准**：
- 常见错误有中文友好提示
- 提示中包含修复建议

**工作量**：2 小时

---

## Phase 3 — 对外发布（第 5-6 周）

### 3.1 README 重写

**目标**：基于新定位"Replayable Development History"重写 README。

**新 README 结构**：
1. 一句话定位
2. 为什么不是 Git（NOT 列表）
3. 核心能力（带代码示例）
4. 快速开始（5 分钟跑通）
5. CLI 命令速查
6. 架构概览
7. 安全与治理
8. 贡献指南
9. License

**工作量**：2-3 小时

---

### 3.2 ht log --graph

**目标**：可视化 checkpoint 谱系，展示 changeset 之间的父子关系。

**这是 aoruLola 认定的 killer feature。**

**实施步骤**：

```powershell
ht log --repo demo-repo --branch main --graph --limit 20
```

输出示例：
```
* 5d7e4fb (2026-05-02) fix: CI format fixes
* 8a9f179 (2026-05-02) feat: CLI 统一 + 回归测试
|
* 05235f9 (2026-05-01) docs: split readme by language
* e272a54 (2026-05-01) docs: add review-only license
|
* 6c54bbc (2026-04-30) docs: expand bilingual README
|
| * f14f5f8 (2026-04-29) feat(cli): governance command surface
|/
* 0dd236c (2026-04-28) Merge: agent-session-checkpoints
```

**工作量**：4-6 小时

---

### 3.3 ht doctor

**目标**：一键检查登录状态、服务端连通性、repo 配置、stage 状态。

```powershell
ht doctor
```

输出示例：
```
✓ 登录状态：已登录（api_key, 直接模式）
✓ 服务端连通：http://localhost:3000 响应正常
✓ 默认仓库：demo-repo 存在
✓ 默认分支：main 存在
✓ 工作区状态：已检出 42 个资产
⚠ 暂存区：有 3 个待提交的修改
✗ Token 过期：将在 5 分钟后过期，建议执行 'ht login' 刷新
```

**工作量**：2-3 小时

---

### 3.4 ht stage list / clear

**目标**：显式管理暂存区。

```powershell
# 查看暂存内容
ht stage list

# 清空暂存区
ht stage clear

# 清空暂存区（确认）
ht stage clear --yes
```

**工作量**：1-2 小时

---

### 3.5 存储后端抽象

**目标**：定义 `StorageBackend` trait，提供 LocalFS 和 S3 实现。

**实施步骤**：

**Step 1：定义 trait**

```rust
// crates/server/src/core/storage_backend.rs
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store(&self, hash: &str, data: &[u8]) -> Result<(), StorageError>;
    async fn retrieve(&self, hash: &str) -> Result<Vec<u8>, StorageError>;
    async fn exists(&self, hash: &str) -> Result<bool, StorageError>;
    async fn delete(&self, hash: &str) -> Result<(), StorageError>;
    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError>;
}
```

**Step 2：实现 LocalFsBackend**

```rust
pub struct LocalFsBackend {
    root: PathBuf,
}

#[async_trait]
impl StorageBackend for LocalFsBackend {
    // 复用现有 StorageManager 逻辑
}
```

**Step 3：实现 S3Backend**

```rust
pub struct S3Backend {
    bucket: String,
    prefix: String,
    client: aws_sdk_s3::Client,
}

#[async_trait]
impl StorageBackend for S3Backend {
    // S3 实现
}
```

**Step 4：配置驱动选择**

```toml
# .env
STORAGE_BACKEND=local  # 或 s3
STORAGE_PATH=./storage
# S3_BUCKET=hypertide-assets
# S3_PREFIX=production/
# S3_REGION=us-east-1
```

**工作量**：6-8 小时

---

### 3.6 JSON 输出模式

**目标**：所有命令支持 `--json` 参数，输出结构化 JSON。

```powershell
ht status --json
```

输出：
```json
{
  "branch": "main",
  "repo": "demo-repo",
  "assets": [
    {"path": "Content/Props/tree.uasset", "status": "modified", "base_hash": "abc123", "local_hash": "def456"},
    {"path": "Content/Textures/grass.png", "status": "staged", "blob_hash": "789abc"}
  ]
}
```

**工作量**：3-4 小时

---

### 3.7 Shell 补全

**目标**：生成 bash/zsh/fish/PowerShell 补全脚本。

```powershell
ht completions bash > /etc/bash_completion.d/ht
ht completions zsh > ~/.zfunc/_ht
ht completions fish > ~/.config/fish/completions/ht.fish
ht completions powershell > $PROFILE
```

使用 clap 的 `clap_complete` crate。

**工作量**：1 小时

---

## Phase 4 — 产品化（Q3）

### 4.1 Web UI 可视化时间线

**目标**：在 Tauri + React 前端中实现 changeset 时间线可视化。

**核心功能**：
- 分支时间线图（类似 Git graph）
- Checkpoint 谱系可视化
- 点击 changeset 查看详情（assets、diff、author）
- 锁状态面板

**工作量**：2-3 周

---

### 4.2 Cloud SaaS 原型

**目标**：提供托管版本，降低用户部署门槛。

**实施方向**：
- 多租户 PostgreSQL
- API Key 管理
- 用量计量
- 支付集成

**工作量**：4-6 周

---

### 4.3 速率限制 + 指标

**目标**：生产级中间件。

```rust
// 速率限制
use tower::limit::RateLimitLayer;

let rate_limit = RateLimitLayer::new(100, Duration::from_secs(1));

// Prometheus 指标
use metrics::{counter, histogram};

counter!("http_requests_total", "method" => "POST", "path" => "/v2/changesets").increment(1);
histogram!("http_request_duration_seconds", "method" => "GET").record(duration);
```

**工作量**：3-4 小时

---

### 4.4 Git 桥接

**目标**：允许 Git 仓库作为 HyperTide 的一个分支，实现双向同步。

**使用场景**：
- 开发者在 Git 中写代码，HyperTide 管理构建产物
- CI 系统将 Git commit 关联到 HyperTide changeset

**工作量**：2-3 周

---

## 附录 A：依赖关系图

```
Phase 0.1 (License) ──────────────────────────────────────┐
Phase 0.2 (checkout 预检) ──→ Phase 1.2 (stage 保护) ──→ Phase 2.5 (锁模型)
Phase 0.3 (enterprise repo) ──→ Phase 1.1 (attribution) ──→ Phase 2.1 (checkpoint)
                                                          ──→ Phase 2.2 (replay)
Phase 1.3 (CLI 模块化) ──→ Phase 3.4 (ht stage)
                         ──→ Phase 3.3 (ht doctor)
Phase 1.4 (危险操作确认) ──→ Phase 2.3 (attestation 签名)
Phase 3.5 (存储抽象) ──→ Phase 4.4 (Git 桥接)
Phase 3.1 (README) ──→ Phase 3 (开源发布)
```

---

## 附录 B：风险登记

| 风险 | 概率 | 影响 | 缓解措施 |
|------|:----:|:----:|---------|
| CLI 模块化拆分破坏命令注册 | 中 | 中 | 分步做，每步保留测试，先迁移简单命令 |
| checkout 预检误报（hash 不一致但内容相同） | 低 | 中 | 使用 BLAKE3 校验实际内容，不依赖文件时间戳 |
| event attribution 增加 API 延迟 | 低 | 低 | 异步写入 event_store，不阻塞主流程 |
| HMAC 签名迁移破坏现有 attestation | 中 | 高 | 提供兼容层，同时支持新旧签名格式 |
| 锁模型升级需要数据迁移 | 中 | 中 | 新字段默认空字符串，渐进式迁移 |
| 存储抽象引入 S3 依赖增加编译时间 | 低 | 低 | 使用 feature flag，默认只编译 LocalFS |

---

## 附录 C：验收检查清单

### Phase 0 完成标准
- [ ] MIT LICENSE 文件存在
- [ ] `ht checkout` 检测本地修改并拒绝覆盖
- [ ] `ht checkout --force` 正常覆盖
- [ ] hypertide-enterprise 私有仓库已创建
- [ ] 所有现有测试通过

### Phase 1 完成标准
- [ ] 所有 event 包含 actor_id
- [ ] CLI 发送 tool_id 和 session_id header
- [ ] `ht sync` 不清空 stage
- [ ] `ht branch switch` 在 stage 非空时拒绝（除非 --force）
- [ ] main.rs < 200 行
- [ ] 每个 cmd/*.rs < 300 行
- [ ] 4 个危险命令有 --yes 确认

### Phase 2 完成标准
- [ ] save / checkpoint / trust checkpoint 术语统一
- [ ] 增量 replay 实现且性能达标
- [ ] attestation 使用 HMAC-SHA256
- [ ] 锁支持 repo_id 维度
- [ ] 常见错误有友好中文提示

### Phase 3 完成标准
- [ ] README 基于新定位重写
- [ ] `ht log --graph` 实现
- [ ] `ht doctor` 实现
- [ ] `ht stage list/clear` 实现
- [ ] StorageBackend trait + LocalFS + S3 实现
- [ ] 所有命令支持 `--json`
- [ ] Shell 补全脚本生成

---

*文档版本: v1.0 | 2026-05-02 | 基于五方评审综合方案*
