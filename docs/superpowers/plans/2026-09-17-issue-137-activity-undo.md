# Activity, undo, and delete for comments and checks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Record comment/check writes as activity so Undo hits them, allow deleting a check or the latest comment, and show the missing Activity labels plus relative comment time.

**Architecture:** Add `EntityType::Comment` and `EntityType::Check`. `comment_add_inner` / `check_add_inner` / `check_toggle_inner` call `record_activity` (`comment.add`, `check.add`, `check.toggle`). Undo stays latest-one: create compensation deletes the row; toggle restores `before_json`. New `App::check_remove` and `App::comment_remove_latest`. Inspector delete on every check and only the last comment. Labels map the new operations plus `task.review` / `task.spawn` / `run.cancel`.

**Tech Stack:** Rust App/Store/CLI/HTTP/desktop, React Inspector, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- `run finish` still does not move the card
- Undo is still the latest one event only
- Comment delete is latest-only; no thread rewrite
- No markdown preview
- CLI `--json` snake_case
- Do not change review/spawn verbs beyond activity labels

User already chose sequential inline execution.

## File map

- Modify: `crates/core/src/models.rs` — `EntityType::{Comment,Check}`
- Modify: `crates/application/src/store.rs`, `app.rs`
- Modify: `crates/store-sqlite/src/store.rs` — get/delete comment and check
- Modify: `crates/application/tests/comments.rs`, `checks.rs`
- Modify: `crates/cli/src/args.rs`, `main.rs`, `output.rs`, `tests/cli_json.rs`, `tests/help.rs`
- Modify: `crates/api` routes + tests
- Modify: `crates/desktop-commands` + tauri
- Modify: `packages/types`, `client`, `ui` Inspector + fakeTransport

---

### Task 1: Record activity and undo comment/check

**Files:**
- Modify: `crates/core/src/models.rs`
- Modify: `crates/application/src/store.rs`, `app.rs`
- Modify: `crates/store-sqlite/src/store.rs`
- Modify: `crates/application/tests/comments.rs`, `checks.rs`
- Modify: `crates/cli/src/output.rs` `undo_human`

**Interfaces:**
- `EntityType { Project, Task, Run, Link, Comment, Check }`
- Operations: `comment.add`, `check.add`, `check.toggle`
- `Store::get_comment` / `delete_comment` / `get_check` / `delete_check`
- `check_add_inner(store, actor, cmd, now)` records activity
- Undo create → delete row; undo toggle → `update_check(before)`

- [x] **Step 1: Write failing tests** in `comments.rs` / `checks.rs`:

```rust
#[tokio::test]
async fn comment_add_records_activity_and_undo_deletes_it() {
    let app = seeded_task().await;
    app.comment_add(&cli_actor(), CommentAdd { task_display_id: "TASK-1".into(), body: "use TDD".into() }).await.unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert!(shown.recent_activities.iter().any(|a| a.operation == "comment.add"));
    app.undo(&cli_actor()).await.unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert!(shown.comments.is_empty());
}

#[tokio::test]
async fn check_add_and_toggle_record_activity_and_undo_toggle() {
    let app = seeded_task().await;
    app.check_add(&cli_actor(), CheckAdd { task_display_id: "TASK-1".into(), text: "Write tests".into() }).await.unwrap();
    app.check_toggle(&cli_actor(), "CHECK-1").await.unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert!(shown.recent_activities.iter().any(|a| a.operation == "check.toggle"));
    assert!(shown.checks[0].done);
    app.undo(&cli_actor()).await.unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert!(!shown.checks[0].done);
}
```

- [x] **Step 2: Run to fail** — `cargo test -p taskboard-application --test comments --test checks comment_add_records check_add_and_toggle`

- [x] **Step 3: Implement** EntityType variants; store get/delete; record_activity in the three inners (thread `actor` + `now` into check inners); exhaustive match arms treat Comment/Check like Link (sync parent task; `revision_just_undone` None; compensate_create deletes; restore_snapshot updates check; comment restore unused). `resolve_activity_target` uses parent task display id.

- [x] **Step 4: Tests pass**

- [x] **Step 5: Commit** `feat(core): record comment and check activity for undo`

---

### Task 2: check remove + latest comment remove

**Files:**
- Modify: application App + tests
- Modify: CLI args/main/help/cli_json
- Modify: HTTP DELETE routes + desktop inners
- Modify: types/client/fakeTransport/Inspector

**Interfaces:**
- `App::check_remove(actor, display_id) -> Result<Check, AppError>` records `check.remove` (before_json = check)
- `App::comment_remove_latest(actor, task_display_id) -> Result<Comment, AppError>` — empty thread → validation `comment`; records `comment.remove`
- Undo of remove restores via `before_json` + insert
- CLI: `tb check remove CHECK-n`, `tb comment remove TASK-n`
- HTTP: `DELETE /api/v1/checks/{id}`, `DELETE /api/v1/tasks/{id}/comments/latest`
- Inspector: Remove on each check; Remove only on last comment
- Comment list uses `relativeTime(createdAt)` not raw ISO

- [x] **Step 1: Failing tests** for App remove + CLI JSON + Inspector delete + relative time (existing ISO assertion in Inspector comments test must change)

- [x] **Step 2: Run to fail**

- [x] **Step 3: Implement** store insert already exists; remove + HTTP/desktop/CLI/UI

- [x] **Step 4: Tests pass**

- [x] **Step 5: Commit** `feat: delete checks and the latest comment`

---

### Task 3: Activity labels

**Files:**
- Modify: `packages/ui/src/Inspector.tsx` `ACTIVITY_LABELS`
- Modify: `packages/ui/src/Inspector.test.tsx` history test

```ts
  "comment.add": "Commented",
  "check.add": "Check added",
  "check.toggle": "Check toggled",
  "check.remove": "Check removed",
  "comment.remove": "Comment removed",
  "task.review": "Review",
  "task.spawn": "Spawned",
  "run.cancel": "Run canceled",
```

Record `task.review` at the end of `review_inner` (before/after task) and `task.spawn` at the end of `task_spawn_inner` (parent before/after) so the labels have operations. `run.cancel` already exists.

- [x] **Step 1–5:** failing inspector test for labels, implement, commit `feat(ui): activity labels for comment check review spawn`

---

## Self-review

1. Spec: activity + undo + check remove + latest comment delete + labels + relative time.
2. No placeholders.
3. Operations snake_case; HTTP camelCase unchanged.
