# HTTP / desktop / types contract tests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lock Rust `TaskUpdate` / `TaskSummary` / `EntityType` against `packages/types`, and reject unknown HTTP/desktop task patch fields with 400.

**Architecture:** File-reading contract tests (same style as `skill_contract.rs`) compare snake_case Rust fields to camelCase TS keys. No codegen. HTTP `PatchTaskBody` and a desktop patch DTO use `#[serde(deny_unknown_fields)]`; HTTP maps serde failures to 400.

**Tech Stack:** Rust integration tests, Axum JSON, serde, Vitest not required.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `run finish` still does not move the card
- CLI JSON stays snake_case; HTTP stays camelCase
- No large codegen / OpenAPI rewrite
- Do not invent a third naming convention
- No new product fields beyond locking what CLI `task update` already has (`title`, `worktree`, `branch`) plus the existing combined HTTP/desktop patch surface

User already chose sequential inline execution.

## File map

- Create: `crates/cli/tests/types_contract.rs`
- Modify: `crates/api/src/dto.rs` — `#[serde(deny_unknown_fields)]` on `PatchTaskBody`
- Modify: `crates/api/src/routes.rs` — unknown JSON fields → 400
- Modify: `crates/api/tests/routes.rs` — unknown field 400
- Modify: `crates/desktop-commands/src/commands.rs` — derive Deserialize + deny_unknown on `TaskPatchArgs`
- Modify: `crates/desktop-commands/tests/commands.rs` — extra key is rejected

---

### Task 1: Types contract tests

**Files:**
- Create: `crates/cli/tests/types_contract.rs`

**Interfaces:**
- Consumes: `crates/core/src/models.rs` `TaskSummary` / `EntityType`; `crates/application/src/commands.rs` `TaskUpdate`; `packages/types/src/index.ts` `TaskSummary` / `TaskPatch` / `EntityType`
- Produces: CI red if keys drift

```rust
fn snake_to_camel(name: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for ch in name.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}
```

- [x] **Step 1: Write failing tests** that `TaskSummary` rust fields map 1:1 to TS `TaskSummary`; `TaskUpdate` product fields `title` / `worktree_path` / `branch` exist on TS `TaskPatch`; `EntityType` includes `comment` and `check` on both sides.

```rust
#[test]
fn task_summary_keys_match_types_package() { /* parse both, camelCase compare */ }

#[test]
fn task_update_product_fields_are_on_task_patch() {
    // title, worktreePath, branch on TaskPatch
}

#[test]
fn entity_type_includes_comment_and_check() {
    // rust enum + TS union
}
```

- [x] **Step 2: Run to fail** — `cargo test -p taskboard-cli --test types_contract`

- [x] **Step 3: Implement** the parser + assertions (types already have the keys after #135/#137; this task locks them)

- [x] **Step 4: Tests pass**

- [x] **Step 5: Commit** `test: lock TaskSummary TaskUpdate EntityType against types`

---

### Task 2: Unknown patch fields are 400

**Files:**
- Modify: `crates/api/src/dto.rs`, `routes.rs`, `tests/routes.rs`
- Modify: `crates/desktop-commands/src/commands.rs`, `tests/commands.rs`

**Interfaces:**
- `PatchTaskBody` and `TaskPatchArgs` `#[serde(deny_unknown_fields)]`
- HTTP PATCH `/api/v1/tasks/TASK-1` with `{ "nope": true }` → 400
- Desktop: `serde_json::from_value::<TaskPatchArgs>(json!({..., "nope": true}))` fails

```rust
#[tokio::test]
async fn patch_task_unknown_field_is_400() {
    let s = seeded_task_server().await;
    let res = authed(&s)
        .patch(format!("{}/api/v1/tasks/TASK-1", s.base))
        .json(&json!({"nope": true}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
}
```

CLI `task update` already accepts `--title` / `--worktree` / `--branch`; HTTP/desktop already map those. This task only rejects extras.

- [x] **Step 1: Failing HTTP + desktop tests**

- [x] **Step 2: Run to fail**

- [x] **Step 3: Implement** `deny_unknown_fields` + map `JsonRejection` to 400 with `validation_error`

- [x] **Step 4: Tests pass**

- [x] **Step 5: Commit** `fix(api): reject unknown task patch fields with 400`

---

## Self-review

1. Spec: key lock + same optional CLI update fields + unknown 400 + EntityType comment/check + no codegen.
2. No placeholders.
3. HTTP camelCase / CLI snake_case unchanged.
