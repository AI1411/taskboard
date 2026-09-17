# Cancel a dead running run Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a person close a dead running or waiting run without adding a new status value: `tb run cancel` (and Inspector Cancel) ends the run as `failed` with default summary `canceled`.

**Architecture:** `App::run_cancel` validates the run is `running` or `waiting`, then reuses `run_fail_inner` with summary `canceled` (or an explicit `--summary`). No new `RunStatus`. HTTP/desktop add `RunOp::Cancel`. Inspector shows Cancel on open run rows, including stale-badged cards.

**Tech Stack:** Rust App/CLI/HTTP, React Inspector, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing or canceling a run does not move the card
- Do not add a new run status; terminal status stays `failed`
- Default summary is `canceled`
- Do not auto-fail on stale; do not launch or restart agents from the app
- CLI `--json` snake_case; HTTP camelCase
- No OS notifications

## File map

- Modify: `crates/application/src/commands.rs`, `lib.rs`, `app.rs`
- Create: `crates/application/tests/run_cancel.rs`
- Modify: `crates/cli/src/args.rs`, `main.rs`, `mcp.rs`
- Modify: `crates/cli/tests/cli_json.rs`, `mcp.rs`
- Modify: `crates/api/src/dto.rs`, `routes.rs`
- Modify: `crates/desktop-commands/src/dto.rs`, `commands.rs`
- Modify: `packages/types/src/index.ts`
- Modify: `packages/ui/src/Inspector.tsx`, `Inspector.test.tsx`, `TaskboardApp.tsx`, `fakeTransport.ts`

User already chose sequential inline execution.

---

### Task 1: App::run_cancel

**Files:**
- Modify: `crates/application/src/commands.rs`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/application/src/app.rs`
- Create: `crates/application/tests/run_cancel.rs`

**Interfaces:**
- Consumes: `run_fail_inner`, `require_run`, `RunFail`, `RunStatus`
- Produces:
  - `pub struct RunCancel { run_display_id: String, summary: Option<String>, revision: Option<i64> }`
  - `App::run_cancel(actor, cmd) -> Result<Run, AppError>`
  - Running or waiting → `RunStatus::Failed`, `ended_at` set, summary `canceled` unless `--summary` given
  - Completed or already failed → `validation_error` field `status`, message `run must be running or waiting`
  - Blank explicit summary → `validation_error` field `summary`
  - Column does not move
  - Activity stays `run.fail` (reuse `run_fail_inner`)

- [x] **Step 1: Write failing tests**

Create `crates/application/tests/run_cancel.rs` using the same `TestApp` / `cli_actor` / `test_app` pattern as `run_continue.rs`:

```rust
use std::ops::Deref;

use taskboard_application::{
    Actor, App, ProjectAdd, RunCancel, RunFail, RunFinish, RunStart, RunWait, SystemClock,
    TaskCreate,
};
use taskboard_core::{ActorKind, CardDisplayStatus, Column, RunStatus};
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

async fn seeded_running() -> TestApp {
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
            title: "Stuck".into(),
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
    app
}

#[tokio::test]
async fn cancel_running_ends_as_failed_with_default_summary() {
    let app = seeded_running().await;
    let run = app
        .run_cancel(
            &cli_actor(),
            RunCancel {
                run_display_id: "RUN-1".into(),
                summary: None,
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.summary.as_deref(), Some("canceled"));
    assert!(run.ended_at.is_some());
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.column, Column::Todo);
    assert_eq!(shown.display_status, CardDisplayStatus::Failed);
}

#[tokio::test]
async fn cancel_waiting_ends_as_failed() {
    let app = seeded_running().await;
    app.run_wait(
        &cli_actor(),
        RunWait {
            run_display_id: "RUN-1".into(),
            reason: "Need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let run = app
        .run_cancel(
            &cli_actor(),
            RunCancel {
                run_display_id: "RUN-1".into(),
                summary: Some("agent died".into()),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.summary.as_deref(), Some("agent died"));
}

#[tokio::test]
async fn cancel_completed_or_failed_is_validation_error() {
    let app = seeded_running().await;
    app.run_finish(
        &cli_actor(),
        RunFinish {
            run_display_id: "RUN-1".into(),
            summary: "done".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let err = app
        .run_cancel(
            &cli_actor(),
            RunCancel {
                run_display_id: "RUN-1".into(),
                summary: None,
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");

    let app = seeded_running().await;
    app.run_fail(
        &cli_actor(),
        RunFail {
            run_display_id: "RUN-1".into(),
            summary: "boom".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let err = app
        .run_cancel(
            &cli_actor(),
            RunCancel {
                run_display_id: "RUN-1".into(),
                summary: None,
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}
```

- [x] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-application --test run_cancel -- --nocapture`

Expected: FAIL — `RunCancel` / `run_cancel` not found.

- [x] **Step 3: Minimal implementation**

`commands.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCancel {
    pub run_display_id: String,
    pub summary: Option<String>,
    pub revision: Option<i64>,
}
```

Export `RunCancel` from `lib.rs`.

`app.rs` `run_cancel` + `run_cancel_inner`:

```rust
    pub async fn run_cancel(&self, actor: &Actor, cmd: RunCancel) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = run_cancel_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }
```

```rust
async fn run_cancel_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: RunCancel,
    now: DateTime<Utc>,
) -> Result<Run, AppError> {
    let run = require_run(store, &cmd.run_display_id).await?;
    if !matches!(run.status, RunStatus::Running | RunStatus::Waiting) {
        return Err(AppError::Validation {
            field: "status".into(),
            message: "run must be running or waiting".into(),
        });
    }
    let summary = match cmd.summary {
        Some(raw) => require_non_blank("summary", &raw)?,
        None => "canceled".into(),
    };
    run_fail_inner(
        store,
        actor,
        RunFail {
            run_display_id: cmd.run_display_id,
            summary,
            revision: cmd.revision,
        },
        now,
    )
    .await
}
```

Import `RunCancel` in `app.rs` `use crate::commands`.

- [x] **Step 4: Run tests**

Run: `cargo test -p taskboard-application --test run_cancel -- --nocapture`

Expected: PASS

- [x] **Step 5: Commit**

```bash
git add crates/application
git commit -m "feat: cancel running or waiting runs as failed"
```

---

### Task 2: CLI + MCP

**Files:**
- Modify: `crates/cli/src/args.rs`, `main.rs`, `mcp.rs`
- Modify: `crates/cli/tests/cli_json.rs`, `mcp.rs`

**Interfaces:**
- `RunCommand::Cancel { run_id: String, summary: Option<String> }` via `tb run cancel RUN-n [--summary "…"]`
- `--json` prints `{ ok, entity: Run }` with `status: failed`, `summary: canceled`
- MCP tool `run_cancel` with `display_id` required, `summary` optional

- [ ] **Step 1: Failing CLI tests**

Append to `crates/cli/tests/cli_json.rs`:

```rust
#[test]
fn run_cancel_json_defaults_summary() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir).args(["project", "add", "--name", "Renai Sim"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "renai-sim", "--title", "Stuck"]).assert().success();
    tb_in(&dir).args(["run", "start", "TASK-1", "--agent", "cursor"]).assert().success();
    let v = json_ok(&dir, &["run", "cancel", "RUN-1"]);
    assert_eq!(v["entity"]["status"], "failed");
    assert_eq!(v["entity"]["summary"], "canceled");
}

#[test]
fn run_cancel_completed_is_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir).args(["project", "add", "--name", "Renai Sim"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "renai-sim", "--title", "Done"]).assert().success();
    tb_in(&dir).args(["run", "start", "TASK-1", "--agent", "cursor"]).assert().success();
    tb_in(&dir).args(["run", "finish", "RUN-1", "--summary", "shipped"]).assert().success();
    let out = tb_in(&dir)
        .args(["run", "cancel", "RUN-1", "--json"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["error"]["code"], "validation_error");
}
```

In `crates/cli/tests/mcp.rs`, add `"run_cancel"` to the required tools list.

- [ ] **Step 2: Run to fail**

Run: `cargo test -p taskboard-cli --test cli_json run_cancel -- --nocapture`

Expected: FAIL — unexpected argument / unrecognized subcommand `cancel`.

- [ ] **Step 3: Implement CLI/MCP**

`args.rs` after `Finish`:

```rust
    /// Cancel a running or waiting run
    Cancel {
        run_id: String,
        #[arg(long)]
        summary: Option<String>,
    },
```

`main.rs` match arm:

```rust
        RunCommand::Cancel { run_id, summary } => {
            let run = app
                .run_cancel(
                    actor,
                    RunCancel {
                        run_display_id: run_id,
                        summary,
                        revision,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entity(json, &run, run.revision, || {
                println!("Canceled {}", run.display_id);
            });
        }
```

Import `RunCancel`.

MCP: add `run_cancel` tool (`display_id` required, `summary` optional) and handler calling `app.run_cancel`.

- [ ] **Step 4: Tests pass**

Run: `cargo test -p taskboard-cli --test cli_json run_cancel -- --nocapture`

Run: `cargo test -p taskboard-cli --test mcp -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit** `feat(cli): add tb run cancel and MCP run_cancel`

---

### Task 3: Inspector Cancel + HTTP/desktop op

**Files:**
- Modify: `crates/api/src/dto.rs` — `RunOp::Cancel`
- Modify: `crates/api/src/routes.rs` — `RunOp::Cancel` calls `run_cancel` (`summary` optional)
- Modify: `crates/desktop-commands/src/dto.rs`, `commands.rs`
- Modify: `packages/types/src/index.ts` — `{ op: "cancel"; summary?: string }`
- Modify: `packages/ui/src/Inspector.tsx` — Cancel on running/waiting rows
- Modify: `packages/ui/src/Inspector.test.tsx`, `TaskboardApp.tsx`, `fakeTransport.ts`

**Interfaces:**
- `onRunCancel?: (runDisplayId: string) => void`
- Cancel button `aria-label` / name `Cancel RUN-n` only when `run.status` is `running` or `waiting`
- `transport.runPatch(id, { op: "cancel" })`
- Stale-badged running cards use the same button (no separate chrome)

- [ ] **Step 1: Failing Inspector tests**

```tsx
  it("cancels a running run from the inspector", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Stuck", column: "todo" });
    const startedAt = "2026-09-16T12:00:00Z";
    task.displayStatus = "running";
    task.runs = [
      {
        id: "r1",
        displayId: "RUN-1",
        taskId: task.id,
        agent: "cursor",
        sessionId: null,
        status: "running",
        message: "implementing",
        waitingReason: null,
        summary: null,
        startedAt,
        endedAt: null,
        revision: 1,
        createdAt: startedAt,
        updatedAt: startedAt,
        worktreePath: null,
        branch: null,
      },
    ];
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Stuck"));
    await userEvent.click(screen.getByRole("button", { name: "Cancel RUN-1" }));
    await waitFor(() =>
      expect(transport.runPatch).toHaveBeenCalledWith("RUN-1", { op: "cancel" }),
    );
  });

  it("cancels a stale-badged running run from the inspector", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Heartbeat", column: "todo" });
    const startedAt = "2026-09-16T12:00:00Z";
    task.displayStatus = "running";
    task.stale = true;
    task.runs = [
      {
        id: "r1",
        displayId: "RUN-2",
        taskId: task.id,
        agent: "cursor",
        sessionId: null,
        status: "running",
        message: null,
        waitingReason: null,
        summary: null,
        startedAt,
        endedAt: null,
        revision: 1,
        createdAt: startedAt,
        updatedAt: startedAt,
        worktreePath: null,
        branch: null,
      },
    ];
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Heartbeat"));
    await userEvent.click(screen.getByRole("button", { name: "Cancel RUN-2" }));
    await waitFor(() =>
      expect(transport.runPatch).toHaveBeenCalledWith("RUN-2", { op: "cancel" }),
    );
  });
```

- [ ] **Step 2: Fail** — Cancel button missing / `runPatch` not implemented on fakeTransport.

- [ ] **Step 3: Implement**

Inspector run row: if status is `running` or `waiting`, render `<button type="button" className={styles.linkButton} onClick={() => props.onRunCancel?.(run.displayId)}>Cancel</button>` with `aria-label={`Cancel ${run.displayId}`}`.

`TaskboardApp`:

```tsx
        onRunCancel={(runDisplayId) => {
          void (async () => {
            try {
              await transport.runPatch(runDisplayId, { op: "cancel" });
              const id = selectedIdRef.current;
              if (id) applyDetail(await transport.taskShow(id));
              const project = selectedProjectRef.current;
              if (project) await refreshTasks(project.slug);
            } catch (err) {
              setToast({ message: errorMessage(err), error: true });
            }
          })();
        }}
```

`fakeTransport.runPatch`: if `op.op === "cancel"`, set matching run `status` to `failed`, `summary` to `op.summary ?? "canceled"`, `displayStatus` to `failed`.

HTTP/desktop `RunOp::Cancel` → `app.run_cancel` with optional summary.

- [ ] **Step 4:** `pnpm --filter @taskboard/ui test` and `cargo test -p taskboard-desktop-commands -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit** `feat(ui): cancel open runs from the inspector`

---

## Self-review

**1. Spec coverage:** cancel running/waiting; default summary `canceled`; status stays `failed`; Inspector Cancel; stale-badged cards use the same action; no new status; no agent launch; snake_case JSON.

**2. Placeholder scan:** none

**3. Types:** `RunCancel`, `RunOp::Cancel`, Inspector `onRunCancel(runDisplayId)`.
