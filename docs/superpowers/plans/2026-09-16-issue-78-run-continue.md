# Resume waiting runs with `run continue`

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `tb run continue RUN-n [--message "..."]` so a waiting run returns to running on the same `RUN-n` instead of starting a new trail of runs.

**Architecture:** Reuse `mutate_run` the same way `run_wait` / `run_update` do. `App::run_continue` accepts only `RunStatus::Waiting`; it sets `status = Running`, clears `waiting_reason`, leaves `ended_at` unset, and optionally replaces `message` via `parse_run_message`. Failed / completed / running runs stay rejected (`validation_error` on `status`). No HTTP, Tauri, or inspector button.

**Tech Stack:** `taskboard-application`, `taskboard-cli`, clap, existing SQLite store; tokio application tests and `assert_cmd`.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Titles are never task identifiers; mutations still require `TASK-n` / `RUN-n`
- Failed-run restart is still a new `run start` (do not reuse the failed `RUN-n`)
- Inspector must not grow a Continue button
- CLI `--json` is snake_case
- Prefer CLI/domain; keep `packages/ui` untouched
- `--actor` remains global; JSON success is `"ok": true` with `entity` / `revision`

## File map

- Modify: `crates/application/src/commands.rs` — `RunContinue`
- Modify: `crates/application/src/lib.rs` — export `RunContinue`
- Modify: `crates/application/src/app.rs` — `run_continue` + `run_continue_inner`
- Modify: `crates/cli/src/args.rs` — `RunCommand::Continue`
- Modify: `crates/cli/src/main.rs` — dispatch
- Modify: `AGENTS.md` and `skills/using-taskboard/SKILL.md` — wait then `run continue`
- Test: `crates/application/tests/run_continue.rs`
- Test: `crates/cli/tests/cli_json.rs`

---

### Task 1: `App::run_continue`

**Files:**
- Modify: `crates/application/src/commands.rs`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/application/src/app.rs`
- Create: `crates/application/tests/run_continue.rs`

**Interfaces:**
- Consumes: `mutate_run`, `parse_run_message`, `require_run`, `RunWait` / `RunFail` fixtures
- Produces:
  - `pub struct RunContinue { pub run_display_id: String, pub message: Option<String>, pub revision: Option<i64> }`
  - `App::run_continue(&self, actor: &Actor, cmd: RunContinue) -> Result<Run, AppError>`
  - Waiting → `status = Running`, `waiting_reason = None`, `ended_at` unchanged (still `None`)
  - `message: Some` goes through `parse_run_message` (max 500); `None` leaves the current message
  - Non-waiting → `AppError::Validation { field: "status", message: "run must be waiting" }`
  - Activity operation: `"run.continue"`
  - Missing `RUN-n` stays `not_found`

- [ ] **Step 1: Write the failing tests**

Create `crates/application/tests/run_continue.rs`:

```rust
use std::ops::Deref;

use taskboard_application::{
    Actor, App, ProjectAdd, RunContinue, RunFail, RunStart, RunWait, SystemClock, TaskCreate,
};
use taskboard_core::{ActorKind, CardDisplayStatus, RunStatus};
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

async fn seeded_waiting() -> TestApp {
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
            title: "Fix login".into(),
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
            session_id: None,
        },
    )
    .await
    .unwrap();
    app.run_wait(
        &actor,
        RunWait {
            run_display_id: "RUN-1".into(),
            reason: "need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    app
}

#[tokio::test]
async fn continue_moves_waiting_to_running_and_clears_reason() {
    let app = seeded_waiting().await;
    let continued = app
        .run_continue(
            &cli_actor(),
            RunContinue {
                run_display_id: "RUN-1".into(),
                message: Some("back to it".into()),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(continued.display_id, "RUN-1");
    assert_eq!(continued.status, RunStatus::Running);
    assert_eq!(continued.waiting_reason, None);
    assert!(continued.ended_at.is_none());
    assert_eq!(continued.message.as_deref(), Some("back to it"));
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.display_status, CardDisplayStatus::Running);
    assert_eq!(shown.column, taskboard_core::Column::Todo);
}

#[tokio::test]
async fn continue_without_message_keeps_existing_message() {
    let app = seeded_waiting().await;
    app.run_update(
        &cli_actor(),
        taskboard_application::RunUpdate {
            run_display_id: "RUN-1".into(),
            message: Some("paused".into()),
            revision: None,
        },
    )
    .await
    .unwrap();
    let continued = app
        .run_continue(
            &cli_actor(),
            RunContinue {
                run_display_id: "RUN-1".into(),
                message: None,
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(continued.message.as_deref(), Some("paused"));
}

#[tokio::test]
async fn continue_running_or_failed_is_validation_error() {
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
            title: "Fix login".into(),
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
            session_id: None,
        },
    )
    .await
    .unwrap();
    let running_err = app
        .run_continue(
            &actor,
            RunContinue {
                run_display_id: "RUN-1".into(),
                message: None,
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(running_err.code(), "validation_error");
    app.run_fail(
        &actor,
        RunFail {
            run_display_id: "RUN-1".into(),
            summary: "boom".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let failed_err = app
        .run_continue(
            &actor,
            RunContinue {
                run_display_id: "RUN-1".into(),
                message: None,
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(failed_err.code(), "validation_error");
    let restart = app
        .run_start(
            &actor,
            RunStart {
                task_display_id: "TASK-1".into(),
                agent: "cursor".into(),
                session_id: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(restart.display_id, "RUN-2");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-application --test run_continue -- --nocapture`

Expected: FAIL — `RunContinue` / `run_continue` not found.

- [ ] **Step 3: Write minimal implementation**

Add to `crates/application/src/commands.rs` after `RunWait`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunContinue {
    pub run_display_id: String,
    pub message: Option<String>,
    pub revision: Option<i64>,
}
```

Export from `crates/application/src/lib.rs`:

```rust
pub use commands::{
    InboxScope, LinkAdd, ProjectAdd, ProjectUpdate, RunContinue, RunFail, RunFinish, RunListQuery,
    RunStart, RunUpdate, RunWait, TaskCreate, TaskListQuery, TaskUpdate,
};
```

Import `RunContinue` in `crates/application/src/app.rs` and add after `run_wait`:

```rust
    pub async fn run_continue(&self, actor: &Actor, cmd: RunContinue) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = run_continue_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }
```

Add `run_continue_inner` next to `run_wait_inner`:

```rust
async fn run_continue_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: RunContinue,
    now: DateTime<Utc>,
) -> Result<Run, AppError> {
    mutate_run(
        store,
        actor,
        &cmd.run_display_id,
        cmd.revision,
        now,
        "run.continue",
        |run| {
            if run.status != RunStatus::Waiting {
                return Err(AppError::Validation {
                    field: "status".into(),
                    message: "run must be waiting".into(),
                });
            }
            if let Some(message) = cmd.message {
                run.message = Some(parse_run_message(&message)?);
            }
            run.status = RunStatus::Running;
            run.waiting_reason = None;
            Ok(())
        },
    )
    .await
}
```

Do not assign `ended_at`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p taskboard-application --test run_continue --test notes_runs`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/application/src/commands.rs crates/application/src/lib.rs crates/application/src/app.rs crates/application/tests/run_continue.rs docs/superpowers/plans/2026-09-16-issue-78-run-continue.md
git commit -m "feat: resume waiting runs with run continue"
```

---

### Task 2: CLI `run continue`

**Files:**
- Modify: `crates/cli/src/args.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/tests/cli_json.rs`
- Modify: `AGENTS.md`
- Modify: `skills/using-taskboard/SKILL.md`

**Interfaces:**
- Consumes: `RunContinue`, `App::run_continue`
- Produces:
  - `tb run continue RUN-n [--message "..."]`
  - JSON `{ ok, entity, revision }` snake_case
  - Human: `Continued RUN-n`
  - Non-waiting JSON: `"error": { "code": "validation_error", "field": "status" }`

- [ ] **Step 1: Write the failing CLI tests**

Append to `crates/cli/tests/cli_json.rs` before `fn json_ok`:

```rust
#[test]
fn run_continue_json_resumes_waiting() {
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
        .args(["run", "start", "TASK-1", "--agent", "cursor"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["run", "wait", "RUN-1", "--reason", "need spec"])
        .assert()
        .success();
    let continued = json_ok(
        &dir,
        &["run", "continue", "RUN-1", "--message", "got spec"],
    );
    assert_eq!(continued["entity"]["display_id"], "RUN-1");
    assert_eq!(continued["entity"]["status"], "running");
    assert_eq!(continued["entity"]["waiting_reason"], serde_json::Value::Null);
    assert_eq!(continued["entity"]["ended_at"], serde_json::Value::Null);
    assert_eq!(continued["entity"]["message"], "got spec");
}

#[test]
fn run_continue_running_is_validation_error() {
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
        .args(["run", "start", "TASK-1", "--agent", "cursor"])
        .assert()
        .success();
    let out = tb_in(&dir)
        .args(["run", "continue", "RUN-1", "--json"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "validation_error");
    assert_eq!(v["error"]["field"], "status");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-cli --test cli_json run_continue -- --nocapture`

Expected: FAIL — unexpected argument / unknown subcommand `continue`.

- [ ] **Step 3: Write minimal implementation**

In `crates/cli/src/args.rs`, add after `Wait`:

```rust
    /// Resume a waiting run
    Continue {
        run_id: String,
        #[arg(long)]
        message: Option<String>,
    },
```

In `crates/cli/src/main.rs`, import `RunContinue` and add an arm after `Wait`:

```rust
        RunCommand::Continue { run_id, message } => {
            let run = app
                .run_continue(
                    actor,
                    RunContinue {
                        run_display_id: run_id,
                        message,
                        revision,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &run, run.revision, || {
                println!("Continued {}", run.display_id);
            });
        }
```

In `AGENTS.md` and `skills/using-taskboard/SKILL.md`, after the wait sentence, add:

```
When unblocked, `run continue RUN-n [--message "..."]` instead of starting a new run.
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p taskboard-cli --test cli_json --test help`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/args.rs crates/cli/src/main.rs crates/cli/tests/cli_json.rs AGENTS.md skills/using-taskboard/SKILL.md
git commit -m "feat: add tb run continue for waiting runs"
```

---

## Self-review

**1. Spec coverage**

| Acceptance | Task |
| --- | --- |
| waiting → running, clears `waiting_reason` | Task 1 |
| optional `--message` | Tasks 1 + 2 |
| `ended_at` stays unset | Task 1 |
| non-waiting is validation_error | Tasks 1 + 2 |
| failed still needs new `run start` | Task 1 (`RUN-2`) |

Out of scope left out: inspector Continue, `tb work start`, failed-run reuse, HTTP.

**2. Placeholder scan:** no TBD / “add tests later”.

**3. Type consistency:** `RunContinue` fields match CLI dispatch.

User already chose sequential inline execution — do not ask Subagent-Driven vs Inline.
