# Usability Mechanisms Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the approved usability layer: named create, Attention Inbox (`App` + `tb inbox` + UI strip), inspector/sidebar spec debt, toasts, and thin `tb project detect`.

**Architecture:** Inbox membership and sort stay pure functions in `taskboard-core`. `App::inbox` filters live task lists in process. CLI/HTTP/desktop call that use case. Shared React UI in `packages/ui` adds composers, toasts, Trash, links, and the Inbox strip on top of the existing Transport. `ui.toml` stores last project slug; no SQLite migration.

**Tech Stack:** Existing Rust crates (core, application, store-sqlite, cli, api, desktop-commands), clap, Axum, React, Vitest, Testing Library. Spec: `docs/superpowers/specs/2026-09-06-usability-mechanisms-design.md`.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Titles are never task identifiers; mutations still require `TASK-n` / `RUN-n`
- Inbox is a derived query, not a stored column or table
- No OS notifications, command palette, `tb work start`, markdown preview, or fifth column
- UI copy is English
- CLI `--json` is snake_case; HTTP JSON is camelCase
- `h`/`l` still move a selected card; `Shift+h`/`Shift+l` jump column only
- UI and `n`/`p` never create an `Untitled` entity
- Native Glass tokens: `--waiting` amber, `--urgent` orange-red, `--failed` existing red-muted, `--completed` green
- Specs: `docs/superpowers/specs/2026-09-06-usability-mechanisms-design.md` (this layer) and the 2026-09-05 product specs (unchanged domain)

## File map

- Create: `crates/core/src/inbox.rs`
- Create: `crates/application/tests/inbox.rs`
- Create: `crates/store-sqlite/src/ui_state.rs`
- Create: `packages/ui/src/Composer.tsx`, `packages/ui/src/Composer.module.css`
- Create: `packages/ui/src/ShortcutLegend.tsx`, `packages/ui/src/ShortcutLegend.module.css`
- Create: `packages/ui/src/Toast.tsx`, `packages/ui/src/Toast.module.css`
- Create: `packages/ui/src/InboxStrip.tsx`, `packages/ui/src/InboxStrip.module.css`
- Create: `packages/ui/src/TrashPanel.tsx`, `packages/ui/src/TrashPanel.module.css`
- Create: `packages/ui/src/ConfirmDialog.tsx`
- Modify: `crates/core/src/{lib.rs,models.rs,display_status.rs}`
- Modify: `crates/application/src/{lib.rs,app.rs,error.rs,commands.rs}`
- Modify: `crates/cli/src/{args.rs,main.rs,output.rs}`
- Modify: `crates/api/src/{routes.rs,dto.rs}`
- Modify: `crates/desktop-commands/src/{commands.rs,dto.rs,lib.rs}`
- Modify: `apps/desktop/src-tauri/src/{lib.rs,commands.rs}`
- Modify: `packages/types/src/index.ts` (hand-maintained; there is no `pnpm generate`)
- Modify: `packages/client/src/{transport.ts,http.ts,tauri.ts}`
- Modify: `packages/ui/src/{TaskboardApp.tsx,Board.tsx,Card.tsx,Inspector.tsx,Sidebar.tsx,summary.ts,fakeTransport.ts,index.ts}`
- Test: `crates/core` unit tests in `inbox.rs` / `display_status.rs`
- Test: `crates/application/tests/inbox.rs`
- Test: `crates/cli/tests/cli_json.rs`, `crates/api/tests/routes.rs`
- Test: `packages/ui/src/{keyboard.test.tsx,Board.test.tsx,Card.test.tsx}` plus new UI test files per task
- Test: `packages/client/src/http.test.ts`

---

### Task 1: TaskSummary.waiting_reason and inbox membership rules

**Files:**
- Create: `crates/core/src/inbox.rs`
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/core/src/models.rs` (`TaskSummary`, `TaskDetail`)
- Modify: `crates/application/src/app.rs` (`to_task_summary`, `load_task_detail`, `display_from_runs`)
- Modify: `crates/api/src/dto.rs` (`TaskSummaryDto`, `TaskDetailDto`)
- Modify: `crates/desktop-commands/src/dto.rs` (same DTO fields)
- Modify: `packages/types/src/index.ts`
- Modify: `packages/ui/src/summary.ts`
- Modify: `packages/ui/src/fakeTransport.ts`

**Interfaces:**
- Consumes: `CardDisplayStatus`, `Column`, `Run`, `Task`
- Produces:
  - `TaskSummary.waiting_reason: Option<String>` (serde `waiting_reason` / TS `waitingReason`)
  - `TaskDetail.waiting_reason: Option<String>` (same)
  - `pub fn inbox_membership(column: Column, urgent: bool, status: CardDisplayStatus) -> Option<InboxGroup>`
  - `pub enum InboxGroup { Waiting, Failed, Urgent }` with sort rank Waiting=0, Failed=1, Urgent=2
  - `pub fn inbox_reason(waiting_reason: Option<&str>, run_message: Option<&str>) -> String`
  - `display_from_runs` returns `(status, run_message, waiting_reason)` from the same winning run as today; `waiting_reason` is that run's `waiting_reason`

- [ ] **Step 1: Write the failing core tests**

Create `crates/core/src/inbox.rs`:

```rust
use crate::column::Column;
use crate::display_status::CardDisplayStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InboxGroup {
    Waiting = 0,
    Failed = 1,
    Urgent = 2,
}

pub fn inbox_membership(
    column: Column,
    urgent: bool,
    status: CardDisplayStatus,
) -> Option<InboxGroup> {
    unimplemented!()
}

pub fn inbox_reason(waiting_reason: Option<&str>, run_message: Option<&str>) -> String {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_is_inbox_even_in_done() {
        assert_eq!(
            inbox_membership(Column::Done, false, CardDisplayStatus::Waiting),
            Some(InboxGroup::Waiting)
        );
    }

    #[test]
    fn failed_is_inbox() {
        assert_eq!(
            inbox_membership(Column::InProgress, false, CardDisplayStatus::Failed),
            Some(InboxGroup::Failed)
        );
    }

    #[test]
    fn urgent_todo_is_inbox() {
        assert_eq!(
            inbox_membership(Column::Todo, true, CardDisplayStatus::Idle),
            Some(InboxGroup::Urgent)
        );
    }

    #[test]
    fn urgent_done_without_waiting_or_failed_is_out() {
        assert_eq!(
            inbox_membership(Column::Done, true, CardDisplayStatus::Completed),
            None
        );
        assert_eq!(
            inbox_membership(Column::Done, true, CardDisplayStatus::Idle),
            None
        );
    }

    #[test]
    fn idle_non_urgent_is_out() {
        assert_eq!(
            inbox_membership(Column::Todo, false, CardDisplayStatus::Idle),
            None
        );
    }

    #[test]
    fn completed_in_review_is_out_unless_urgent() {
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Completed),
            None
        );
        assert_eq!(
            inbox_membership(Column::InReview, true, CardDisplayStatus::Completed),
            Some(InboxGroup::Urgent)
        );
    }

    #[test]
    fn running_is_out_unless_urgent() {
        assert_eq!(
            inbox_membership(Column::InProgress, false, CardDisplayStatus::Running),
            None
        );
    }

    #[test]
    fn waiting_outranks_urgent_flag() {
        assert_eq!(
            inbox_membership(Column::Todo, true, CardDisplayStatus::Waiting),
            Some(InboxGroup::Waiting)
        );
    }

    #[test]
    fn reason_prefers_waiting_reason_then_run_message() {
        assert_eq!(
            inbox_reason(Some("Need spec"), Some("still going")),
            "Need spec"
        );
        assert_eq!(inbox_reason(None, Some("still going")), "still going");
        assert_eq!(inbox_reason(None, None), "");
        assert_eq!(inbox_reason(Some(""), Some("msg")), "msg");
    }
}
```

Export it from `crates/core/src/lib.rs`: `mod inbox; pub use inbox::{inbox_membership, inbox_reason, InboxGroup};`

Add `waiting_reason: Option<String>` to `TaskSummary` and `TaskDetail` in `crates/core/src/models.rs` (after `run_message`). Do not implement `inbox_membership` yet (leave `unimplemented!()`).

- [ ] **Step 2: Run the core tests to verify they fail**

Run: `cargo test -p taskboard-core inbox::`

Expected: FAIL (`unimplemented` panic) or compile errors wherever `TaskSummary { ... }` is constructed without `waiting_reason`.

- [ ] **Step 3: Implement membership and thread `waiting_reason`**

```rust
pub fn inbox_membership(
    column: Column,
    urgent: bool,
    status: CardDisplayStatus,
) -> Option<InboxGroup> {
    match status {
        CardDisplayStatus::Waiting => Some(InboxGroup::Waiting),
        CardDisplayStatus::Failed => Some(InboxGroup::Failed),
        _ if urgent && column != Column::Done => Some(InboxGroup::Urgent),
        _ => None,
    }
}

pub fn inbox_reason(waiting_reason: Option<&str>, run_message: Option<&str>) -> String {
    let waiting = waiting_reason.map(str::trim).filter(|s| !s.is_empty());
    let message = run_message.map(str::trim).filter(|s| !s.is_empty());
    waiting.or(message).unwrap_or("").to_string()
}
```

Change `display_from_runs` in `crates/application/src/app.rs` to return the winning run's `message` and `waiting_reason`:

```rust
fn display_from_runs(runs: &[Run]) -> (CardDisplayStatus, Option<String>, Option<String>) {
    // keep existing status + run_message selection
    // additionally clone winning_run.waiting_reason
}
```

Fill `waiting_reason` in `to_task_summary` and `load_task_detail`. Add the field to `TaskSummaryDto` / `TaskDetailDto` in both `crates/api/src/dto.rs` and `crates/desktop-commands/src/dto.rs`. Add `waitingReason: string | null` to `TaskSummary` in `packages/types/src/index.ts`. Set `waitingReason: null` in `packages/ui/src/summary.ts` and `fakeTransport` `taskCreate`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-core inbox::` and `cargo test -p taskboard-application -- --test-threads=1` and `pnpm --filter @taskboard/ui test`

Expected: PASS. Existing application tests keep passing because `waiting_reason` is populated from runs.

- [ ] **Step 5: Commit**

```bash
git add crates/core crates/application/src/app.rs crates/api/src/dto.rs crates/desktop-commands/src/dto.rs packages/types/src/index.ts packages/ui/src/summary.ts packages/ui/src/fakeTransport.ts
git commit -m "feat: add waiting_reason and inbox membership rules"
```

---

### Task 2: App::inbox

**Files:**
- Modify: `crates/application/src/commands.rs`
- Modify: `crates/application/src/app.rs`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/core/src/models.rs` (add `InboxItem`)
- Modify: `crates/core/src/lib.rs`
- Test: `crates/application/tests/inbox.rs`

**Interfaces:**
- Consumes: `inbox_membership`, `inbox_reason`, `App::task_list`, `App::project_list`, `App::project_show`
- Produces:
  - `pub struct InboxScope { pub project: Option<String>, pub include_archived: bool }`
  - `pub struct InboxItem` with fields: `id, display_id, project_id, project_slug, project_name, title, column, urgent, revision, display_status, run_message, waiting_reason, reason, updated_at` (serde snake_case)
  - `pub async fn App::inbox(&self, scope: InboxScope) -> Result<Vec<InboxItem>, AppError>`
  - Sort: `InboxGroup` ascending, then urgent `true` before `false`, then `updated_at` descending, then `display_id` ascending
  - Unknown `--project` slug is `not_found` on entity `project`

`InboxItem.updated_at` comes from the live `Task.updated_at` (load via `task_list` is not enough). Implementation: for each in-scope live project, `store.list_tasks(project_id)`, skip `deleted_at.is_some()`, `to_task_summary`, keep rows where `inbox_membership` is `Some`, attach project slug/name and `updated_at` from the `Task` row.

- [ ] **Step 1: Write the failing application tests**

Create `crates/application/tests/inbox.rs` using the same `TestApp` / `cli_actor` / `test_app` pattern as `crates/application/tests/notes_runs.rs`.

```rust
#[tokio::test]
async fn inbox_includes_waiting_failed_and_urgent_not_done() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(&actor, ProjectAdd { name: "Renai Sim".into(), repo_path: None, slug: None }).await.unwrap();
    app.task_create(&actor, TaskCreate { project_slug: "renai-sim".into(), title: "Wait me".into(), column: None, urgent: false }).await.unwrap();
    app.task_create(&actor, TaskCreate { project_slug: "renai-sim".into(), title: "Fail me".into(), column: None, urgent: false }).await.unwrap();
    app.task_create(&actor, TaskCreate { project_slug: "renai-sim".into(), title: "Pin me".into(), column: None, urgent: true }).await.unwrap();
    app.task_create(&actor, TaskCreate { project_slug: "renai-sim".into(), title: "Idle".into(), column: None, urgent: false }).await.unwrap();
    let wait = app.run_start(&actor, RunStart { task_display_id: "TASK-1".into(), agent: "codex".into(), session_id: None }).await.unwrap();
    app.run_wait(&actor, RunWait { run_display_id: wait.display_id, reason: "Need spec".into(), revision: None }).await.unwrap();
    let fail = app.run_start(&actor, RunStart { task_display_id: "TASK-2".into(), agent: "codex".into(), session_id: None }).await.unwrap();
    app.run_fail(&actor, RunFail { run_display_id: fail.display_id, summary: "boom".into(), revision: None }).await.unwrap();

    let items = app.inbox(InboxScope { project: None, include_archived: false }).await.unwrap();
    let titles: Vec<_> = items.iter().map(|i| i.title.as_str()).collect();
    assert_eq!(titles, vec!["Wait me", "Fail me", "Pin me"]);
    assert_eq!(items[0].reason, "Need spec");
    assert_eq!(items[0].project_slug, "renai-sim");
}

#[tokio::test]
async fn inbox_drops_urgent_done_and_archived_by_default() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(&actor, ProjectAdd { name: "A".into(), repo_path: None, slug: None }).await.unwrap();
    app.task_create(&actor, TaskCreate { project_slug: "a".into(), title: "Done pin".into(), column: Some(Column::Done), urgent: true }).await.unwrap();
    app.project_add(&actor, ProjectAdd { name: "B".into(), repo_path: None, slug: None }).await.unwrap();
    app.task_create(&actor, TaskCreate { project_slug: "b".into(), title: "Archived wait".into(), column: None, urgent: false }).await.unwrap();
    let run = app.run_start(&actor, RunStart { task_display_id: "TASK-2".into(), agent: "codex".into(), session_id: None }).await.unwrap();
    app.run_wait(&actor, RunWait { run_display_id: run.display_id, reason: "x".into(), revision: None }).await.unwrap();
    app.project_archive(&actor, "b", true, None).await.unwrap();

    let live = app.inbox(InboxScope { project: None, include_archived: false }).await.unwrap();
    assert!(live.is_empty());
    let with_arch = app.inbox(InboxScope { project: None, include_archived: true }).await.unwrap();
    assert_eq!(with_arch.iter().map(|i| i.title.as_str()).collect::<Vec<_>>(), vec!["Archived wait"]);
}

#[tokio::test]
async fn inbox_project_scope_and_sort() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(&actor, ProjectAdd { name: "A".into(), repo_path: None, slug: None }).await.unwrap();
    app.project_add(&actor, ProjectAdd { name: "B".into(), repo_path: None, slug: None }).await.unwrap();
    app.task_create(&actor, TaskCreate { project_slug: "a".into(), title: "A wait".into(), column: None, urgent: false }).await.unwrap();
    app.task_create(&actor, TaskCreate { project_slug: "b".into(), title: "B wait".into(), column: None, urgent: false }).await.unwrap();
    for id in ["TASK-1", "TASK-2"] {
        let run = app.run_start(&actor, RunStart { task_display_id: id.into(), agent: "codex".into(), session_id: None }).await.unwrap();
        app.run_wait(&actor, RunWait { run_display_id: run.display_id, reason: id.into(), revision: None }).await.unwrap();
    }
    let only_a = app.inbox(InboxScope { project: Some("a".into()), include_archived: false }).await.unwrap();
    assert_eq!(only_a.len(), 1);
    assert_eq!(only_a[0].title, "A wait");
}

#[tokio::test]
async fn inbox_unknown_project_is_not_found() {
    let app = test_app().await;
    let err = app.inbox(InboxScope { project: Some("nope".into()), include_archived: false }).await.unwrap_err();
    assert_eq!(err.code(), "not_found");
}
```

Copy `TestApp` / `test_app` / `cli_actor` helpers into this file (do not import them from another integration test). Import `InboxScope` from `taskboard_application`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p taskboard-application --test inbox -- --test-threads=1`

Expected: FAIL compile (`InboxScope` / `App::inbox` missing).

- [ ] **Step 3: Implement `InboxItem`, `InboxScope`, and `App::inbox`**

Add to `crates/core/src/models.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InboxItem {
    pub id: Uuid,
    pub display_id: String,
    pub project_id: Uuid,
    pub project_slug: String,
    pub project_name: String,
    pub title: String,
    pub column: Column,
    pub urgent: bool,
    pub revision: i64,
    pub display_status: CardDisplayStatus,
    pub run_message: Option<String>,
    pub waiting_reason: Option<String>,
    pub reason: String,
    pub updated_at: DateTime<Utc>,
}
```

Add `InboxScope` to `crates/application/src/commands.rs` and re-export it from `lib.rs`. Implement `App::inbox` in `app.rs`: lock store, list projects (`include_archived` as given; always skip `deleted_at`), optional slug filter via `require_live_project` when `project` is `Some` (archived allowed only if `include_archived`), list tasks per project, map through `to_task_summary`, keep membership hits, sort as specified, return.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-application --test inbox -- --test-threads=1`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/models.rs crates/core/src/lib.rs crates/application
git commit -m "feat: add App::inbox derived attention query"
```

---

### Task 3: CLI `tb inbox`

**Files:**
- Modify: `crates/cli/src/args.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/src/output.rs`
- Test: `crates/cli/tests/cli_json.rs`
- Test: `crates/cli/tests/help.rs` (assert `--help` contains `inbox`)

**Interfaces:**
- Consumes: `App::inbox`, `InboxScope`
- Produces:
  - `Command::Inbox { project: Option<String>, archived: bool }`
  - `tb inbox [--project SLUG] [--archived] [--json]`
  - JSON `{ "ok": true, "entities": [ InboxItem, ... ] }`
  - Human headerless rows: `ID  PROJECT  COLUMN  URGENT  RUN  TITLE  REASON` using existing urgent `U`/`-` and run status words
  - Empty human: `inbox is empty` plus newline
  - Do not add a `--all` flag

- [ ] **Step 1: Write the failing CLI tests**

Append to `crates/cli/tests/cli_json.rs`:

```rust
#[test]
fn inbox_json_lists_waiting_across_projects() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir).args(["project", "add", "--name", "A"]).assert().success();
    tb_in(&dir).args(["project", "add", "--name", "B"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "a", "--title", "Wait A"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "b", "--title", "Wait B"]).assert().success();
    tb_in(&dir).args(["run", "start", "TASK-1", "--agent", "codex"]).assert().success();
    tb_in(&dir).args(["run", "wait", "RUN-1", "--reason", "Need spec"]).assert().success();
    tb_in(&dir).args(["run", "start", "TASK-2", "--agent", "codex"]).assert().success();
    tb_in(&dir).args(["run", "wait", "RUN-2", "--reason", "Need B"]).assert().success();
    let out = tb_in(&dir).args(["inbox", "--json"]).assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["entities"].as_array().unwrap().len(), 2);
    assert_eq!(v["entities"][0]["reason"], "Need spec");
    assert_eq!(v["entities"][0]["project_slug"], "a");
}

#[test]
fn inbox_empty_human() {
    let (mut cmd, _dir) = tb();
    cmd.args(["inbox"]).assert().success().stdout(predicate::str::contains("inbox is empty"));
}

#[test]
fn inbox_project_filter_json() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir).args(["project", "add", "--name", "A"]).assert().success();
    tb_in(&dir).args(["project", "add", "--name", "B"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "a", "--title", "Pin A", "--urgent"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "b", "--title", "Pin B", "--urgent"]).assert().success();
    let out = tb_in(&dir).args(["inbox", "--project", "b", "--json"]).assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["entities"].as_array().unwrap().len(), 1);
    assert_eq!(v["entities"][0]["title"], "Pin B");
}
```

In `crates/cli/tests/help.rs`, also `assert!(out.contains("inbox"));`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-cli --test cli_json inbox -- --nocapture`

Expected: FAIL (`unexpected argument 'inbox'`).

- [ ] **Step 3: Implement the subcommand**

In `args.rs` add `Inbox { #[arg(long)] project: Option<String>, #[arg(long)] archived: bool }` to `Command`. In `dispatch`, call `app.inbox(InboxScope { project, include_archived: archived })`. In `output.rs`:

```rust
pub fn print_inbox(items: &[InboxItem]) {
    if items.is_empty() {
        println!("inbox is empty");
        return;
    }
    println!("ID  PROJECT  COLUMN  URGENT  RUN  TITLE  REASON");
    for item in items {
        let urgent = if item.urgent { "U" } else { "-" };
        println!(
            "{}  {}  {}  {}  {}  {}  {}",
            item.display_id,
            item.project_slug,
            item.column.as_str(),
            urgent,
            run_badge(item.display_status),
            item.title,
            item.reason
        );
    }
}
```

Reuse `print_task_list`'s run-status formatting. Look at `crates/cli/src/output.rs` `print_task_list` and copy its `URGENT` / `RUN` cell format rather than `Debug`. JSON goes through existing `print_entities`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-cli --test cli_json inbox` and `cargo test -p taskboard-cli --test help`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cli
git commit -m "feat: add tb inbox for waiting failed and urgent cards"
```

---

### Task 4: HTTP, Transport, and desktop inbox

**Files:**
- Modify: `crates/api/src/dto.rs`
- Modify: `crates/api/src/routes.rs`
- Modify: `crates/desktop-commands/src/dto.rs`
- Modify: `crates/desktop-commands/src/commands.rs`
- Modify: `crates/desktop-commands/src/lib.rs`
- Modify: `apps/desktop/src-tauri/src/commands.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `packages/types/src/index.ts`
- Modify: `packages/client/src/transport.ts`
- Modify: `packages/client/src/http.ts`
- Modify: `packages/client/src/tauri.ts`
- Modify: `packages/client/src/http.test.ts`
- Modify: `packages/ui/src/fakeTransport.ts`
- Test: `crates/api/tests/routes.rs`
- Test: `crates/desktop-commands/tests/commands.rs` (add inbox call if that crate tests commands directly)

**Interfaces:**
- Consumes: `App::inbox`
- Produces:
  - `GET /api/v1/inbox?project={slug}&archived=0|1` (omit `project` for all live projects; `archived` default `0`)
  - Response `{ "ok": true, "entities": [ InboxItemDto camelCase ] }`
  - `Transport.inbox(opts?: { project?: string; includeArchived?: boolean }): Promise<InboxItem[]>`
  - Tauri command `inbox` with args `{ project: Option<String>, include_archived: bool }`
  - TS:

```ts
export interface InboxItem {
  id: string;
  displayId: string;
  projectId: string;
  projectSlug: string;
  projectName: string;
  title: string;
  column: Column;
  urgent: boolean;
  revision: number;
  displayStatus: DisplayStatus;
  runMessage: string | null;
  waitingReason: string | null;
  reason: string;
  updatedAt: string;
}
```

- [ ] **Step 1: Write the failing HTTP and client tests**

In `crates/api/tests/routes.rs`:

```rust
#[tokio::test]
async fn inbox_returns_waiting_card() {
    let s = seeded_task_server().await;
    let client = authed(&s);
    client
        .post(format!("{}/api/v1/tasks/TASK-1/runs", s.base))
        .json(&json!({"agent": "codex"}))
        .send()
        .await
        .unwrap();
    client
        .patch(format!("{}/api/v1/runs/RUN-1", s.base))
        .json(&json!({"op": "wait", "reason": "Need spec"}))
        .send()
        .await
        .unwrap();
    let res = client
        .get(format!("{}/api/v1/inbox", s.base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let v: serde_json::Value = res.json().await.unwrap();
    assert_eq!(v["entities"][0]["displayId"], "TASK-1");
    assert_eq!(v["entities"][0]["reason"], "Need spec");
    assert_eq!(v["entities"][0]["projectSlug"], "renai-sim");
}
```

In `packages/client/src/http.test.ts`:

```ts
it("inbox sends project and archived query", async () => {
  const fetches: Request[] = [];
  const fetchImpl: typeof fetch = async (input, init) => {
    fetches.push(new Request(input, init));
    return new Response(JSON.stringify({ ok: true, entities: [] }), {
      headers: { "Content-Type": "application/json" },
    });
  };
  const t = new HttpTransport("http://127.0.0.1:9", "deadbeef", fetchImpl);
  await t.inbox({ project: "renai-sim", includeArchived: true });
  const url = new URL(fetches[0].url);
  assert.equal(url.pathname, "/api/v1/inbox");
  assert.equal(url.searchParams.get("project"), "renai-sim");
  assert.equal(url.searchParams.get("archived"), "true");
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-api --test routes inbox` and `pnpm --filter @taskboard/client test`

Expected: FAIL 404 / missing `inbox` on Transport.

- [ ] **Step 3: Implement DTOs, route, Transport, Tauri**

Add `InboxItemDto` with `From<InboxItem>` in both DTO files. Route: `Query` struct `{ project: Option<String>, archived: Option<bool> }`. `HttpTransport.inbox` builds `/api/v1/inbox?archived=false` plus optional `project`. `TauriTransport.inbox` invokes `"inbox"`. `fakeTransport.inbox` returns `[]`. Register `commands::inbox` in the Tauri `generate_handler!` list and add a thin wrapper in `apps/desktop/src-tauri/src/commands.rs` matching existing `task_list` style.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-api --test routes inbox` and `pnpm --filter @taskboard/client test` and `cargo test -p taskboard-desktop-commands`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/api crates/desktop-commands apps/desktop packages/types packages/client packages/ui/src/fakeTransport.ts
git commit -m "feat: expose inbox over HTTP and Tauri transport"
```

---

### Task 5: Named create and shortcut legend

**Files:**
- Create: `packages/ui/src/Composer.tsx`, `packages/ui/src/Composer.module.css`
- Create: `packages/ui/src/ShortcutLegend.tsx`, `packages/ui/src/ShortcutLegend.module.css`
- Modify: `packages/ui/src/Board.tsx`
- Modify: `packages/ui/src/Sidebar.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx`
- Modify: `packages/ui/src/index.ts`
- Modify: `packages/ui/src/Board.test.tsx` (replace Untitled assertions)
- Modify: `packages/ui/src/keyboard.test.tsx` (replace `n`/`p` Untitled behavior)
- Test: `packages/ui/src/Composer.test.tsx`
- Test: `packages/ui/src/keyboard.test.tsx`

**Interfaces:**
- Consumes: `transport.taskCreate`, `transport.projectAdd`
- Produces:
  - `Composer` with props `{ placeholder: string; onSubmit: (title: string) => void; onCancel: () => void; inputRef?: Ref<HTMLInputElement> }`
  - Enter with trimmed non-empty value calls `onSubmit`; empty Enter or Escape calls `onCancel` and writes nothing
  - Board toolbar: Search, then **New task** button `name="New task"`
  - `n` focuses the task composer at the top of `currentColumn` (does not call `taskCreate`)
  - `p` / New project focuses the project composer (does not call `projectAdd` until submit)
  - `?` toggles `ShortcutLegend` listing the keyboard table from the detailed design plus `?`, `i` (note: `i` is a no-op until Task 8; still document it)
  - English copy only

- [ ] **Step 1: Write the failing UI tests**

`packages/ui/src/Composer.test.tsx`:

```tsx
it("submits trimmed title on Enter and cancels empty Enter", async () => {
  const onSubmit = vi.fn();
  const onCancel = vi.fn();
  render(<Composer placeholder="Task title" onSubmit={onSubmit} onCancel={onCancel} />);
  const input = screen.getByPlaceholderText("Task title");
  await userEvent.type(input, "  Fix login  {Enter}");
  expect(onSubmit).toHaveBeenCalledWith("Fix login");
  onSubmit.mockClear();
  await userEvent.clear(input);
  await userEvent.type(input, "{Enter}");
  expect(onSubmit).not.toHaveBeenCalled();
  expect(onCancel).toHaveBeenCalled();
});
```

Replace `Board.test.tsx` `"New project calls projectAdd with Untitled"` with:

```tsx
it("New project composer commits a named project", async () => {
  const transport = fakeTransport();
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(screen.getByRole("button", { name: "New project" }));
  expect(transport.projectAdd).not.toHaveBeenCalled();
  await userEvent.type(screen.getByPlaceholderText("Project name"), "Renai Sim{Enter}");
  await waitFor(() => expect(transport.projectAdd).toHaveBeenCalledWith({ name: "Renai Sim" }));
});
```

Replace `keyboard.test.tsx` `"n creates a task through transport"` and `"p creates a project... Untitled"` and `"1-4 jump columns so n creates in the current column"`:

```tsx
it("n opens composer and Enter creates in the current column", async () => {
  const transport = fakeTransport();
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(screen.getByRole("button", { name: "New project" }));
  await userEvent.type(screen.getByPlaceholderText("Project name"), "Alpha{Enter}");
  await screen.findByRole("list", { name: "Todo" });
  await userEvent.keyboard("2");
  await userEvent.keyboard("n");
  expect(transport.taskCreate).not.toHaveBeenCalled();
  await userEvent.type(screen.getByPlaceholderText("Task title"), "Ship it{Enter}");
  await waitFor(() =>
    expect(transport.taskCreate).toHaveBeenCalledWith("alpha", {
      title: "Ship it",
      column: "in-progress",
    }),
  );
});

it("question mark toggles shortcut legend", async () => {
  const transport = fakeTransport();
  render(<TaskboardApp transport={transport} />);
  await userEvent.keyboard("?");
  expect(screen.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeTruthy();
  expect(screen.getByText("New task")).toBeTruthy();
  await userEvent.keyboard("?");
  expect(screen.queryByRole("dialog", { name: "Keyboard shortcuts" })).toBeNull();
});
```

Keep `j`/`k`/`l`/`meta+z` tests working (they still create a project first — use the composer).

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test`

Expected: FAIL missing New task / composer / legend.

- [ ] **Step 3: Implement Composer, legend, and rewire create**

`Composer` is a single-line `<input>`. `Board` accepts `composing: boolean`, `onNewTask`, `onComposerSubmit`, `onComposerCancel`, `composerRef`. `TaskboardApp.createTask` no longer calls `taskCreate` with `"Untitled"`. `addProject` no longer uses `"Untitled"`. When no project exists, New task is hidden or no-ops. Shortcut legend is a `role="dialog"` overlay; `?` while typing in an input does not open it (`isTypingTarget` already returns early — keep that, except allow `Escape` as today).

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS. No test still expects `{ name: "Untitled" }`.

- [ ] **Step 5: Commit**

```bash
git add packages/ui
git commit -m "feat: name tasks and projects in composers instead of Untitled"
```

---

### Task 6: Toasts, safer h/l, last project

**Files:**
- Create: `packages/ui/src/Toast.tsx`, `packages/ui/src/Toast.module.css`
- Create: `crates/store-sqlite/src/ui_state.rs`
- Modify: `crates/store-sqlite/src/lib.rs`
- Modify: `crates/application/src/{app.rs,lib.rs,error.rs}` only if ui-state goes through App; prefer store-sqlite helpers called from API/desktop/CLI is out of scope for last-project — UI persists via Transport
- Modify: `crates/api/src/routes.rs`, `crates/api/src/dto.rs`
- Modify: `packages/client/src/{transport.ts,http.ts,tauri.ts}`
- Modify: `packages/ui/src/TaskboardApp.tsx`
- Modify: `packages/ui/src/columns.ts` (`COLUMNS` already has labels)
- Modify: `apps/desktop/src-tauri/src/{commands.rs,lib.rs}`
- Modify: `crates/desktop-commands/src/commands.rs`
- Test: `crates/store-sqlite` unit tests in `ui_state.rs`
- Test: `packages/ui/src/keyboard.test.tsx`
- Test: `packages/ui/src/Toast.test.tsx`
- Test: `crates/api/tests/routes.rs`

**Interfaces:**
- Consumes: `transport.undo`, `transport.taskMove`, `transport.projectList`
- Produces:
  - `UiState { last_project_slug: Option<String> }` stored in `{data_dir}/ui.toml`
  - `load_ui_state(data_dir) -> UiState` missing file = default; unknown keys ignored
  - `save_ui_state(data_dir, &UiState) -> Result<(), AppError>`
  - `GET /api/v1/ui-state` and `PATCH /api/v1/ui-state` body `{ "lastProjectSlug": "slug" | null }`
  - `Transport.uiState(): Promise<{ lastProjectSlug: string | null }>`
  - `Transport.uiStateSet(lastProjectSlug: string | null): Promise<{ lastProjectSlug: string | null }>`
  - Toast: `{ message: string; action?: { label: string; onClick: () => void }; error?: boolean }` auto-dismiss 2400ms, errors 8000ms, bottom-center
  - Keyboard move (`h`/`l` with selection) shows `Moved to {Column}` with Undo (calls `transport.undo` then `reloadBoard`)
  - `Shift+h` / `Shift+l` change `currentColumn` only (no `taskMove`)
  - On load, if `lastProjectSlug` matches a live listed project, `applyProject` that one instead of `list[0]`

Do not change `config.toml`. CLI does not write `ui.toml`.

- [ ] **Step 1: Write the failing tests**

`crates/store-sqlite/src/ui_state.rs` tests:

```rust
#[test]
fn missing_ui_toml_is_empty_slug() {
    let tmp = tempfile::tempdir().unwrap();
    let state = load_ui_state(tmp.path());
    assert_eq!(state.last_project_slug, None);
}

#[test]
fn save_and_load_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    save_ui_state(tmp.path(), &UiState { last_project_slug: Some("taskboard".into()) }).unwrap();
    let state = load_ui_state(tmp.path());
    assert_eq!(state.last_project_slug.as_deref(), Some("taskboard"));
}
```

Keyboard:

```tsx
it("shift+l changes current column without moving", async () => {
  const transport = fakeTransport();
  const project = await transport.projectAdd({ name: "Alpha" });
  await transport.taskCreate(project.slug, { title: "Stay", column: "todo" });
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(await screen.findByText("Stay"));
  await userEvent.keyboard("{Shift>}l{/Shift}");
  expect(transport.taskMove).not.toHaveBeenCalled();
});

it("l move shows undo toast", async () => {
  const transport = fakeTransport();
  const project = await transport.projectAdd({ name: "Alpha" });
  await transport.taskCreate(project.slug, { title: "Move me", column: "todo" });
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(await screen.findByText("Move me"));
  await userEvent.keyboard("l");
  await waitFor(() => expect(transport.taskMove).toHaveBeenCalled());
  expect(await screen.findByText("Moved to In Progress")).toBeTruthy();
  await userEvent.click(screen.getByRole("button", { name: "Undo" }));
  await waitFor(() => expect(transport.undo).toHaveBeenCalled());
});
```

API: `GET /api/v1/ui-state` returns `{ entity: { lastProjectSlug: null } }`; PATCH then GET returns the slug.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-store-sqlite ui_state` and `pnpm --filter @taskboard/ui test`

Expected: FAIL missing module / toast / Shift+l still moves (current `l` handler ignores shift).

- [ ] **Step 3: Implement**

`load_ui_state` uses `toml::from_str` like `config.rs`. `save_ui_state` writes:

```toml
last_project_slug = "taskboard"
```

or omits the key when `None`. `TaskboardApp` onKeyDown: if `e.shiftKey && (e.key === "h" || e.key === "H" || e.key === "l" || e.key === "L")` then `jump` via `neighborColumn` without `moveSelected`. Toast region is a sibling inside `.app`. After `applyProject`, call `uiStateSet(project.slug)`. Initial load: `uiState()` then pick matching project.

Wire App helpers used by HTTP/desktop:

```rust
pub fn ui_state(data_dir: &Path) -> UiState { load_ui_state(data_dir) }
```

Pass `data_dir` from API `AppState` — if `AppState` does not have `data_dir` today, add it in `crates/api/src/server.rs` where `serve` already knows the store path. Desktop `resolve_data_dir` is already in `apps/desktop/src-tauri/src/lib.rs`; store it on `DesktopState`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-store-sqlite ui_state` and `cargo test -p taskboard-api --test routes ui_state` and `pnpm --filter @taskboard/ui test`

Expected: PASS. Existing `l moves the selected card` test still passes.

- [ ] **Step 5: Commit**

```bash
git add crates/store-sqlite crates/api crates/desktop-commands apps/desktop packages/ui packages/client packages/types
git commit -m "feat: add move toasts, shift column jump, and last project ui.toml"
```

---

### Task 7: Inspector, Trash, project menu, delete confirm

**Files:**
- Create: `packages/ui/src/TrashPanel.tsx`, `packages/ui/src/TrashPanel.module.css`
- Create: `packages/ui/src/ConfirmDialog.tsx`
- Modify: `packages/ui/src/Inspector.tsx`
- Modify: `packages/ui/src/Sidebar.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx`
- Modify: `packages/ui/src/fakeTransport.ts` (trashList should remember deleted tasks)
- Test: `packages/ui/src/Inspector.test.tsx`
- Test: `packages/ui/src/TrashPanel.test.tsx`
- Test: `packages/ui/src/keyboard.test.tsx` (Delete key)

**Interfaces:**
- Consumes: `linkAdd`, `linkRemove`, `trashList`, `taskRestore`, `projectRestore`, `projectUpdate`, `projectArchive`, `projectDelete`, `projectReorder`, `taskDelete`
- Produces:
  - Inspector Links: text input + Add (URL if value contains `://` else path), each row a Remove button
  - Inspector run row: `{displayId} {status} — {message}` and if `waitingReason` present ` — wait: {waitingReason}`
  - Inspector activity: `{operation} · {actorLabel} · {relative or ISO createdAt}`
  - Delete button opens confirm `Delete {title}?` with Delete / Cancel; success toast `Deleted {TASK-n}` with Undo
  - Board-focused `Delete` / `Backspace` (not while typing) same confirm
  - Trash button opens panel listing `trash.projects` and `trash.tasks` with Restore; empty copy `Trash is empty`
  - Selected project name is an input committed on blur via `projectUpdate({ name })`
  - `⋯` menu `aria-label="Project actions"`: Set path (prompt via small inline field), Archive, Delete project (confirm), Move up, Move down (reorder adjacent slugs through `projectReorder`)

- [ ] **Step 1: Write the failing tests**

```tsx
it("adds and removes a link from the inspector", async () => {
  const transport = fakeTransport();
  const project = await transport.projectAdd({ name: "Alpha" });
  await transport.taskCreate(project.slug, { title: "Linked", column: "todo" });
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(await screen.findByText("Linked"));
  await userEvent.type(screen.getByPlaceholderText("https:// or /path"), "https://example.com");
  await userEvent.click(screen.getByRole("button", { name: "Add link" }));
  await waitFor(() => expect(transport.linkAdd).toHaveBeenCalled());
  expect(await screen.findByText("https://example.com")).toBeTruthy();
});

it("trash panel restores a deleted task", async () => {
  const transport = fakeTransport();
  const project = await transport.projectAdd({ name: "Alpha" });
  await transport.taskCreate(project.slug, { title: "Gone", column: "todo" });
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(await screen.findByText("Gone"));
  await userEvent.click(screen.getByRole("button", { name: "Delete" }));
  await userEvent.click(screen.getByRole("button", { name: "Delete" })); // confirm
  await userEvent.click(screen.getByRole("button", { name: "Trash" }));
  expect(await screen.findByText("Gone")).toBeTruthy();
  await userEvent.click(screen.getByRole("button", { name: "Restore TASK-1" }));
  await waitFor(() => expect(transport.taskRestore).toHaveBeenCalledWith("TASK-1"));
});

it("delete key confirms and deletes the selected card", async () => {
  const transport = fakeTransport();
  const project = await transport.projectAdd({ name: "Alpha" });
  await transport.taskCreate(project.slug, { title: "Drop me", column: "todo" });
  render(<TaskboardApp transport={transport} />);
  await userEvent.click(await screen.findByText("Drop me"));
  await userEvent.keyboard("{Delete}");
  expect(transport.taskDelete).not.toHaveBeenCalled();
  await userEvent.click(screen.getByRole("button", { name: "Delete" }));
  await waitFor(() => expect(transport.taskDelete).toHaveBeenCalled());
});
```

Update `fakeTransport.taskDelete` to push the detail into an internal trash list that `trashList` returns. Transport error on `taskShow` that is not `not_found`: TaskboardApp currently swallows; add a test that a thrown `Error("nope")` after select shows toast `nope` — only if `TransportError` exists; for fakeTransport, `taskShow` reject with `new Error("nope")` and expect `screen.findByText("nope")`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test`

Expected: FAIL missing Add link / Trash panel / confirm.

- [ ] **Step 3: Implement inspector, trash, menu, confirm**

Keep inspector section order: Title, id, column, urgent, note, links, runs, activity, Delete. `ConfirmDialog` is a `role="alertdialog"`. Mutation failures (`taskMove`, `taskCreate`, `undo` reject) `toast` the `error.message`. `not_found` on `taskShow` still clears selection without a toast.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/ui
git commit -m "feat: wire links, trash restore, project actions, and delete confirm"
```

---

### Task 8: Inbox UI strip and card waiting_reason

**Files:**
- Create: `packages/ui/src/InboxStrip.tsx`, `packages/ui/src/InboxStrip.module.css`
- Modify: `packages/ui/src/Board.tsx` (or TaskboardApp above Board)
- Modify: `packages/ui/src/Card.tsx`
- Modify: `packages/ui/src/Sidebar.tsx`
- Modify: `packages/ui/src/TaskboardApp.tsx`
- Modify: `packages/ui/src/fakeTransport.ts` (`inbox` implementation from in-memory tasks)
- Test: `packages/ui/src/InboxStrip.test.tsx`
- Test: `packages/ui/src/Card.test.tsx`
- Test: `packages/ui/src/keyboard.test.tsx` (`i` toggle)

**Interfaces:**
- Consumes: `Transport.inbox`
- Produces:
  - Strip above columns, hidden when `items.length === 0`
  - Collapsed: `Inbox · N` plus `Waiting a · Failed b · Urgent c` (omit zero counts). Amber pip if any Waiting
  - Click header or `i` expands/collapses (session-only)
  - Scope toggle buttons `This project` (default) and `All projects`
  - Expanded row: `{projectName} · {displayId} · {title} · {badge} · {reason}`; click calls existing `applyProject` + `selectCard`
  - Sidebar project button shows a count badge when that project's inbox count > 0 (from All-projects inbox)
  - Card face: if `(waiting|failed)` and `!runMessage && waitingReason`, show `waitingReason` with class `runMessage`

- [ ] **Step 1: Write the failing tests**

```tsx
it("hides inbox when empty and shows waiting row when present", async () => {
  const transport = fakeTransport();
  const project = await transport.projectAdd({ name: "Alpha" });
  await transport.taskCreate(project.slug, { title: "Idle", column: "todo" });
  render(<TaskboardApp transport={transport} />);
  await screen.findByText("Idle");
  expect(screen.queryByText(/Inbox ·/)).toBeNull();
});

it("lists waiting cards and jump selects them", async () => {
  const transport = fakeTransport();
  const a = await transport.projectAdd({ name: "Alpha" });
  const b = await transport.projectAdd({ name: "Beta" });
  const t1 = await transport.taskCreate(a.slug, { title: "Wait A", column: "todo" });
  const t2 = await transport.taskCreate(b.slug, { title: "Wait B", column: "todo" });
  // fakeTransport.inbox must return these when displayStatus waiting
  (t1 as TaskDetail).displayStatus = "waiting";
  (t2 as TaskDetail).displayStatus = "waiting";
  render(<TaskboardApp transport={transport} />);
  expect(await screen.findByText(/Inbox · 2/)).toBeTruthy();
  await userEvent.click(screen.getByRole("button", { name: /Inbox/ }));
  await userEvent.click(screen.getByRole("button", { name: "All projects" }));
  await userEvent.click(screen.getByRole("button", { name: /Wait B/ }));
  expect(await screen.findByDisplayValue("Wait B")).toBeTruthy();
});

it("i toggles the inbox strip expanded", async () => {
  // seed one waiting card, press i, expect a row, press i, row hidden
});
```

Card:

```tsx
it("shows waitingReason when runMessage is empty", () => {
  const { getByText } = render(
    <Card
      task={summary({ displayStatus: "waiting", runMessage: null, waitingReason: "Need spec" })}
      selected={false}
    />,
  );
  expect(getByText("Need spec")).toBeTruthy();
});
```

Extend `fakeTransport.inbox` to scan tasks and details: membership using the same rules (waiting/failed/urgent && column !== "done"), attach `projectSlug`/`projectName`/`reason`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `pnpm --filter @taskboard/ui test`

Expected: FAIL missing Inbox copy / waitingReason on card.

- [ ] **Step 3: Implement InboxStrip and card fallback**

`TaskboardApp` loads inbox on mount, after `reloadBoard`, and when scope changes. This-project scope calls `inbox({ project: selectedSlug })`. All-projects calls `inbox()`. Sidebar badges use All-projects counts grouped by `projectSlug`. Do not persist expanded/scope. Use existing badge colors.

- [ ] **Step 4: Run tests to verify they pass**

Run: `pnpm --filter @taskboard/ui test`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add packages/ui
git commit -m "feat: show attention inbox strip and waiting reasons on cards"
```

---

### Task 9: `tb project detect` and optional `--project`

**Files:**
- Create: `crates/application/src/detect.rs` (or functions in `app.rs`)
- Modify: `crates/application/src/{lib.rs,error.rs,app.rs}`
- Modify: `crates/cli/src/{args.rs,main.rs}`
- Modify: `AGENTS.md` (only the session workflow opening list; keep TASK-n mutations)
- Test: `crates/application/tests/detect.rs`
- Test: `crates/cli/tests/cli_json.rs`
- Test: `crates/application/src/error.rs` unit test for `project_required`

**Interfaces:**
- Consumes: `App::project_list(false)` (live non-archived)
- Produces:
  - `AppError::ProjectRequired` with `code() == "project_required"` and message `pass --project or set TASKBOARD_PROJECT`
  - `pub async fn App::detect_project(&self, cwd: &Path, env_slug: Option<&str>) -> Result<Project, AppError>`
  - Resolution: `env_slug` if the live slug exists; else unique live project whose `repo_path` is an ancestor of `cwd` (longest `repo_path` wins if several match); else `project_required`. Empty/missing `repo_path` never matches. `env_slug` that does not exist is `not_found`, not `project_required`
  - `tb project detect [--json]` prints the Project entity (human: slug and name)
  - `tb task create` and `tb task list`: `--project` optional; when omitted, call `detect_project(cwd, TASKBOARD_PROJECT)`
  - `tb inbox` does **not** use detect (stays all-projects when `--project` omitted)
  - Titles still cannot address tasks

AGENTS.md change (replace the first two bullets of the session workflow):

```markdown
1. `tb project detect --json` — if `project_required`, `project list --json` then `project add --name taskboard --path <repo-root> --json`
2. `task list --project taskboard --json` — reuse a matching `TASK-n`, or `task create --project taskboard --title "<short title>" --json`
```

Keep `--json --actor cursor` and TASK-n rules.

- [ ] **Step 1: Write the failing tests**

`crates/application/tests/detect.rs`:

```rust
#[tokio::test]
async fn detect_uses_env_slug() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(&actor, ProjectAdd { name: "Renai Sim".into(), repo_path: Some("/tmp/renai".into()), slug: None }).await.unwrap();
    let found = app.detect_project(Path::new("/tmp/unrelated"), Some("renai-sim")).await.unwrap();
    assert_eq!(found.slug, "renai-sim");
}

#[tokio::test]
async fn detect_unique_repo_ancestor() {
    let app = test_app().await;
    let actor = cli_actor();
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("renai");
    std::fs::create_dir_all(repo.join("crates")).unwrap();
    app.project_add(&actor, ProjectAdd { name: "Renai Sim".into(), repo_path: Some(repo.to_string_lossy().into()), slug: None }).await.unwrap();
    let found = app.detect_project(&repo.join("crates"), None).await.unwrap();
    assert_eq!(found.slug, "renai-sim");
}

#[tokio::test]
async fn detect_missing_is_project_required() {
    let app = test_app().await;
    let err = app.detect_project(Path::new("/tmp/nope"), None).await.unwrap_err();
    assert_eq!(err.code(), "project_required");
}

#[tokio::test]
async fn detect_unknown_env_slug_is_not_found() {
    let app = test_app().await;
    let err = app.detect_project(Path::new("/tmp"), Some("missing")).await.unwrap_err();
    assert_eq!(err.code(), "not_found");
}

#[tokio::test]
async fn detect_longest_repo_path_wins() {
    let app = test_app().await;
    let actor = cli_actor();
    let tmp = tempfile::tempdir().unwrap();
    let parent = tmp.path().join("mono");
    let child = parent.join("svc");
    std::fs::create_dir_all(&child).unwrap();
    app.project_add(&actor, ProjectAdd { name: "Mono".into(), repo_path: Some(parent.to_string_lossy().into()), slug: None }).await.unwrap();
    app.project_add(&actor, ProjectAdd { name: "Svc".into(), repo_path: Some(child.to_string_lossy().into()), slug: None }).await.unwrap();
    let found = app.detect_project(&child, None).await.unwrap();
    assert_eq!(found.slug, "svc");
}
```

CLI:

```rust
#[test]
fn project_detect_json_from_path() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    tb_in(&dir).args(["project", "add", "--name", "Renai", "--path", repo.to_str().unwrap()]).assert().success();
    let out = tb_in(&dir)
        .current_dir(&repo)
        .args(["project", "detect", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["entity"]["slug"], "renai");
}

#[test]
fn task_list_without_project_uses_detect() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    tb_in(&dir).args(["project", "add", "--name", "Renai", "--path", repo.to_str().unwrap()]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "renai", "--title", "X"]).assert().success();
    let out = tb_in(&dir)
        .current_dir(&repo)
        .args(["task", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["entities"][0]["title"], "X");
}

#[test]
fn task_list_without_project_unlinked_is_project_required() {
    let (mut cmd, _dir) = tb();
    let out = cmd.args(["task", "list", "--json"]).assert().failure().code(1).get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["error"]["code"], "project_required");
}
```

Error unit test: `assert_eq!(AppError::ProjectRequired.code(), "project_required");`

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-application --test detect -- --test-threads=1`

Expected: FAIL compile.

- [ ] **Step 3: Implement detect and optional `--project`**

Ancestor check: canonicalize when the path exists; otherwise prefix-match on `/` boundaries so `/tmp/renai` is an ancestor of `/tmp/renai/crates` and not of `/tmp/renai-other`. `task create` / `task list` in `main.rs`: if `project` is `None`, `let project = app.detect_project(&std::env::current_dir().unwrap(), std::env::var("TASKBOARD_PROJECT").ok().as_deref()).await?;` then use `project.slug`. Add `ProjectCommand::Detect`. Print errors through existing `print_error`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-application --test detect -- --test-threads=1` and `cargo test -p taskboard-cli --test cli_json` and `cargo test -p taskboard-application error::`

Expected: PASS. Existing `--project` tests still pass.

- [ ] **Step 5: Commit**

```bash
git add crates/application crates/cli AGENTS.md
git commit -m "feat: detect current project from repo path and TASKBOARD_PROJECT"
```

---

## Execution notes

- Run `cargo test -p taskboard-application -- --test-threads=1` after Tasks 1–3 and 9 (SQLite file locking).
- After Task 5, grep the repo for `Untitled` in UI tests; none should remain except slugify fallback in `fakeTransport` for an empty name (composer must not submit empty names).
- Do not add `tb work start`, Cmd-K, or title-as-ID in any task.
- Slice mapping: Tasks 1–4 = Inbox backend; 5–6 = Mechanism 1+3; 7 = spec debt; 8 = Inbox UI; 9 = Mechanism 4.
