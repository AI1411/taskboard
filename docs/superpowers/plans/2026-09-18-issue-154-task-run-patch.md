# Move task_patch / run_patch orchestration into application (#154) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Own task-patch and run-patch orchestration once in `taskboard-application` so HTTP and desktop only adapt headers/args.

**Architecture:** Add `TaskPatch`, `RunPatch`, and `RunPatchOp` to `application::commands`. Implement `App::task_patch` / `App::run_patch` with the current title→workspace→note→urgent→column→reorder sequence and RunOp match. Share `empty_to_none`. HTTP/desktop map wire bodies into these commands.

**Tech Stack:** Rust application / api / desktop-commands; existing API + desktop tests.

## Global Constraints

- Behavior-preserving — same patch order and revision chaining (`Some(updated.revision)` after each step)
- No new patch fields
- Request serde types may remain on surfaces; domain `RunPatchOp` lives in application

User already chose sequential inline execution.

## File map

- Modify: `crates/application/src/commands.rs` — `TaskPatch`, `RunPatch`, `RunPatchOp`, export
- Modify: `crates/application/src/lib.rs` — re-export
- Modify: `crates/application/src/app/mod.rs` (+ maybe `tasks.rs`/`runs.rs`) — `task_patch` / `run_patch`
- Modify: `crates/api/src/routes.rs` — call App methods
- Modify: `crates/desktop-commands/src/commands.rs` — call App methods
- Modify: keep surface `RunOp` serde enums mapping into `RunPatchOp` (or re-export)

---

### Task 1: Application commands + App methods

**Files:** application commands + app façade

**Interfaces:**
```rust
pub struct TaskPatch {
    pub display_id: String,
    pub title: Option<String>,
    pub note_markdown: Option<String>,
    pub urgent: Option<bool>,
    pub column: Option<Column>,
    pub before_display_id: Option<Option<String>>,
    pub worktree_path: Option<String>,
    pub branch: Option<String>,
    pub revision: Option<i64>,
}
pub enum RunPatchOp { Update, Wait, Fail, Finish, Cancel }
pub struct RunPatch {
    pub run_display_id: String,
    pub op: RunPatchOp,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub summary: Option<String>,
    pub revision: Option<i64>,
}
pub fn empty_to_none(value: String) -> Option<String>;
// App::task_patch(actor, TaskPatch) -> Result<TaskDetail, AppError>
// App::run_patch(actor, RunPatch) -> Result<Run, AppError>
// missing reason/summary => AppError::Validation { field, message: "is required" }
```

- [ ] **Step 1:** Add types + `empty_to_none` + App methods mirroring current HTTP/desktop sequence
- [ ] **Step 2:** `cargo test -p taskboard-application -- --test-threads=1`
- [ ] **Step 3:** Commit `feat(application): add task_patch and run_patch orchestration`

### Task 2: Wire HTTP and desktop

- [ ] **Step 1:** `patch_task` / `patch_run` / `task_update_inner` / `run_patch_inner` become thin adapters
- [ ] **Step 2:** Map surface `RunOp` → `RunPatchOp`; remove local `empty_to_none` copies used only for patch
- [ ] **Step 3:** `cargo test -p taskboard-api -p taskboard-desktop-commands -- --test-threads=1`
- [ ] **Step 4:** Commit + PR

## Self-review

Spec: single orchestration, RunOp once in application, empty_to_none shared, tests cover both surfaces — covered.
