# HyperTide Object Model

> Version: 0.1 (draft)
> Status: Proposed — not yet implemented against

## Core Principle

The entire system revolves around **one** core object: **Checkpoint**.

Everything else — save, submit, branch, rollback, replay — is a derived operation on Checkpoint with different state.

## 1. Checkpoint

```rust
struct Checkpoint {
    id:         Hash,
    parent:     Option<CheckpointId>,
    state:      CheckpointState,
    workspace:  WorkspaceSnapshot,
    execution:  ExecutionLog,
    attestation: Option<Attestation>,
    metadata:   Metadata,
}

enum CheckpointState {
    Temporary,    // 「草稿」— 原 save
    Reviewable,   // 「可审阅」— 原 changeset / submit
    Promoted,     // 「已发布」— 原 promoted changeset
}
```

### State Transition

```
Temporary ──submit──→ Reviewable ──promote──→ Promoted
     ↑                    │
     └──── rollback ──────┘
     (回到之前的 Temporary)
```

### Key Behaviors

| Operation | Old Name | New Semantics |
|-----------|----------|---------------|
| `ht save` | save | Create a Temporary checkpoint of current workspace |
| `ht submit` | submit | Advance Temporary → Reviewable |
| `ht promote` | (new) | Advance Reviewable → Promoted |
| `ht rollback` | rollback | Restore workspace to a previous checkpoint's state |
| `ht replay` | replay | Reconstruct workspace state from an execution log |
| `ht log` | log | Show checkpoint timeline with ancestry |

## 2. WorkspaceSnapshot

```rust
struct WorkspaceSnapshot {
    root_hash:  Hash,
    assets:     Vec<AssetEntry>,
    toolchain:  ToolchainInfo,
    parameters: HashMap<String, Value>,  // generator params, environment, etc.
}

struct AssetEntry {
    path:       String,
    blob_hash:  Hash,
    asset_type: AssetKind,
    metadata:   HashMap<String, Value>,
}
```

A WorkspaceSnapshot captures **what the workspace looked like**, not just file contents. This includes:
- File tree (content-addressed via root_hash)
- Generator parameters that produced certain assets
- Toolchain versions involved
- Pending outputs not yet materialized

## 3. ExecutionLog

```rust
struct ExecutionLog {
    events: Vec<Event>,
}

struct Event {
    seq:        u64,
    kind:       EventKind,
    payload:    Value,        // JSON — action-specific data
    agent:      AgentId,
    timestamp:  DateTime<Utc>,
    parent:     Option<EventId>,
}
```

### Event Kinds

```
AssetCreated | AssetModified | AssetDeleted
BranchCreated | BranchSwitched
CheckpointCreated | CheckpointPromoted
LockAcquired | LockReleased
AgentAction  // AI-specific: prompt, tool call, result
ExternalTool // import, build, export
```

The execution log is **append-only**. It is the source of truth for replay. It is not deterministic — it records what actually happened, not what should happen.

## 4. Attestation

```rust
struct Attestation {
    signature:  String,
    witness_id: String,
    scope:      String,
    // Proves: "this checkpoint's log → state_root is correct"
}
```

Renamed from "witness receipt". An attestation is an optional cryptographic proof that a checkpoint's execution log, when replayed, produces the claimed workspace state root. This is useful for compliance/audit but entirely optional for normal use.

## 5. Asset

```rust
struct Asset {
    hash: Hash,
    size: u64,
    kind: AssetKind,  // texture, mesh, material, blueprint, etc.
}
```

Content-addressed binary blob. Assets are **immutable** — identified by their hash, stored in CAS storage. Asset metadata (name, path, semantic properties) lives in the workspace snapshot, not on the blob itself.

## 6. Agent

```rust
struct Agent {
    id:   AgentId,
    kind: AgentKind,   // human | ai | ci | toolchain
    name: String,
}
```

An actor that produces events. Agents are first-class citizens because the system is designed for **human + AI collaboration**. Every event in the execution log is attributed to an agent.

---

## Key Design Decisions

### 1. Checkpoint is the only persistent object

No separate "changeset", "commit", or "save" object. Everything is a Checkpoint with a state. This means:
- One query pattern: `get_checkpoint(id)`
- One mutation pattern: `create_checkpoint(parent, state, workspace)`
- One history pattern: `checkpoint.ancestors()`

### 2. Replay = Execution Reconstruction, not Deterministic Re-execution

Replay reads events from the execution log and reconstructs the workspace state. It does NOT re-run AI models or external toolchains. It is a **computational reconstruction** (the state transitions implied by the events), not a **literal re-execution**.

### 3. Assets are immutable, Workspaces are mutable

Assets go into CAS and never change. Workspace snapshots are lightweight pointers into the CAS. This is Git's approach and it's correct.

### 4. Metadata is extensible

Everything carries a `HashMap<String, Value>` for metadata. This is critical for:
- AI prompt/response storage
- Toolchain version tracking
- Custom integration data
- Future semantic annotation

---

## Open Questions

1. Should Checkpoint state transitions require server approval (gate)?
   - Currently leaning: no for Temporary→Reviewable, optional for Reviewable→Promoted
2. Should Temporary checkpoints be syncable to server, or only local?
   - Leaning: syncable (for AI agent collaboration)
3. Attestation scope — single checkpoint or entire lineage?
   - Leaning: per-checkpoint, but verifiable against parent chain
