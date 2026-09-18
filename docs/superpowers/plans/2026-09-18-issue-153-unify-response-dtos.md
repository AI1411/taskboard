# Unify HTTP/desktop response DTOs (#153) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the 17 identical camelCase response DTOs (plus `json_keys_to_camel`) from `api` and `desktop-commands` into one shared `taskboard-wire` crate so HTTP and desktop cannot drift.

**Architecture:** New workspace crate `crates/wire` owns Serialize response DTOs + `From` conversions + `json_keys_to_camel` / `snake_to_camel`. `api` and `desktop-commands` re-export those types and keep request bodies / Tauri-only args / `RunOp` locally. Extend `types_contract.rs` so a duplicated `pub struct ProjectDto` (etc.) in api or desktop-commands fails the build.

**Tech Stack:** Rust workspace, serde, `taskboard-core` / `taskboard-application` / `taskboard-store-sqlite`, existing CLI integration test `types_contract`.

## Global Constraints

- Behavior-preserving — no wire field renames or payload changes
- Request bodies / Tauri args stay on each surface
- `packages/types` stays hand-maintained (contract only)
- Do not unify `RunOp` or request DTOs in this issue

User already chose sequential inline execution.

## File map

- Create: `crates/wire/Cargo.toml`, `crates/wire/src/lib.rs` (response DTOs + helpers)
- Modify: root `Cargo.toml` — add `crates/wire` member
- Modify: `crates/api/Cargo.toml`, `crates/api/src/dto.rs`, `crates/api/src/routes.rs` (if import paths change)
- Modify: `crates/desktop-commands/Cargo.toml`, `crates/desktop-commands/src/dto.rs`, `crates/desktop-commands/src/error.rs`
- Modify: `crates/cli/tests/types_contract.rs` — assert dual definitions are gone
- Keep: desktop `UndoResultDto` moves into wire (desktop-only today but uses shared helper); API request structs stay in api

## Shared types to move (byte-identical today)

`ProjectDto`, `TaskSummaryDto`, `LinkDto`, `CommentDto`, `CheckDto`, `RunDto`, `ActivityDto`, `TaskDetailDto`, `SyncDeltaDto`, `TrashDto`, `InboxItemDto`, `StatusLineDto`, `InboxCountsDto`, `BoardStatusDto`, `OccupancyRunDto`, `OccupancyGroupDto`, `UiStateDto`, plus `json_keys_to_camel` / `snake_to_camel`. Also move `UndoResultDto` + `From<UndoResult>` into wire (single owner).

---

### Task 1: Failing contract — dual definitions must disappear

**Files:**
- Modify: `crates/cli/tests/types_contract.rs`

**Interfaces:**
- Produces: test `response_dtos_live_only_in_wire` that fails until wire exists and api/desktop stop defining the structs

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn response_dtos_live_only_in_wire() {
    let names = [
        "ProjectDto", "TaskSummaryDto", "LinkDto", "CommentDto", "CheckDto",
        "RunDto", "ActivityDto", "TaskDetailDto", "SyncDeltaDto", "TrashDto",
        "InboxItemDto", "StatusLineDto", "InboxCountsDto", "BoardStatusDto",
        "OccupancyRunDto", "OccupancyGroupDto", "UiStateDto", "UndoResultDto",
    ];
    let wire = read_repo("crates/wire/src/lib.rs");
    let api = read_repo("crates/api/src/dto.rs");
    let desktop = read_repo("crates/desktop-commands/src/dto.rs");
    for name in names {
        let header = format!("pub struct {name} {{");
        assert!(
            wire.contains(&header),
            "{name} must be defined in crates/wire"
        );
        assert!(
            !api.contains(&header),
            "{name} must not be redefined in api dto.rs"
        );
        assert!(
            !desktop.contains(&header),
            "{name} must not be redefined in desktop-commands dto.rs"
        );
    }
    assert!(
        wire.contains("pub fn json_keys_to_camel"),
        "json_keys_to_camel must live in wire"
    );
    assert!(
        !api.contains("pub fn json_keys_to_camel"),
        "api must not redefine json_keys_to_camel"
    );
    assert!(
        !desktop.contains("pub fn json_keys_to_camel"),
        "desktop must not redefine json_keys_to_camel"
    );
}
```

- [ ] **Step 2: Run to fail**

Run: `cargo test -p taskboard-cli --test types_contract response_dtos_live_only_in_wire -- --nocapture`  
Expected: FAIL (missing `crates/wire` and/or structs still in api/desktop)

- [ ] **Step 3: Commit** (optional mid-point) or continue Task 2 in same commit series

---

### Task 2: Create `taskboard-wire` and switch consumers

**Files:**
- Create: `crates/wire/*`
- Modify: workspace + api + desktop-commands Cargo.toml / dto modules

**Interfaces:**
- `taskboard_wire::{ProjectDto, …, json_keys_to_camel}`
- api/desktop: `pub use taskboard_wire::*;` (or selective) from `dto.rs` so existing `use crate::dto::ProjectDto` keeps working

- [ ] **Step 1: Scaffold crate**

```toml
# crates/wire/Cargo.toml
[package]
name = "taskboard-wire"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
chrono.workspace = true
serde.workspace = true
serde_json.workspace = true
taskboard-application = { path = "../application" }
taskboard-core = { path = "../core" }
taskboard-store-sqlite = { path = "../store-sqlite" }
uuid.workspace = true
```

Add `"crates/wire"` to workspace `members`.

- [ ] **Step 2: Move response DTO source into `crates/wire/src/lib.rs`**

Copy the shared struct + `From` blocks and helpers from `crates/api/src/dto.rs` / desktop `UndoResultDto`. Include unit test `project_dto_serializes_camel_case_keys` from desktop.

- [ ] **Step 3: Slim api/desktop dto.rs**

- api: delete moved structs/helpers; `pub use taskboard_wire::{…};` keep request bodies, `RunOp`, `empty_to_none`, `deserialize_present_option` if present
- desktop: same; keep `RunOp`, `deserialize_present_option`; re-export wire types
- `error.rs` / `routes.rs`: keep importing `json_keys_to_camel` via `crate::dto` re-export

- [ ] **Step 4: Tests pass**

```bash
cargo test -p taskboard-cli --test types_contract -- --test-threads=1
cargo test -p taskboard-api -p taskboard-desktop-commands -p taskboard-wire -- --test-threads=1
cargo clippy --all-targets -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git commit -m "refactor(wire): share HTTP/desktop response DTOs (#153)"
```

---

### Task 3: PR merge gate

- [ ] Open PR, wait CI green, merge, delete branch, check off #153 on refactor-implementation.md

## Self-review

1. Spec coverage: shared crate, single `json_keys_to_camel`, contract against dual defs — yes.
2. No placeholders.
3. Request DTOs remain local; response names unchanged.
