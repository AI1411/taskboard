# `tb status` snapshot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give people and agents one command, `tb status [--project slug] [--json]`, that composes inbox / open runs / stale / ready / in-review / blocked into a single snapshot.

**Architecture:** New `App::status(project)` calls existing `inbox`, `task_query` (`ready`, `blocked`, `column: InReview`), `run_list(open)`, and `stale_list(30)`. No new table. JSON entity is `BoardStatus` (snake_case). Human text prints counts plus up to 3 head rows per group (`TASK-n  status  agent  detail`). Project scope filters inbox and task queries directly; open/stale runs are filtered to tasks in that project.

**Tech Stack:** Existing Rust crates, clap, serde JSON, assert_cmd CLI tests.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- No new entity table
- No activity `--follow`
- No OS notifications
- Inbox membership is not changed here (Stale mixing is #110, already on main)
- CLI `--json` stays snake_case
- Default stale threshold stays 30 minutes
- Head size is 3 rows per group

## File map

- Modify: `crates/application/src/commands.rs` — `BoardStatus`, `StatusCounts`, `StatusLine`
- Modify: `crates/application/src/lib.rs` — export
- Modify: `crates/application/src/app.rs` — `App::status`
- Create: `crates/application/tests/status.rs`
- Modify: `crates/cli/src/args.rs` — `Command::Status`
- Modify: `crates/cli/src/main.rs` — dispatch
- Modify: `crates/cli/src/output.rs` — `print_status`
- Modify: `crates/cli/tests/cli_json.rs`, `crates/cli/tests/help.rs`

User already chose sequential inline execution.

---

### Task 1: `App::status` composes existing queries

**Files:**
- Modify: `crates/application/src/commands.rs`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/application/src/app.rs`
- Create: `crates/application/tests/status.rs`

**Interfaces:**
- Consumes: `inbox`, `task_query`, `run_list`, `stale_list(30)`
- Produces:
  - `pub struct StatusLine { display_id: String, status: String, agent: Option<String>, detail: String }`
  - `pub struct InboxCounts { total: usize, waiting: usize, failed: usize, stale: usize, urgent: usize }`
  - `pub struct BoardStatus { inbox: InboxCounts, inbox_head: Vec<StatusLine>, open_runs: usize, open_run_head: Vec<StatusLine>, stale: usize, stale_head: Vec<StatusLine>, ready: usize, ready_head: Vec<StatusLine>, in_review: usize, in_review_head: Vec<StatusLine>, blocked: usize, blocked_head: Vec<StatusLine> }`
  - `App::status(project: Option<String>) -> Result<BoardStatus, AppError>`
  - Unknown project → `not_found`
  - `STATUS_HEAD: usize = 3`
  - Inbox counts: waiting / failed from `display_status`; stale = `item.stale && !waiting && !failed`; urgent = remainder
  - Task line: `display_id`, `display_status` as string, `agent: None`, `detail` = `run_message` or `waiting_reason` or `title`
  - Run line: mapped to the task’s `TASK-n`, run `status`, `agent`, `detail` = message or waiting_reason or summary or `""`
  - `--project` filters open/stale runs to tasks returned by `task_query` for that slug

- [x] **Step 1: Write the failing application test**

Create `crates/application/tests/status.rs` using the same `TestApp` / `cli_actor` pattern as `inbox.rs` (SystemClock):

```rust
use std::ops::Deref;

use taskboard_application::{
    Actor, App, InboxScope, LinkAdd, ProjectAdd, RunFail, RunStart, RunWait, SystemClock,
    TaskCreate,
};
use taskboard_core::{ActorKind, Column, LinkKind};
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
async fn status_counts_inbox_open_ready_review_blocked() {
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
            title: "Wait me".into(),
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
            title: "Ready me".into(),
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
            title: "Review me".into(),
            column: Some(Column::InReview),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_create(
        &actor,
        TaskCreate {
            project_slug: "renai-sim".into(),
            title: "Blocked me".into(),
            column: None,
            urgent: false,
        },
    )
    .await
    .unwrap();
    let wait = app
        .run_start(
            &actor,
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "codex".into(),
                session_id: None,
            },
        )
        .await
        .unwrap();
    app.run_wait(
        &actor,
        RunWait {
            run_display_id: wait.display_id,
            reason: "Need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    app.link_add(
        &actor,
        LinkAdd {
            task_display_id: "TASK-4".into(),
            kind: LinkKind::BlockedBy,
            value: "TASK-2".into(),
            revision: None,
        },
    )
    .await
    .unwrap();

    let snap = app.status(None).await.unwrap();
    assert_eq!(snap.inbox.total, 1);
    assert_eq!(snap.inbox.waiting, 1);
    assert_eq!(snap.open_runs, 1);
    assert_eq!(snap.ready, 1);
    assert_eq!(snap.in_review, 1);
    assert_eq!(snap.blocked, 1);
    assert_eq!(snap.inbox_head[0].display_id, "TASK-1");
    assert_eq!(snap.inbox_head[0].detail, "Need spec");
    assert_eq!(snap.open_run_head[0].display_id, "TASK-1");
    assert_eq!(snap.open_run_head[0].agent.as_deref(), Some("codex"));
    assert_eq!(snap.ready_head[0].display_id, "TASK-2");
    assert_eq!(snap.in_review_head[0].display_id, "TASK-3");
    assert_eq!(snap.blocked_head[0].display_id, "TASK-4");
}

#[tokio::test]
async fn status_unknown_project_is_not_found() {
    let app = test_app().await;
    let err = app.status(Some("nope".into())).await.unwrap_err();
    assert_eq!(err.code(), "not_found");
}
```

Ready count: TASK-2 is idle/todo with no blocker. TASK-1 is waiting so not ready. TASK-3 is in-review so not ready. TASK-4 is blocked. `ready == 1` (TASK-2). Open run is the waiting RUN-1.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-application --test status -- --nocapture`

Expected: FAIL — `App::status` not found.

- [ ] **Step 3: Write minimal implementation**

In `crates/application/src/commands.rs`:

```rust
pub const STATUS_HEAD: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StatusLine {
    pub display_id: String,
    pub status: String,
    pub agent: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct InboxCounts {
    pub total: usize,
    pub waiting: usize,
    pub failed: usize,
    pub stale: usize,
    pub urgent: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BoardStatus {
    pub inbox: InboxCounts,
    pub inbox_head: Vec<StatusLine>,
    pub open_runs: usize,
    pub open_run_head: Vec<StatusLine>,
    pub stale: usize,
    pub stale_head: Vec<StatusLine>,
    pub ready: usize,
    pub ready_head: Vec<StatusLine>,
    pub in_review: usize,
    pub in_review_head: Vec<StatusLine>,
    pub blocked: usize,
    pub blocked_head: Vec<StatusLine>,
}
```

Add `use serde::{Deserialize, Serialize};` to commands.rs if missing.

Export `BoardStatus`, `InboxCounts`, `StatusLine`, `STATUS_HEAD` from `lib.rs`.

In `app.rs`:

```rust
    pub async fn status(&self, project: Option<String>) -> Result<BoardStatus, AppError> {
        let inbox = self
            .inbox(InboxScope {
                project: project.clone(),
                include_archived: false,
            })
            .await?;
        let ready = self
            .task_query(TaskListQuery {
                project: project.clone(),
                ready: true,
                ..TaskListQuery::default()
            })
            .await?;
        let blocked = self
            .task_query(TaskListQuery {
                project: project.clone(),
                blocked: true,
                ..TaskListQuery::default()
            })
            .await?;
        let in_review = self
            .task_query(TaskListQuery {
                project: project.clone(),
                column: Some(Column::InReview),
                ..TaskListQuery::default()
            })
            .await?;
        let tasks = self
            .task_query(TaskListQuery {
                project: project.clone(),
                ..TaskListQuery::default()
            })
            .await?;
        let task_ids: HashSet<_> = tasks.iter().map(|task| task.id).collect();
        let task_by_id: HashMap<_, _> = tasks.iter().map(|task| (task.id, task)).collect();
        let mut open = self
            .run_list(RunListQuery {
                open: true,
                ..RunListQuery::default()
            })
            .await?;
        let mut stale_runs = self.stale_list(30).await?;
        if project.is_some() {
            open.retain(|run| task_ids.contains(&run.task_id));
            stale_runs.retain(|run| task_ids.contains(&run.task_id));
        }
        let inbox_counts = InboxCounts {
            total: inbox.len(),
            waiting: inbox
                .iter()
                .filter(|item| item.display_status == CardDisplayStatus::Waiting)
                .count(),
            failed: inbox
                .iter()
                .filter(|item| item.display_status == CardDisplayStatus::Failed)
                .count(),
            stale: inbox
                .iter()
                .filter(|item| {
                    item.stale
                        && item.display_status != CardDisplayStatus::Waiting
                        && item.display_status != CardDisplayStatus::Failed
                })
                .count(),
            urgent: inbox
                .iter()
                .filter(|item| {
                    !item.stale
                        && item.display_status != CardDisplayStatus::Waiting
                        && item.display_status != CardDisplayStatus::Failed
                })
                .count(),
        };
        Ok(BoardStatus {
            inbox: inbox_counts,
            inbox_head: inbox
                .iter()
                .take(STATUS_HEAD)
                .map(|item| StatusLine {
                    display_id: item.display_id.clone(),
                    status: format!("{}", item.display_status),
                    agent: None,
                    detail: if item.reason.is_empty() {
                        item.title.clone()
                    } else {
                        item.reason.clone()
                    },
                })
                .collect(),
            open_runs: open.len(),
            open_run_head: open
                .iter()
                .take(STATUS_HEAD)
                .map(|run| run_line(run, &task_by_id))
                .collect(),
            stale: stale_runs.len(),
            stale_head: stale_runs
                .iter()
                .take(STATUS_HEAD)
                .map(|run| run_line(run, &task_by_id))
                .collect(),
            ready: ready.len(),
            ready_head: ready.iter().take(STATUS_HEAD).map(task_line).collect(),
            in_review: in_review.len(),
            in_review_head: in_review.iter().take(STATUS_HEAD).map(task_line).collect(),
            blocked: blocked.len(),
            blocked_head: blocked.iter().take(STATUS_HEAD).map(task_line).collect(),
        })
    }
```

Helpers in `app.rs` (private):

```rust
fn task_line(task: &TaskSummary) -> StatusLine {
    let detail = task
        .waiting_reason
        .as_deref()
        .or(task.run_message.as_deref())
        .filter(|s| !s.is_empty())
        .unwrap_or(task.title.as_str())
        .to_string();
    StatusLine {
        display_id: task.display_id.clone(),
        status: format!("{}", task.display_status),
        agent: None,
        detail,
    }
}

fn run_line(run: &Run, tasks: &HashMap<Uuid, &TaskSummary>) -> StatusLine {
    let display_id = tasks
        .get(&run.task_id)
        .map(|task| task.display_id.clone())
        .unwrap_or_else(|| run.display_id.clone());
    let detail = run
        .waiting_reason
        .as_deref()
        .or(run.message.as_deref())
        .or(run.summary.as_deref())
        .unwrap_or("")
        .to_string();
    StatusLine {
        display_id,
        status: run.status.as_str().to_string(),
        agent: Some(run.agent.clone()),
        detail,
    }
}
```

Check `CardDisplayStatus` / `RunStatus` Display/`as_str`. Use existing `as_str()` if Display is missing:

- If `display_status` has no Display, use a match or `serde_json` — prefer the same string as CLI (`waiting`, `running`, `idle`, `failed`, `completed`).

Look up `impl CardDisplayStatus` / `as_str` and reuse it. If only serde exists, add:

```rust
status: match task.display_status {
    CardDisplayStatus::Idle => "idle",
    CardDisplayStatus::Running => "running",
    CardDisplayStatus::Waiting => "waiting",
    CardDisplayStatus::Failed => "failed",
    CardDisplayStatus::Completed => "completed",
}
.to_string()
```

Import `BoardStatus`, `InboxCounts`, `StatusLine`, `STATUS_HEAD`, `HashMap` already imported.

Need `Serialize` on commands — check if commands.rs already uses serde. If not, add the derive import. Prefer putting the structs in `commands.rs` even if that crate module currently has no serde; alternatively put them in `app.rs`. If `commands.rs` has no serde, add:

```rust
use serde::{Deserialize, Serialize};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-application --test status -- --nocapture`

Expected: PASS. If ready count is not 1, print titles and adjust the assertion to the real ready set (TASK-1 waiting is not ready; TASK-2 should be).

- [ ] **Step 5: Commit**

```bash
git add crates/application/src/commands.rs crates/application/src/lib.rs crates/application/src/app.rs crates/application/tests/status.rs
git commit -m "feat: compose board snapshot in App::status"
```

---

### Task 2: CLI `tb status`

**Files:**
- Modify: `crates/cli/src/args.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/src/output.rs`
- Modify: `crates/cli/tests/cli_json.rs`
- Modify: `crates/cli/tests/help.rs`

**Interfaces:**
- Consumes: `App::status`
- Produces:
  - `Command::Status { project: Option<String> }`
  - `--json` prints `{ ok, entity: BoardStatus }` (no revision required — use `print_entity(..., 0)` or `print_entities` style with a single entity)
  - Human:

```
inbox  1  (waiting 1)
  TASK-1  waiting  -  Need spec
open_runs  1
  TASK-1  waiting  codex  Need spec
stale  0
ready  1
  TASK-2  idle  -  Ready me
in_review  1
  TASK-3  idle  -  Review me
blocked  1
  TASK-4  idle  -  Blocked me
```

  Agent column is `-` when `None`.
  Empty groups still print the count line, omit head rows.

- [ ] **Step 1: Write the failing CLI tests**

In `help.rs` add `assert!(out.contains("status"), ...);`

In `cli_json.rs`:

```rust
#[test]
fn status_json_counts_and_heads() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "Wait me"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "Ready me"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "start", "TASK-1", "--agent", "codex"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "wait", "RUN-1", "--reason", "Need spec"])
        .assert()
        .success();
    let out = tb_in(&dir)
        .args(["status", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["entity"]["inbox"]["waiting"], 1);
    assert_eq!(v["entity"]["open_runs"], 1);
    assert_eq!(v["entity"]["ready"], 1);
    assert_eq!(v["entity"]["inbox_head"][0]["display_id"], "TASK-1");
    assert_eq!(v["entity"]["inbox_head"][0]["detail"], "Need spec");
}

#[test]
fn status_project_scope_json() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "A"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["project", "add", "--name", "B"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "a", "--title", "In A"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "b", "--title", "In B"])
        .assert()
        .success();
    let out = tb_in(&dir)
        .args(["status", "--project", "b", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["entity"]["ready"], 1);
    assert_eq!(v["entity"]["ready_head"][0]["display_id"], "TASK-2");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-cli status_json_counts -- --nocapture`

Expected: FAIL — unexpected argument `status`.

- [ ] **Step 3: Write minimal implementation**

`args.rs` after `Inbox`:

```rust
    /// Board-wide snapshot
    Status {
        #[arg(long)]
        project: Option<String>,
    },
```

`main.rs` match arm:

```rust
        Command::Status { project } => {
            let snap = app
                .status(project)
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &snap, 0, || output::print_status(&snap));
            Ok(())
        }
```

`output.rs`:

```rust
pub fn print_status(snap: &BoardStatus) {
    println!(
        "inbox  {}  ({})",
        snap.inbox.total,
        inbox_parts(&snap.inbox)
    );
    print_status_lines(&snap.inbox_head);
    println!("open_runs  {}", snap.open_runs);
    print_status_lines(&snap.open_run_head);
    println!("stale  {}", snap.stale);
    print_status_lines(&snap.stale_head);
    println!("ready  {}", snap.ready);
    print_status_lines(&snap.ready_head);
    println!("in_review  {}", snap.in_review);
    print_status_lines(&snap.in_review_head);
    println!("blocked  {}", snap.blocked);
    print_status_lines(&snap.blocked_head);
}

fn inbox_parts(counts: &InboxCounts) -> String {
    let parts = [
        (counts.waiting, "waiting"),
        (counts.failed, "failed"),
        (counts.stale, "stale"),
        (counts.urgent, "urgent"),
    ]
    .into_iter()
    .filter(|(n, _)| *n > 0)
    .map(|(n, label)| format!("{label} {n}"))
    .collect::<Vec<_>>();
    if parts.is_empty() {
        "-".into()
    } else {
        parts.join(" · ")
    }
}

fn print_status_lines(lines: &[StatusLine]) {
    for line in lines {
        println!(
            "  {}  {}  {}  {}",
            line.display_id,
            line.status,
            line.agent.as_deref().unwrap_or("-"),
            line.detail
        );
    }
}
```

Import `BoardStatus`, `InboxCounts`, `StatusLine` in output.rs.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-cli status -- --nocapture`

Run: `cargo test -p taskboard-cli help_lists -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/args.rs crates/cli/src/main.rs crates/cli/src/output.rs crates/cli/tests/cli_json.rs crates/cli/tests/help.rs
git commit -m "feat(cli): add tb status snapshot"
```

---

## Self-review

**1. Spec coverage**
- Counts inbox (breakdown) · open runs · stale · ready · in-review · blocked → Task 1
- `--project` → Task 1 filter + Task 2 CLI
- Head rows `TASK-n · status · agent · one-liner` → StatusLine
- Compose existing queries, no new table → `App::status`
- snake_case `--json` → BoardStatus serde

**2. Placeholder scan:** none

**3. Type consistency:** `BoardStatus` / `StatusLine` / `InboxCounts` used in App, CLI output, and JSON tests.
