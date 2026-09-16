# Cross-cutting task list filters and run read APIs

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let agents list tasks across projects by display status, column, and agent, and read runs via `run list` / `run show` / `run current` / `--session` without washing every project's `task show` array.

**Architecture:** Keep `task_list(slug)` for existing callers. Add `App::task_query` that walks live projects the same way `inbox` does, then filters summaries in process. Extract `winning_run_view` so `run current` uses the same run that drives `CardDisplayStatus`. Add `App::run_list`, `run_show`, `run_current`, and `run_by_session` as read-only use cases. Wire CLI only — no HTTP, Tauri, or `packages/ui` changes.

**Tech Stack:** Rust crates `taskboard-core`, `taskboard-application`, `taskboard-cli`; clap; existing SQLite store; `assert_cmd` / tokio application tests.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Titles are never task identifiers; mutations still require `TASK-n` / `RUN-n`
- CLI `--json` is snake_case; HTTP JSON is camelCase (HTTP is out of scope)
- Do not open HTTP to agents
- Do not change Inbox membership (`tb inbox` / #52)
- Do not implement UI `/` search widening (#67)
- Prefer CLI/domain; keep `packages/ui` untouched
- `--actor` remains global; JSON success is `"ok": true` with `entity` / `entities`

## File map

- Modify: `crates/core/src/display_status.rs` — `as_str`, `FromStr`, `winning_run_view`
- Modify: `crates/core/src/lib.rs` — re-export new items
- Modify: `crates/application/src/commands.rs` — `TaskListQuery`, `RunListQuery`
- Modify: `crates/application/src/lib.rs` — export query structs
- Modify: `crates/application/src/app.rs` — `task_query`, run reads, share winning-run helper
- Modify: `crates/application/src/store.rs` — `list_all_runs`, `list_runs_by_session_id`
- Modify: `crates/store-sqlite/src/store.rs` — implement the two Store methods
- Modify: `crates/cli/src/args.rs` — task list flags; `run list` / `show` / `current`
- Modify: `crates/cli/src/main.rs` — dispatch
- Modify: `crates/cli/src/output.rs` — human run list/show
- Modify: `AGENTS.md` — one line pointing at `run current` / `run list --open`
- Test: `crates/core/src/display_status.rs` unit tests
- Test: `crates/application/tests/task_query.rs`
- Test: `crates/application/tests/run_reads.rs`
- Test: `crates/cli/tests/cli_json.rs`

---

### Task 1: Parse display status and share winning-run selection

**Files:**
- Modify: `crates/core/src/display_status.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `RunStatus`, `RunStatusView`, existing `card_display_status`
- Produces:
  - `CardDisplayStatus::as_str(self) -> &'static str` (`idle` / `running` / `waiting` / `failed` / `completed`)
  - `impl FromStr for CardDisplayStatus` with `ParseCardDisplayStatusError`
  - `pub fn winning_run_view(runs: &[RunStatusView]) -> Option<&RunStatusView>` — same filter + `max_by(started_at, display_id)` as today's `card_display_status`

- [ ] **Step 1: Write the failing tests**

Add to `crates/core/src/display_status.rs` inside the existing `tests` module:

```rust
    #[test]
    fn display_status_from_str_round_trips() {
        assert_eq!(
            "running".parse::<CardDisplayStatus>().unwrap(),
            CardDisplayStatus::Running
        );
        assert_eq!(CardDisplayStatus::InReviewWait, CardDisplayStatus::Waiting);
    }
```

Do not add `InReviewWait`. Use this instead:

```rust
    #[test]
    fn display_status_from_str_round_trips() {
        for status in [
            CardDisplayStatus::Idle,
            CardDisplayStatus::Running,
            CardDisplayStatus::Waiting,
            CardDisplayStatus::Failed,
            CardDisplayStatus::Completed,
        ] {
            assert_eq!(status.as_str().parse::<CardDisplayStatus>().unwrap(), status);
        }
        assert!("active".parse::<CardDisplayStatus>().is_err());
    }

    #[test]
    fn winning_run_view_picks_newest_active_then_terminal() {
        let started = Utc.with_ymd_and_hms(2026, 9, 5, 10, 0, 0).unwrap();
        let later = Utc.with_ymd_and_hms(2026, 9, 5, 11, 0, 0).unwrap();
        let runs = [
            RunStatusView {
                status: RunStatus::Completed,
                started_at: later,
                display_id: "RUN-1".into(),
            },
            RunStatusView {
                status: RunStatus::Waiting,
                started_at: started,
                display_id: "RUN-2".into(),
            },
        ];
        assert_eq!(
            winning_run_view(&runs).map(|run| run.display_id.as_str()),
            Some("RUN-2")
        );
        assert_eq!(winning_run_view(&[]), None);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-core display_status_from_str_round_trips winning_run_view_picks_newest_active_then_terminal -- --nocapture`

Expected: FAIL — `as_str` / `winning_run_view` / `FromStr` not found.

- [ ] **Step 3: Write minimal implementation**

In `crates/core/src/display_status.rs` add `use std::str::FromStr;` and `use thiserror::Error;`. After `CardDisplayStatus`:

```rust
#[derive(Debug, Error, PartialEq, Eq)]
#[error("unknown display status: {0}")]
pub struct ParseCardDisplayStatusError(pub String);

impl CardDisplayStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            CardDisplayStatus::Idle => "idle",
            CardDisplayStatus::Running => "running",
            CardDisplayStatus::Waiting => "waiting",
            CardDisplayStatus::Failed => "failed",
            CardDisplayStatus::Completed => "completed",
        }
    }
}

impl FromStr for CardDisplayStatus {
    type Err = ParseCardDisplayStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "idle" => Ok(CardDisplayStatus::Idle),
            "running" => Ok(CardDisplayStatus::Running),
            "waiting" => Ok(CardDisplayStatus::Waiting),
            "failed" => Ok(CardDisplayStatus::Failed),
            "completed" => Ok(CardDisplayStatus::Completed),
            other => Err(ParseCardDisplayStatusError(other.to_string())),
        }
    }
}

pub fn winning_run_view(runs: &[RunStatusView]) -> Option<&RunStatusView> {
    if runs.is_empty() {
        return None;
    }
    let has_active = runs
        .iter()
        .any(|run| matches!(run.status, RunStatus::Running | RunStatus::Waiting));
    runs.iter()
        .filter(|run| !has_active || matches!(run.status, RunStatus::Running | RunStatus::Waiting))
        .max_by(|left, right| {
            left.started_at
                .cmp(&right.started_at)
                .then_with(|| left.display_id.cmp(&right.display_id))
        })
}

pub fn card_display_status(runs: &[RunStatusView]) -> CardDisplayStatus {
    match winning_run_view(runs) {
        None => CardDisplayStatus::Idle,
        Some(best) => match best.status {
            RunStatus::Running => CardDisplayStatus::Running,
            RunStatus::Waiting => CardDisplayStatus::Waiting,
            RunStatus::Failed => CardDisplayStatus::Failed,
            RunStatus::Completed => CardDisplayStatus::Completed,
        },
    }
}
```

Replace the old `card_display_status` body with the version above (keep the existing function, do not duplicate it).

In `crates/core/src/lib.rs` change the display_status export to:

```rust
pub use display_status::{
    card_display_status, winning_run_view, CardDisplayStatus, ParseCardDisplayStatusError,
    RunStatusView,
};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p taskboard-core --lib`

Expected: PASS (existing display-status tests still pass because selection is unchanged).

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/display_status.rs crates/core/src/lib.rs docs/superpowers/plans/2026-09-16-issue-77-cross-cutting-task-run-apis.md
git commit -m "feat: parse card display status and share winning-run selection"
```

---

### Task 2: `App::task_query` filters across projects

**Files:**
- Modify: `crates/application/src/commands.rs`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/application/src/app.rs` (`task_list` stays; add `task_query`)
- Create: `crates/application/tests/task_query.rs`

**Interfaces:**
- Consumes: `App::task_list`, `to_task_summary`, `display_from_runs`, `require_live_project`, `Store::list_projects` / `list_tasks` / `list_runs`
- Produces:
  - `pub struct TaskListQuery { pub project: Option<String>, pub statuses: Vec<CardDisplayStatus>, pub column: Option<Column>, pub agent: Option<String> }`
  - `App::task_query(&self, query: TaskListQuery) -> Result<Vec<TaskSummary>, AppError>`
  - `project: Some(slug)` lists that live project (same not_found as `task_list`)
  - `project: None` lists every live non-archived project (inbox default, no archived)
  - `statuses` empty = no status filter; otherwise keep summaries whose `display_status` is in the list
  - `column` filters `TaskSummary.column`
  - `agent` filters by the winning run's `agent` (exact). Idle cards with no runs never match
  - Sort within a project stays urgent-then-position (existing `list_tasks` order). Across projects: project `sort_order`, then that task order
  - `task_list(slug)` becomes `task_query(TaskListQuery { project: Some(slug.into()), ..Default::default() })`

- [ ] **Step 1: Write the failing application tests**

Create `crates/application/tests/task_query.rs`:

```rust
use std::ops::Deref;

use taskboard_application::{
    Actor, App, ProjectAdd, RunStart, RunWait, SystemClock, TaskCreate, TaskListQuery,
};
use taskboard_core::{ActorKind, CardDisplayStatus, Column};
use taskboard_store_sqlite::{open_db, SqliteStore};

struct TestApp {
    app: App,
    _tmp: tempfile::TempDir,
}

impl Deref for TestApp {
    type Target = App;
    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

fn cli_actor() -> Actor {
    Actor {
        kind: ActorKind::Cli,
        label: "local-cli".into(),
    }
}

async fn test_app() -> TestApp {
    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    let store = SqliteStore::new(pool, tmp.path());
    TestApp {
        app: App::new(store, SystemClock),
        _tmp: tmp,
    }
}

#[tokio::test]
async fn task_query_filters_all_projects_by_status_column_and_agent() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "Alpha".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "Beta".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "alpha".into(),
            title: "Review cursor".into(),
            column: Some(Column::InReview),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "beta".into(),
            title: "Wait other".into(),
            column: Some(Column::InReview),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "alpha".into(),
            title: "Todo cursor".into(),
            column: Some(Column::Todo),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-1".into(),
            agent: "cursor".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-2".into(),
            agent: "codex".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    app.run_wait(
        &actor,
        taskboard_application::RunWait {
            run_display_id: "RUN-2".into(),
            reason: "need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-3".into(),
            agent: "cursor".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();

    let listed = app
        .task_query(TaskListQuery {
            project: None,
            statuses: vec![CardDisplayStatus::Running, CardDisplayStatus::Waiting],
            column: Some(Column::InReview),
            agent: Some("cursor".into()),
        })
        .await
        .unwrap();
    let ids: Vec<_> = listed.iter().map(|task| task.display_id.as_str()).collect();
    assert_eq!(ids, ["TASK-1"]);
}

#[tokio::test]
async fn task_query_unknown_project_is_not_found() {
    let app = test_app().await;
    let err = app
        .task_query(TaskListQuery {
            project: Some("missing".into()),
            statuses: Vec::new(),
            column: None,
            agent: None,
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), "not_found");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-application --test task_query -- --nocapture`

Expected: FAIL — `TaskListQuery` / `task_query` not found.

- [ ] **Step 3: Write minimal implementation**

Add to `crates/application/src/commands.rs`:

```rust
use taskboard_core::{CardDisplayStatus, Column, LinkKind};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskListQuery {
    pub project: Option<String>,
    pub statuses: Vec<CardDisplayStatus>,
    pub column: Option<Column>,
    pub agent: Option<String>,
}
```

(`Column` / `LinkKind` stay; add `CardDisplayStatus` to the existing import rather than adding a second `use`.)

Export from `crates/application/src/lib.rs`:

```rust
pub use commands::{
    InboxScope, LinkAdd, ProjectAdd, ProjectUpdate, RunFail, RunFinish, RunListQuery, RunStart,
    RunUpdate, RunWait, TaskCreate, TaskListQuery, TaskUpdate,
};
```

Do not add `RunListQuery` until Task 3 — keep this export as:

```rust
pub use commands::{
    InboxScope, LinkAdd, ProjectAdd, ProjectUpdate, RunFail, RunFinish, RunStart, RunUpdate,
    RunWait, TaskCreate, TaskListQuery, TaskUpdate,
};
```

In `crates/application/src/app.rs` import `TaskListQuery` and `winning_run_view`. Replace `task_list` with:

```rust
    pub async fn task_list(&self, project_slug: &str) -> Result<Vec<TaskSummary>, AppError> {
        self.task_query(TaskListQuery {
            project: Some(project_slug.to_string()),
            statuses: Vec::new(),
            column: None,
            agent: None,
        })
        .await
    }

    pub async fn task_query(&self, query: TaskListQuery) -> Result<Vec<TaskSummary>, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let projects = if let Some(slug) = query.project.as_deref() {
            vec![require_live_project(store, slug).await?]
        } else {
            store.list_projects(false).await?
        };
        let mut summaries = Vec::new();
        for project in projects {
            let tasks = store.list_tasks(project.id).await?;
            for task in tasks {
                let runs = store.list_runs(task.id).await?;
                let summary = to_task_summary_from_runs(task, &runs);
                if let Some(column) = query.column {
                    if summary.column != column {
                        continue;
                    }
                }
                if !query.statuses.is_empty() && !query.statuses.contains(&summary.display_status) {
                    continue;
                }
                if let Some(agent) = query.agent.as_deref() {
                    let Some(run) = winning_run(&runs) else {
                        continue;
                    };
                    if run.agent != agent {
                        continue;
                    }
                }
                summaries.push(summary);
            }
        }
        Ok(summaries)
    }
```

Add helpers next to `display_from_runs` (reuse the existing `winning` local by extracting `winning_run`):

```rust
fn winning_run(runs: &[Run]) -> Option<&Run> {
    let views: Vec<RunStatusView> = runs
        .iter()
        .map(|run| RunStatusView {
            status: run.status,
            started_at: run.started_at,
            display_id: run.display_id.clone(),
        })
        .collect();
    let view = winning_run_view(&views)?;
    runs.iter()
        .find(|run| run.display_id == view.display_id)
}

fn to_task_summary_from_runs(task: Task, runs: &[Run]) -> TaskSummary {
    let (display_status, run_message, waiting_reason) = display_from_runs(runs);
    TaskSummary {
        id: task.id,
        display_id: task.display_id,
        project_id: task.project_id,
        title: task.title,
        column: task.column,
        urgent: task.urgent,
        revision: task.revision,
        display_status,
        run_message,
        waiting_reason,
    }
}

async fn to_task_summary(store: &mut dyn Store, task: Task) -> Result<TaskSummary, AppError> {
    let runs = store.list_runs(task.id).await?;
    Ok(to_task_summary_from_runs(task, &runs))
}

fn display_from_runs(
    runs: &[Run],
) -> (
    taskboard_core::CardDisplayStatus,
    Option<String>,
    Option<String>,
) {
    let winning = winning_run(runs);
    let display_status = {
        let views: Vec<RunStatusView> = runs
            .iter()
            .map(|run| RunStatusView {
                status: run.status,
                started_at: run.started_at,
                display_id: run.display_id.clone(),
            })
            .collect();
        card_display_status(&views)
    };
    let run_message = winning.and_then(|run| run.message.clone());
    let waiting_reason = winning.and_then(|run| run.waiting_reason.clone());
    (display_status, run_message, waiting_reason)
}
```

Add `TaskListQuery` and `winning_run_view` to the `use` lists at the top of `app.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p taskboard-application --test task_query --test tasks`

Expected: PASS. Existing `task_list` tests keep working through the wrapper.

- [ ] **Step 5: Commit**

```bash
git add crates/application/src/commands.rs crates/application/src/lib.rs crates/application/src/app.rs crates/application/tests/task_query.rs
git commit -m "feat: filter task lists by status, column, agent, and project"
```

---

### Task 3: Run read APIs (`list` / `show` / `current` / session)

**Files:**
- Modify: `crates/application/src/commands.rs`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/application/src/store.rs`
- Modify: `crates/store-sqlite/src/store.rs`
- Modify: `crates/application/src/app.rs`
- Create: `crates/application/tests/run_reads.rs`

**Interfaces:**
- Consumes: `require_run`, `require_live_task`, `winning_run`, `Store::list_runs`
- Produces:
  - `pub struct RunListQuery { pub open: bool, pub session_id: Option<String>, pub agent: Option<String> }`
  - `Store::list_all_runs(&mut self) -> Result<Vec<Run>, AppError>` — every run whose task and project are live and the project is not archived, newest `started_at` then `display_id` first
  - `Store::list_runs_by_session_id(&mut self, session_id: &str) -> Result<Vec<Run>, AppError>` — exact `session_id` match, same live-task/project filter, same sort
  - `App::run_list(&self, query: RunListQuery) -> Result<Vec<Run>, AppError>`
    - `open: true` keeps `running` and `waiting` only
    - `session_id: Some` keeps exact session
    - `agent: Some` keeps exact `run.agent`
  - `App::run_show(&self, display_id: &str) -> Result<Run, AppError>` — existing `require_run` / `not_found`
  - `App::run_current(&self, task_display_id: &str) -> Result<Run, AppError>` — live task, then `winning_run`; no runs → `NotFound { entity: "run", id: task_display_id }`
  - `App::run_by_session(&self, session_id: &str) -> Result<Run, AppError>` — first row of `list_runs_by_session_id` (already newest); empty → `NotFound { entity: "run", id: session_id }`
  - Blank `session_id` on `run_by_session` is `validation_error` field `session_id`

- [ ] **Step 1: Write the failing tests**

Create `crates/application/tests/run_reads.rs` using the same `TestApp` helper pattern as Task 2:

```rust
use std::ops::Deref;

use taskboard_application::{
    Actor, App, ProjectAdd, RunListQuery, RunStart, RunWait, SystemClock, TaskCreate,
};
use taskboard_core::{ActorKind, RunStatus};
use taskboard_store_sqlite::{open_db, SqliteStore};

struct TestApp {
    app: App,
    _tmp: tempfile::TempDir,
}

impl Deref for TestApp {
    type Target = App;
    fn deref(&self) -> &Self::Target {
        &self.app
    }
}

fn cli_actor() -> Actor {
    Actor {
        kind: ActorKind::Cli,
        label: "local-cli".into(),
    }
}

async fn test_app() -> TestApp {
    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    let store = SqliteStore::new(pool, tmp.path());
    TestApp {
        app: App::new(store, SystemClock),
        _tmp: tmp,
    }
}

async fn seeded_two_runs() -> TestApp {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "Renai Sim".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "One".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "Two".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-1".into(),
            agent: "cursor".into(),
            session_id: Some("sess-a".into()),
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: "TASK-2".into(),
            agent: "codex".into(),
            session_id: Some("sess-b".into()),
        },
    )
    .await
    .unwrap();
    app.run_wait(
        &actor,
        RunWait {
            run_display_id: "RUN-2".into(),
            reason: "need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    app
}

#[tokio::test]
async fn run_list_open_returns_running_and_waiting() {
    let app = seeded_two_runs().await;
    let listed = app
        .run_list(RunListQuery {
            open: true,
            session_id: None,
            agent: None,
        })
        .await
        .unwrap();
    let ids: Vec<_> = listed.iter().map(|run| run.display_id.as_str()).collect();
    assert!(ids.contains(&"RUN-1"));
    assert!(ids.contains(&"RUN-2"));
    assert!(listed.iter().all(|run| {
        matches!(run.status, RunStatus::Running | RunStatus::Waiting)
    }));
}

#[tokio::test]
async fn run_show_and_session_lookup() {
    let app = seeded_two_runs().await;
    let shown = app.run_show("RUN-1").await.unwrap();
    assert_eq!(shown.agent, "cursor");
    let by_session = app.run_by_session("sess-b").await.unwrap();
    assert_eq!(by_session.display_id, "RUN-2");
    let err = app.run_by_session("missing").await.unwrap_err();
    assert_eq!(err.code(), "not_found");
}

#[tokio::test]
async fn run_current_returns_winning_run() {
    let app = seeded_two_runs().await;
    let current = app.run_current("TASK-2").await.unwrap();
    assert_eq!(current.display_id, "RUN-2");
    assert_eq!(current.status, RunStatus::Waiting);
}

#[tokio::test]
async fn run_current_idle_task_is_not_found() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(
        &actor,
        ProjectAdd {
            name: "Renai Sim".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "Idle".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    let err = app.run_current("TASK-1").await.unwrap_err();
    assert_eq!(err.code(), "not_found");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-application --test run_reads -- --nocapture`

Expected: FAIL — `run_list` / `RunListQuery` / `run_show` / `run_current` / `run_by_session` missing.

- [ ] **Step 3: Write minimal implementation**

Add `RunListQuery` to `commands.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunListQuery {
    pub open: bool,
    pub session_id: Option<String>,
    pub agent: Option<String>,
}
```

Export it from `lib.rs`.

In `crates/application/src/store.rs` after `list_runs`:

```rust
    async fn list_all_runs(&mut self) -> Result<Vec<Run>, AppError>;
    async fn list_runs_by_session_id(
        &mut self,
        session_id: &str,
    ) -> Result<Vec<Run>, AppError>;
```

In `crates/store-sqlite/src/store.rs` after `list_runs`:

```rust
    async fn list_all_runs(&mut self) -> Result<Vec<Run>, AppError> {
        let sql = format!(
            "SELECT {RUN_COLUMNS} FROM runs
             WHERE task_id IN (
               SELECT tasks.id FROM tasks
               JOIN projects ON projects.id = tasks.project_id
               WHERE tasks.deleted_at IS NULL
                 AND projects.deleted_at IS NULL
                 AND projects.archived = 0
             )
             ORDER BY started_at DESC, display_id DESC"
        );
        let query = sqlx::query(&sql);
        let rows = run!(self, query, fetch_all).map_err(map_sqlx)?;
        rows.iter().map(run_from_row).collect()
    }

    async fn list_runs_by_session_id(
        &mut self,
        session_id: &str,
    ) -> Result<Vec<Run>, AppError> {
        let sql = format!(
            "SELECT {RUN_COLUMNS} FROM runs
             WHERE session_id = ?
               AND task_id IN (
                 SELECT tasks.id FROM tasks
                 JOIN projects ON projects.id = tasks.project_id
                 WHERE tasks.deleted_at IS NULL
                   AND projects.deleted_at IS NULL
                   AND projects.archived = 0
               )
             ORDER BY started_at DESC, display_id DESC"
        );
        let query = sqlx::query(&sql).bind(session_id);
        let rows = run!(self, query, fetch_all).map_err(map_sqlx)?;
        rows.iter().map(run_from_row).collect()
    }
```

In `App` impl:

```rust
    pub async fn run_list(&self, query: RunListQuery) -> Result<Vec<Run>, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let mut runs = if let Some(session_id) = query.session_id.as_deref() {
            let session_id = require_non_blank("session_id", session_id)?;
            store.list_runs_by_session_id(&session_id).await?
        } else {
            store.list_all_runs().await?
        };
        if query.open {
            runs.retain(|run| matches!(run.status, RunStatus::Running | RunStatus::Waiting));
        }
        if let Some(agent) = query.agent.as_deref() {
            runs.retain(|run| run.agent == agent);
        }
        Ok(runs)
    }

    pub async fn run_show(&self, display_id: &str) -> Result<Run, AppError> {
        let mut store = self.store.lock().await;
        require_run(&mut **store, display_id).await
    }

    pub async fn run_current(&self, task_display_id: &str) -> Result<Run, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let task = require_live_task(store, task_display_id).await?;
        let runs = store.list_runs(task.id).await?;
        winning_run(&runs)
            .cloned()
            .ok_or_else(|| AppError::NotFound {
                entity: "run".into(),
                id: task_display_id.to_string(),
            })
    }

    pub async fn run_by_session(&self, session_id: &str) -> Result<Run, AppError> {
        let session_id = require_non_blank("session_id", session_id)?;
        let mut store = self.store.lock().await;
        store
            .list_runs_by_session_id(&session_id)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::NotFound {
                entity: "run".into(),
                id: session_id,
            })
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p taskboard-application --test run_reads --test notes_runs`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/application/src/commands.rs crates/application/src/lib.rs crates/application/src/store.rs crates/application/src/app.rs crates/store-sqlite/src/store.rs crates/application/tests/run_reads.rs
git commit -m "feat: add run list, show, current, and session lookup"
```

---

### Task 4: CLI flags and human/JSON output

**Files:**
- Modify: `crates/cli/src/args.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/src/output.rs`
- Modify: `crates/cli/tests/cli_json.rs`
- Modify: `AGENTS.md`

**Interfaces:**
- Consumes: `TaskListQuery`, `RunListQuery`, `App::task_query`, `run_list`, `run_show`, `run_current`, `run_by_session`
- Produces:
  - `tb task list [--all] [--project slug] [--status running,waiting] [--column in-review] [--agent cursor]`
  - `--all` conflicts with `--project`; without either, keep `resolve_project` detect
  - `--status` is comma-separated `CardDisplayStatus`; unknown token → clap error (exit 2) via `parse_status_list`
  - `tb run list [--open] [--session ID] [--agent name]`
  - `tb run show RUN-n` or `tb run show --session ID` (exactly one of positional / `--session`)
  - `tb run current TASK-n`
  - JSON stays snake_case `entities` for lists and `entity`+`revision` for single runs
  - Human run list header: `ID  AGENT  STATUS  SESSION`

- [ ] **Step 1: Write the failing CLI tests**

Append to `crates/cli/tests/cli_json.rs`:

```rust
#[test]
fn task_list_all_filters_status_column_agent() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Alpha"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["project", "add", "--name", "Beta"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "alpha",
            "--title",
            "Review me",
            "--column",
            "in-review",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "beta",
            "--title",
            "Other",
            "--column",
            "in-review",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "cursor", "--session", "s1"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-2", "--agent", "codex"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "wait", "RUN-2", "--reason", "need spec"])
        .assert()
        .success();
    let v = json_ok(
        &dir,
        &[
            "task",
            "list",
            "--all",
            "--status",
            "running,waiting",
            "--column",
            "in-review",
            "--agent",
            "cursor",
        ],
    );
    assert!(v.get("entity").is_none());
    assert_eq!(v["entities"].as_array().unwrap().len(), 1);
    assert_eq!(v["entities"][0]["display_id"], "TASK-1");
    assert_eq!(v["entities"][0]["display_status"], "running");
}

#[test]
fn run_list_show_current_and_session() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "task",
            "create",
            "--project",
            "renai-sim",
            "--title",
            "Fix login",
        ])
        .assert()
        .success();
    tb_in(&dir)
        .args([
            "run",
            "start",
            "TASK-1",
            "--agent",
            "cursor",
            "--session",
            "abc123",
        ])
        .assert()
        .success();
    let listed = json_ok(&dir, &["run", "list", "--open"]);
    assert_eq!(listed["entities"][0]["display_id"], "RUN-1");
    assert_eq!(listed["entities"][0]["status"], "running");
    let shown = json_ok(&dir, &["run", "show", "RUN-1"]);
    assert_eq!(shown["entity"]["display_id"], "RUN-1");
    assert_eq!(shown["revision"], 1);
    let current = json_ok(&dir, &["run", "current", "TASK-1"]);
    assert_eq!(current["entity"]["display_id"], "RUN-1");
    let by_session = json_ok(&dir, &["run", "show", "--session", "abc123"]);
    assert_eq!(by_session["entity"]["session_id"], "abc123");
}

#[test]
fn run_show_missing_is_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let out = tb_in(&dir)
        .args(["run", "show", "RUN-9", "--json"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "not_found");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-cli --test cli_json task_list_all_filters_status_column_agent run_list_show_current_and_session -- --nocapture`

Expected: FAIL — clap rejects unknown flags / subcommands.

- [ ] **Step 3: Write minimal implementation**

`crates/cli/src/args.rs` — change `TaskCommand::List` and `RunCommand`, add parsers:

```rust
    /// List tasks in a project or across all live projects
    List {
        #[arg(long, conflicts_with = "all")]
        project: Option<String>,
        /// List every live project instead of detecting one
        #[arg(long)]
        all: bool,
        #[arg(long, value_parser = parse_status_list)]
        status: Option<Vec<taskboard_core::CardDisplayStatus>>,
        #[arg(long, value_parser = parse_column)]
        column: Option<Column>,
        #[arg(long)]
        agent: Option<String>,
    },
```

Add these variants at the top of `RunCommand` (before `Start`):

```rust
    /// List runs
    List {
        /// Only running and waiting
        #[arg(long)]
        open: bool,
        #[arg(long = "session")]
        session_id: Option<String>,
        #[arg(long)]
        agent: Option<String>,
    },
    /// Show one run by RUN-n or session
    #[command(group(
        clap::ArgGroup::new("target")
            .required(true)
            .args(["run_id", "session_id"])
    ))]
    Show {
        run_id: Option<String>,
        #[arg(long = "session")]
        session_id: Option<String>,
    },
    /// Show the run that drives a card's display status
    Current { display_id: String },
```

Add parsers at the bottom of `args.rs`:

```rust
fn parse_status_list(s: &str) -> Result<Vec<taskboard_core::CardDisplayStatus>, String> {
    let mut statuses = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err("status must not be empty".into());
        }
        statuses.push(
            part.parse::<taskboard_core::CardDisplayStatus>()
                .map_err(|err| err.to_string())?,
        );
    }
    Ok(statuses)
}
```

`crates/cli/src/main.rs` — import `RunListQuery` and `TaskListQuery`. Replace `TaskCommand::List` with:

```rust
        TaskCommand::List {
            project,
            all,
            status,
            column,
            agent,
        } => {
            let project = if all {
                None
            } else {
                Some(
                    resolve_project(app, project)
                        .await
                        .map_err(|err| output::print_error(&err, json))?,
                )
            };
            let tasks = app
                .task_query(TaskListQuery {
                    project,
                    statuses: status.unwrap_or_default(),
                    column,
                    agent,
                })
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entities(json, &tasks, || output::print_task_list(&tasks));
        }
```

In `run_cmd`, add arms **before** `Start`:

```rust
        RunCommand::List {
            open,
            session_id,
            agent,
        } => {
            let runs = app
                .run_list(RunListQuery {
                    open,
                    session_id,
                    agent,
                })
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entities(json, &runs, || output::print_run_list(&runs));
        }
        RunCommand::Show {
            run_id,
            session_id,
        } => {
            let run = if let Some(run_id) = run_id {
                app.run_show(&run_id).await
            } else {
                app.run_by_session(session_id.as_deref().unwrap_or("")).await
            }
            .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &run, run.revision, || {
                output::print_run(&run);
            });
        }
        RunCommand::Current { display_id } => {
            let run = app
                .run_current(&display_id)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &run, run.revision, || {
                output::print_run(&run);
            });
        }
```

`crates/cli/src/output.rs` — import `Run` and add:

```rust
use taskboard_core::{CardDisplayStatus, Column, EntityType, Project, Run, TaskDetail, TaskSummary};

pub fn print_run_list(runs: &[Run]) {
    println!("ID  AGENT  STATUS  SESSION");
    for run in runs {
        println!(
            "{}  {}  {}  {}",
            run.display_id,
            run.agent,
            run.status.as_str(),
            run.session_id.as_deref().unwrap_or("-")
        );
    }
}

pub fn print_run(run: &Run) {
    println!(
        "{}  {}  [{}]",
        run.display_id,
        run.agent,
        run.status.as_str()
    );
}
```

In `AGENTS.md` after “After `run start`, use the returned `RUN-n`.” add:

```
If you forget `RUN-n`, use `run current TASK-n` or `run list --open`. `task list --all --status running,waiting` lists cards across projects.
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p taskboard-cli --test cli_json --test help`

Expected: PASS. `task_list_human_columns` still matches `ID  COLUMN  URGENT  RUN  TITLE`.

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/args.rs crates/cli/src/main.rs crates/cli/src/output.rs crates/cli/tests/cli_json.rs AGENTS.md
git commit -m "feat: expose task query and run reads on the CLI"
```

---

## Self-review

**1. Spec coverage**

| Acceptance | Task |
| --- | --- |
| `tb task list` filters across projects by status, column, agent | Tasks 2 + 4 (`--all --status --column --agent`) |
| `tb run list --open` and `tb run show RUN-n` | Tasks 3 + 4 |
| `tb run current TASK-n` is the display-status run | Tasks 1 + 3 + 4 (`winning_run_view`) |
| `--session` finds an existing `session_id` | Tasks 3 + 4 (`run show --session` / `run list --session`) |
| JSON snake_case `--json` / `--actor` | Task 4 uses existing printers |

Out of scope left out: UI search, Inbox membership, HTTP for agents, Continue, comments.

**2. Placeholder scan:** no TBD / “add tests later” / “similar to Task N”.

**3. Type consistency:** `TaskListQuery`, `RunListQuery`, `winning_run`, `run_by_session` names match across tasks.

User already chose sequential inline execution — do not ask Subagent-Driven vs Inline.
