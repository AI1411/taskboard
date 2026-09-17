# Worktree occupancy lookup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a coordinator see which running or waiting runs share a `worktree_path`, without creating or deleting worktrees or invoking git.

**Architecture:** `App::occupancy` lists open runs, groups them by the parent task’s `worktree_path`, and returns collisions (2+ runs) unless `--path` filters to one directory. Run rows do not persist worktree; occupancy reads the live task field.

**Tech Stack:** Rust App/CLI, clap, assert_cmd.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Do not create or delete worktrees
- Do not invoke git
- Do not paint “shared” on the card
- CLI `--json` snake_case

## File map

- Modify: `crates/application/src/commands.rs`, `lib.rs`, `app.rs`
- Create: `crates/application/tests/occupancy.rs`
- Modify: `crates/cli/src/args.rs`, `main.rs`, `output.rs`
- Modify: `crates/cli/tests/cli_json.rs`

User already chose sequential inline execution.

---

### Task 1: App::occupancy

**Files:**
- Modify: `crates/application/src/commands.rs`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/application/src/app.rs`
- Create: `crates/application/tests/occupancy.rs`

**Interfaces:**
- Consumes: `Store::list_all_runs`, `Store::get_task`
- Produces:
  - `pub struct OccupancyQuery { path: Option<String> }`
  - `pub struct OccupancyRun { run_display_id, task_display_id, status, agent }`
  - `pub struct OccupancyGroup { worktree_path: String, runs: Vec<OccupancyRun> }`
  - `App::occupancy(query) -> Result<Vec<OccupancyGroup>, AppError>`
  - Only `running` / `waiting` runs whose parent task has a `worktree_path`
  - No `--path`: groups with fewer than 2 runs are omitted
  - `--path DIR`: exact match on `task.worktree_path`, include singleton groups
  - Groups sorted by path; runs newest-first as `list_all_runs` already orders

- [ ] **Step 1: Write failing tests**

Create `crates/application/tests/occupancy.rs` with `TestApp` helpers, then:

```rust
async fn seed_open_on(app: &App, title: &str, worktree: &str) -> String {
    let actor = cli_actor();
    let task = app
        .task_create(
            &actor,
            TaskCreate {
                project_slug: "renai-sim".into(),
                title: title.into(),
                column: None,
                urgent: false,
            },
        )
        .await
        .unwrap();
    app.task_update(
        &actor,
        TaskUpdate {
            display_id: task.display_id.clone(),
            title: None,
            worktree_path: Some(Some(worktree.into())),
            branch: None,
            revision: None,
        },
    )
    .await
    .unwrap();
    app.run_start(
        &actor,
        RunStart {
            task_display_id: task.display_id.clone(),
            agent: "cursor".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    task.display_id
}

#[tokio::test]
async fn occupancy_without_path_lists_only_collisions() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(&actor, ProjectAdd { name: "Renai Sim".into(), repo_path: None, slug: None })
        .await
        .unwrap();
    seed_open_on(&app, "A", "/tmp/shared").await;
    seed_open_on(&app, "B", "/tmp/shared").await;
    seed_open_on(&app, "C", "/tmp/alone").await;
    let groups = app.occupancy(OccupancyQuery { path: None }).await.unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].worktree_path, "/tmp/shared");
    assert_eq!(groups[0].runs.len(), 2);
}

#[tokio::test]
async fn occupancy_path_includes_singleton() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(&actor, ProjectAdd { name: "Renai Sim".into(), repo_path: None, slug: None })
        .await
        .unwrap();
    seed_open_on(&app, "Alone", "/tmp/alone").await;
    let none = app.occupancy(OccupancyQuery { path: None }).await.unwrap();
    assert!(none.is_empty());
    let groups = app
        .occupancy(OccupancyQuery { path: Some("/tmp/alone".into()) })
        .await
        .unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].runs[0].task_display_id, "TASK-1");
}

#[tokio::test]
async fn occupancy_ignores_completed_runs() {
    let app = test_app().await;
    let actor = cli_actor();
    app.project_add(&actor, ProjectAdd { name: "Renai Sim".into(), repo_path: None, slug: None })
        .await
        .unwrap();
    seed_open_on(&app, "A", "/tmp/shared").await;
    seed_open_on(&app, "B", "/tmp/shared").await;
    app.run_finish(&actor, RunFinish { run_display_id: "RUN-1".into(), summary: "done".into(), revision: None })
        .await
        .unwrap();
    let groups = app.occupancy(OccupancyQuery { path: None }).await.unwrap();
    assert!(groups.is_empty());
}
```

- [ ] **Step 2: Run** `cargo test -p taskboard-application --test occupancy -- --nocapture`

Expected: FAIL — `OccupancyQuery` / `occupancy` not found.

- [ ] **Step 3: Implement**

```rust
    pub async fn occupancy(&self, query: OccupancyQuery) -> Result<Vec<OccupancyGroup>, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        occupancy_inner(store, query).await
    }
```

`occupancy_inner`: list all runs, keep running/waiting, load each task, skip missing worktree, filter `--path`, group in `BTreeMap`, drop singletons when path is `None`.

- [ ] **Step 4: Tests pass**

- [ ] **Step 5: Commit** `feat: group open runs by worktree occupancy`

---

### Task 2: CLI `tb occupancy`

**Files:**
- Modify: `crates/cli/src/args.rs`, `main.rs`, `output.rs`
- Modify: `crates/cli/tests/cli_json.rs`

**Interfaces:**
- `Command::Occupancy { path: Option<String> }` via `tb occupancy [--path DIR]`
- `--json` prints `{ ok, entities: OccupancyGroup[] }`
- Human: path header, then `RUN-n  TASK-n  status  agent`

- [ ] **Step 1: Failing CLI tests**

```rust
#[test]
fn occupancy_json_lists_collisions() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir).args(["project", "add", "--name", "Renai Sim"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "renai-sim", "--title", "A"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "renai-sim", "--title", "B"]).assert().success();
    tb_in(&dir).args(["task", "update", "TASK-1", "--worktree", "/tmp/shared"]).assert().success();
    tb_in(&dir).args(["task", "update", "TASK-2", "--worktree", "/tmp/shared"]).assert().success();
    tb_in(&dir).args(["run", "start", "TASK-1", "--agent", "cursor"]).assert().success();
    tb_in(&dir).args(["run", "start", "TASK-2", "--agent", "cursor"]).assert().success();
    let v = json_ok(&dir, &["occupancy"]);
    assert_eq!(v["entities"][0]["worktree_path"], "/tmp/shared");
    assert_eq!(v["entities"][0]["runs"].as_array().unwrap().len(), 2);
}

#[test]
fn occupancy_path_json_includes_singleton() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir).args(["project", "add", "--name", "Renai Sim"]).assert().success();
    tb_in(&dir).args(["task", "create", "--project", "renai-sim", "--title", "Alone"]).assert().success();
    tb_in(&dir).args(["task", "update", "TASK-1", "--worktree", "/tmp/alone"]).assert().success();
    tb_in(&dir).args(["run", "start", "TASK-1", "--agent", "cursor"]).assert().success();
    let empty = json_ok(&dir, &["occupancy"]);
    assert_eq!(empty["entities"].as_array().unwrap().len(), 0);
    let v = json_ok(&dir, &["occupancy", "--path", "/tmp/alone"]);
    assert_eq!(v["entities"][0]["runs"][0]["task_display_id"], "TASK-1");
}
```

Confirm `task update --worktree` exists (it does from #84).

- [ ] **Step 2: Fail** — unrecognized command `occupancy`.

- [ ] **Step 3: Implement CLI** `print_occupancy` + `Command::Occupancy`.

- [ ] **Step 4: Tests pass**

- [ ] **Step 5: Commit** `feat(cli): add tb occupancy`

---

## Self-review

**1. Spec coverage:** group running/waiting by worktree; `--path` filter; omit `--path` is collisions only; no git; no card chrome.

**2. Placeholder scan:** none

**3. Types:** `OccupancyQuery`, `OccupancyGroup`, `OccupancyRun`.
