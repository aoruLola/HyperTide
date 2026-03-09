# HyperTide M11-005 Cross-Environment Witness Topology Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend witness topology metadata so M11 can describe and validate cross-environment witness layouts instead of only same-environment scope listings.

**Architecture:** Keep the existing witness service as the single source of configuration parsing and topology reporting. Add environment-aware parsing and a richer topology envelope that exposes grouped environments and whether quorum can be satisfied across multiple environments, then wire the existing API/tests/documents to the new shape.

**Tech Stack:** Rust, Axum, SQLx, serde, OpenAPI YAML

---

### Task 1: Define cross-environment topology behavior in tests

**Files:**
- Modify: `src/core/witness.rs`

**Step 1: Write the failing test**

Add unit tests that configure witness keys across at least two environments and assert topology reports:
- witness environment metadata per witness
- grouped environment summaries
- `cross_environment` true when multiple environments exist
- `cross_environment_quorum_met` true only when quorum can be met without relying on a single environment

**Step 2: Run test to verify it fails**

Run: `cargo test witness_topology_reports_cross_environment_quorum --bin hypertide`
Expected: FAIL because the current topology model does not expose environment metadata or cross-environment quorum status.

**Step 3: Write minimal implementation**

Extend witness parsing and topology shaping in `src/core/witness.rs` with optional environment metadata and backward-compatible defaults.

**Step 4: Run test to verify it passes**

Run: `cargo test witness_topology_reports_cross_environment_quorum --bin hypertide`
Expected: PASS

### Task 2: Expose the richer topology contract at the API boundary

**Files:**
- Modify: `src/main.rs`
- Modify: `docs/api/openapi.yaml`

**Step 1: Write the failing test**

Add or extend route-level tests to assert `/v2/trust/witness/topology` returns the new environment-aware fields.

**Step 2: Run test to verify it fails**

Run: `cargo test witness_topology --bin hypertide`
Expected: FAIL because the API payload still reflects the old topology shape.

**Step 3: Write minimal implementation**

Reuse the enriched witness service output through the existing handler and document the updated response contract in OpenAPI.

**Step 4: Run test to verify it passes**

Run: `cargo test witness_topology --bin hypertide`
Expected: PASS

### Task 3: Align milestone documents with implemented scope

**Files:**
- Modify: `docs/plans/2026-02-26-hypertide-m8-m11-todo.md`
- Modify: `docs/plans/2026-02-26-hypertide-m11-ops-handbook.md`

**Step 1: Update plan status**

Mark `M11-004` done if the implemented compliance endpoints and ops handbook satisfy the roadmap item. Mark `M11-005` done after the topology work is merged.

**Step 2: Update operations wording**

Document that topology now includes cross-environment grouping/quorum metadata.

**Step 3: Verify docs match code**

Run: `rg -n "M11-004|M11-005|witness topology" docs`
Expected: statuses and wording match the implemented API surface.

### Task 4: Final verification

**Files:**
- Modify: none

**Step 1: Run targeted tests**

Run: `cargo test witness_topology --bin hypertide`
Expected: PASS

**Step 2: Run broader binary checks**

Run: `cargo check --locked --bin hypertide --bin ht`
Expected: PASS
