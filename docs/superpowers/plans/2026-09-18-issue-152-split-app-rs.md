# Split `app.rs` into domain modules (#152) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `crates/application/src/app.rs` (~3551 lines) into `app/{mod,helpers,projects,tasks,runs,checks,comments,board,undo}.rs` along existing `*_inner` boundaries, keeping `impl App` a thin façade with no behavior change.

**Architecture:** Convert `app.rs` into an `app/` directory. Shared free functions and types (`ActivityWrite`, `BlockIndex`, `record_activity`, `commit_or_rollback`, `sync_inner`, revision/json helpers) live in `helpers.rs` with `pub(super)` visibility. Domain `*_inner` functions move into matching modules and call helpers / peer inners via `super::`. `board.rs` owns inbox/status shaping plus `occupancy_inner`. `undo.rs` owns compensation. Public API (`App`, `Clock`, `SystemClock`) still re-exported from `crate::app`.

**Tech Stack:** Rust 2021, `taskboard-application`, existing `crates/application/tests/*`, `cargo test -p taskboard-application`, `cargo fmt`, `cargo clippy`.

## Global Constraints

- Behavior-preserving refactor only — no product feature changes
- Do not rename public `App` methods or change their signatures
- Four columns stay `todo | in-progress | in-review | done`
- `run finish` still does not move the card
- CLI `--json` snake_case unchanged
- First cuts must include at least `undo` and `board` (acceptance)
- Existing application tests must keep passing

User already chose sequential inline execution — do not ask Subagent-Driven vs Inline.

## File map

- Create: `crates/application/src/app/mod.rs` — `Clock`, `SystemClock`, `App`, thin `impl App`, `mod` declarations, `pub use` of nothing private
- Create: `crates/application/src/app/helpers.rs` — shared free fns + `ActivityWrite` / `BlockIndex` / `ResolvedActivity`
- Create: `crates/application/src/app/projects.rs` — `project_*_inner`
- Create: `crates/application/src/app/tasks.rs` — `task_*_inner`, `link_*_inner`, `review_inner`, `next_inner`, `write_task_note`
- Create: `crates/application/src/app/runs.rs` — `run_*_inner`, `mutate_run`, `require_run`, run parsers used only by runs
- Create: `crates/application/src/app/checks.rs` — `check_*_inner`
- Create: `crates/application/src/app/comments.rs` — `comment_*_inner`, `comment_add_and_continue_inner`
- Create: `crates/application/src/app/board.rs` — `occupancy_inner`, inbox/status builders, `count_group` / `*_line` / `is_stale`
- Create: `crates/application/src/app/undo.rs` — `undo_inner` through `compensate_project_reorder`
- Delete: `crates/application/src/app.rs` (replaced by `app/` directory)
- Unchanged: `crates/application/src/lib.rs` (`mod app;` still resolves)

## Module ownership (line anchors on pre-split `app.rs`)

| Module | Contents (pre-split anchors) |
| --- | --- |
| `mod.rs` | L1–L44 types; L45–L879 `impl App` (bodies that only lock + call inners stay; inbox/status bodies move to `board`) |
| `helpers` | `commit_or_rollback` L881–898; shared helpers L2446–2720 + L2762–2992 except board-only lines; `assign_unique_*` L3271–3315; `resolve_activity_target`…`child_task_id` L3317–3509; json/slug mappers L3511–3551; `sync_inner` L3424–3485 |
| `projects` | `project_*_inner` L900–1220 |
| `tasks` | L1221–1944 including `has_open_run` → move `has_open_run` to `helpers` (used by `runs`) |
| `runs` | `run_*_inner` + `mutate_run`/`require_run` (skip check/comment blocks) |
| `checks` | `check_*_inner` L2143–2247 |
| `comments` | `comment_add_inner`, `comment_remove_latest_inner`, `comment_add_and_continue_inner` |
| `board` | `occupancy_inner` L2721–2760; `count_group`/`inbox_line`/`task_line`/`run_line`/`is_stale`; extracted `inbox_items` + `board_status` used by façade |
| `undo` | `undo_inner`…`compensate_project_reorder` L2994–3269 |

**Visibility rule:** every free `fn` / `async fn` / private `struct` used outside its file becomes `pub(super)`.

**Import rule:** each submodule starts with the same `use` set it needs from `taskboard_core`, `crate::actor`, `crate::commands`, `crate::error`, `crate::store`, plus `use super::helpers::{...}` and peer `use super::{projects,tasks,...}` only for cross-domain inners actually called.

---

### Task 1: Baseline green + module skeleton with `helpers`

**Files:**
- Create: `crates/application/src/app/mod.rs`, `helpers.rs`
- Delete after move: none yet until later tasks finish
- Test: existing `crates/application/tests/*`

**Interfaces:**
- Consumes: current monolithic `app.rs`
- Produces: `pub struct App`, `pub trait Clock`, `pub struct SystemClock`; `pub(super)` helpers listed below

- [ ] **Step 1: Record baseline**

Run: `cargo test -p taskboard-application -- --test-threads=1`
Expected: all tests PASS (note count for later comparison).

- [ ] **Step 2: Create `app/` directory and move façade + helpers first**

```bash
mkdir -p crates/application/src/app
# Mechanically: copy app.rs → app/mod.rs, then carve helpers.rs
```

In `helpers.rs`, move (verbatim logic) and mark `pub(super)`:

- `commit_or_rollback`
- `require_non_blank`, `parse_session_id`, `parse_run_message`
- `require_live_task`, `task_not_found`, `task_keys`, `insert_into_urgency_group`, `position_of`, `ids_from_keys`
- `rewrite_column`, `rewrite_live_column`, `load_task_detail`
- `winning_run`, `to_task_summary_from_runs`, `to_task_summary`
- `with_task_workspace`, `optional_workspace_value`, `reply_for`
- `BlockIndex`, `load_block_index`, `block_lists`, `has_active_blocker`, `would_cycle`, `parse_blocked_by`
- `display_from_runs`, `map_order_error`
- `require_live_project`, `project_not_found`, `check_revision`
- `ActivityWrite`, `record_activity`
- `assign_unique_task_position`, `assign_unique_project_sort`
- `ResolvedActivity`, `resolve_activity_target`, `current_entity_json`, `child_task_id`
- `uuid_from_json`, `json_value`, `slugify_field`, `map_validation`, `map_slug_error`, `map_json`
- `sync_inner`
- `has_open_run` (moved out of tasks region)

`mod.rs` keeps `impl App` and temporarily keeps remaining domain inners in the same file until later tasks (or move them in the same PR in subsequent tasks before green).

- [ ] **Step 3: Wire modules**

```rust
// crates/application/src/app/mod.rs (top)
mod board;
mod checks;
mod comments;
mod helpers;
mod projects;
mod runs;
mod tasks;
mod undo;

use helpers::*; // only if still calling bare names from façade; prefer path-qualified calls
```

Until other modules exist, either create empty stub modules that `include!` nothing useful, or complete Tasks 2–5 in one branch before expecting compile — preferred: extract helpers + leave other inners in `mod.rs` first so compile stays green after Task 1.

Minimal Task-1 end state that compiles:

```rust
mod helpers;
// domain inners still in mod.rs, calling helpers::foo or `use helpers::*`
```

- [ ] **Step 4: Re-run application tests**

Run: `cargo test -p taskboard-application -- --test-threads=1`
Expected: PASS, same suite green.

- [ ] **Step 5: Commit**

```bash
git add crates/application/src/app docs/superpowers/plans/2026-09-18-issue-152-split-app-rs.md
git commit -m "refactor(application): extract app helpers module from app.rs"
```

---

### Task 2: Extract `undo` and `board` (acceptance first cuts)

**Files:**
- Create: `crates/application/src/app/undo.rs`, `board.rs`
- Modify: `crates/application/src/app/mod.rs`

**Interfaces:**
- Consumes: `helpers::{record_activity, ActivityWrite, json_value, rewrite_live_column, assign_unique_*, current_entity_json, ...}`
- Produces:
  - `pub(super) async fn undo_inner(store, actor, now) -> Result<UndoResult, AppError>`
  - `pub(super) async fn occupancy_inner(store, query) -> Result<Vec<OccupancyGroup>, AppError>`
  - `pub(super) fn is_stale(run, now, minutes) -> bool`
  - `pub(super) fn inbox_from_summaries(summaries, now, stale_after_minutes) -> Vec<InboxItem>`
  - `pub(super) fn board_status_from_parts(...) -> BoardStatus` (or keep status assembly in façade calling small board helpers)

- [ ] **Step 1: Write a structural smoke test (fails until modules exist)**

Add to a small unit test file or `crates/application/src/app/mod.rs`:

```rust
#[cfg(test)]
mod split_smoke {
    #[test]
    fn domain_modules_are_linked() {
        // Touch module paths so a missing file fails compile of this crate's tests.
        let _ = std::any::type_name::<super::App>();
        assert!(true);
    }
}
```

After `board`/`undo` exist, extend smoke to call a pure helper:

```rust
#[test]
fn board_is_stale_helper_usable() {
    use chrono::Utc;
    use taskboard_core::{Run, RunStatus};
    use uuid::Uuid;
    // Construct minimal Run with Running + old updated_at, or call is_stale with fixture from tests.
    // Prefer: assert !super::board::is_stale(&fresh_running_run(), Utc::now(), 30);
}
```

If constructing a full `Run` is heavy, skip the `is_stale` unit and rely on existing `tests/stale.rs` + `tests/inbox.rs` + `tests/status.rs` + undo tests as the behavior gate (TDD-lite for pure moves).

- [ ] **Step 2: Move undo block verbatim into `undo.rs`**

```rust
// crates/application/src/app/undo.rs
use super::helpers::{
    assign_unique_project_sort, assign_unique_task_position, current_entity_json, json_value,
    record_activity, rewrite_live_column, ActivityWrite,
};
// ... other imports matching original ...

pub(super) async fn undo_inner(...) -> Result<UndoResult, AppError> { /* verbatim */ }
pub(super) async fn apply_compensation(...) -> Result<serde_json::Value, AppError> { /* ... */ }
// revision_just_undone, compensate_create, restore_snapshot, compensate_project_reorder
```

Façade:

```rust
let result = undo::undo_inner(store, actor, now).await;
```

- [ ] **Step 3: Move board helpers + `occupancy_inner`; thin inbox/status**

Extract from current `App::inbox` / `App::status` bodies into `board.rs`:

```rust
pub(super) fn collect_inbox_items(
    summaries: Vec<TaskSummary>,
    now: DateTime<Utc>,
    stale_after_minutes: i64,
) -> Vec<InboxItem> {
    // body formerly inside App::inbox after task_query
}

pub(super) fn build_board_status(
    inbox: Vec<InboxItem>,
    open: Vec<TaskSummary>,
    in_review: Vec<TaskSummary>,
    ready: Vec<TaskSummary>,
    open_runs: Vec<Run>,
    tasks_by_id: HashMap<Uuid, &TaskSummary>,
) -> BoardStatus {
    // body formerly assembling InboxCounts + heads
}
```

`App::inbox` becomes:

```rust
pub async fn inbox(&self, scope: InboxScope) -> Result<Vec<InboxItem>, AppError> {
    let now = self.clock.now();
    let summaries = self
        .task_query(TaskListQuery { /* same fields as today */ })
        .await?;
    Ok(board::collect_inbox_items(
        summaries,
        now,
        scope.stale_after_minutes,
    ))
}
```

`App::occupancy` calls `board::occupancy_inner`. `App::stale_list` calls `board::is_stale`.

- [ ] **Step 4: Run focused tests**

Run:

```bash
cargo test -p taskboard-application --test inbox --test status --test occupancy --test stale --test notes_runs -- --test-threads=1
```

Expected: PASS (undo covered heavily in `notes_runs` / activity paths — also run full suite).

- [ ] **Step 5: Commit**

```bash
git commit -m "refactor(application): extract app board and undo modules"
```

---

### Task 3: Extract `projects`, `tasks`, `runs`, `checks`, `comments`

**Files:**
- Create: `projects.rs`, `tasks.rs`, `runs.rs`, `checks.rs`, `comments.rs`
- Modify: `mod.rs` — only façade methods left calling `projects::…`, `tasks::…`, etc.

**Interfaces:**
- `projects::project_add_inner(...) -> Result<Project, AppError>` (and siblings)
- `tasks::task_create_inner`, `task_spawn_inner`, `review_inner`, `next_inner`, `link_*_inner`, …
- `runs::run_start_inner(..., exclusive: bool)`, `run_update_inner`, `run_wait_inner`, `run_continue_inner`, `run_continue_with_reply_inner`, `run_fail_inner`, `run_cancel_inner`, `run_finish_inner`, `mutate_run`, `require_run`
- `checks::check_add_inner` / `check_toggle_inner` / `check_remove_inner`
- `comments::comment_add_inner` / `comment_remove_latest_inner` / `comment_add_and_continue_inner`
- Cross calls (must compile):
  - `tasks::next_inner` → `runs::run_start_inner` + `tasks::task_move_inner`
  - `tasks::review_inner` → `comments::comment_add_inner`
  - `tasks::task_spawn_inner` → `tasks::task_create_inner` + `tasks::link_add_inner`
  - `comments::comment_add_and_continue_inner` → `runs::run_continue_inner`
  - `runs::run_cancel_inner` → `runs::run_fail_inner`

- [ ] **Step 1: Move each domain file; update call sites in `mod.rs`**

Example façade wiring:

```rust
let result = projects::project_add_inner(store, actor, cmd, now).await;
// ...
let result = tasks::task_create_inner(store, actor, cmd, now).await;
let result = runs::run_start_inner(store, actor, cmd, now, false).await;
let result = checks::check_add_inner(store, actor, cmd, now).await;
let result = comments::comment_add_inner(store, actor, cmd, now).await;
```

Inside `tasks.rs`, qualify peer calls:

```rust
let run = super::runs::run_start_inner(store, actor, start_cmd, now, cmd.exclusive).await?;
```

Inside `comments.rs`:

```rust
super::runs::run_continue_inner(store, actor, continue_cmd, now).await?;
```

- [ ] **Step 2: Confirm `mod.rs` has no remaining `*_inner` definitions**

Run: `rg '_inner' crates/application/src/app/mod.rs`
Expected: call sites only, no `async fn .*_inner` definitions.

- [ ] **Step 3: Full application + fmt/clippy gate**

```bash
cargo fmt --all
cargo clippy -p taskboard-application --all-targets -- -D warnings
cargo test -p taskboard-application -- --test-threads=1
```

Expected: fmt clean, clippy clean, all tests PASS.

- [ ] **Step 4: Commit**

```bash
git commit -m "refactor(application): split app domain inners into modules"
```

---

### Task 4: Acceptance checklist + PR polish

**Files:**
- Modify: plan checkboxes; optional tiny doc note only if repo already documents module layout (do not add unsolicited markdown)

- [ ] **Step 1: Verify acceptance**

- [ ] `app.rs` gone; `app/mod.rs` is thin façade; domain logic in `app/*.rs`
- [ ] `undo` and `board` modules present
- [ ] `cargo test -p taskboard-application -- --test-threads=1` green
- [ ] `cargo fmt --all -- --check` and clippy `-D warnings` green for touched crate / workspace as CI does

- [ ] **Step 2: Open PR against `main`**

Title: `refactor(application): split app.rs into domain modules (#152)`

Body: link issue, summarize module map, note behavior-preserving.

- [ ] **Step 3: Wait for CI green, merge, delete remote branch**

---

## Self-review

1. **Spec coverage:** Issue wants split along `*_inner` into listed modules, thin façade, undo+board first cuts, tests pass, no behavior change — Tasks 1–4 cover all.
2. **Placeholders:** No TBD steps; moves are anchored to line ranges and function names.
3. **Type consistency:** `ActivityWrite` / `BlockIndex` / `ResolvedActivity` live in `helpers`; undo/board/tasks import them; public `App` API unchanged.
