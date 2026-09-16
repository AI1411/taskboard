# `tb task spawn` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a coordinator split an existing card into children with one command: create todo/idle children in the same project and mark the parent `blocked-by` each child.

**Architecture:** `App::task_spawn` runs in one transaction: for each title, `task_create_inner` (todo, no worktree/branch) then `link_add_inner` on the parent with `LinkKind::BlockedBy` = child `TASK-n`. Columns do not move. No fifth column. `task show` already exposes `blocks`.

**Tech Stack:** Existing Rust crates, clap, assert_cmd.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Do not copy worktree / branch
- Do not auto-move the parent when children complete
- No fifth column / parent-child column
- CLI `--json` stays snake_case

## File map

- Modify: `crates/application/src/commands.rs` — `TaskSpawn`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/application/src/app.rs` — `App::task_spawn`
- Create: `crates/application/tests/spawn.rs`
- Modify: `crates/cli/src/args.rs`, `main.rs`
- Modify: `crates/cli/tests/cli_json.rs`

User already chose sequential inline execution.

---

### Task 1: `App::task_spawn`

**Files:**
- Modify: `crates/application/src/commands.rs`
- Modify: `crates/application/src/lib.rs`
- Modify: `crates/application/src/app.rs`
- Create: `crates/application/tests/spawn.rs`

**Interfaces:**
- Consumes: `task_create_inner`, `link_add_inner`, `require_live_task`, `Store::get_project`
- Produces:
  - `pub struct TaskSpawn { parent_display_id: String, titles: Vec<String> }`
  - `App::task_spawn(actor, cmd) -> Result<Vec<TaskDetail>, AppError>`
  - Empty titles → `validation_error` field `title`
  - Missing parent → `not_found`
  - Each child: same project slug, `column: Todo`, `urgent: false`, `worktree_path: None`, `branch: None`
  - Parent `blocked_by` contains each child `TASK-n`; parent `blocks` lists them via existing block index
  - Parent column unchanged

- [ ] **Step 1: Write the failing test**

Create `crates/application/tests/spawn.rs` using the same `TestApp` / `cli_actor` / `test_app` pattern as `next.rs`, then:

```rust
#[tokio::test]
async fn spawn_creates_todo_children_and_blocks_parent() {
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
            title: "Parent".into(),
            column: Some(Column::InProgress),
            urgent: false,
        },
    )
    .await
    .unwrap();
    app.task_update(
        &actor,
        TaskUpdate {
            display_id: "TASK-1".into(),
            title: None,
            worktree_path: Some(Some("/tmp/parent-wt".into())),
            branch: Some(Some("feat/parent".into())),
            revision: None,
        },
    )
    .await
    .unwrap();
    let children = app
        .task_spawn(
            &actor,
            TaskSpawn {
                parent_display_id: "TASK-1".into(),
                titles: vec!["API contract".into(), "UI slice".into()],
            },
        )
        .await
        .unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].display_id, "TASK-2");
    assert_eq!(children[1].display_id, "TASK-3");
    for child in &children {
        assert_eq!(child.column, Column::Todo);
        assert_eq!(child.display_status, CardDisplayStatus::Idle);
        assert_eq!(child.worktree_path, None);
        assert_eq!(child.branch, None);
        assert_eq!(child.project_id, children[0].project_id);
    }
    let parent = app.task_show("TASK-1").await.unwrap();
    assert_eq!(parent.column, Column::InProgress);
    assert_eq!(parent.worktree_path.as_deref(), Some("/tmp/parent-wt"));
    assert!(parent.blocked_by.contains(&"TASK-2".into()));
    assert!(parent.blocked_by.contains(&"TASK-3".into()));
    assert!(parent.blocks.contains(&"TASK-2".into()));
    assert!(parent.blocks.contains(&"TASK-3".into()));
}

#[tokio::test]
async fn spawn_empty_titles_is_validation_error() {
    let app = test_app().await;
    let err = app
        .task_spawn(
            &cli_actor(),
            TaskSpawn {
                parent_display_id: "TASK-1".into(),
                titles: vec![],
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}

#[tokio::test]
async fn spawn_missing_parent_is_not_found() {
    let app = test_app().await;
    let err = app
        .task_spawn(
            &cli_actor(),
            TaskSpawn {
                parent_display_id: "TASK-9".into(),
                titles: vec!["Child".into()],
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "not_found");
}
```

Import `TaskSpawn`, `TaskUpdate`, `CardDisplayStatus`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-application --test spawn -- --nocapture`

Expected: FAIL — `TaskSpawn` not found.

- [ ] **Step 3: Write minimal implementation**

`commands.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpawn {
    pub parent_display_id: String,
    pub titles: Vec<String>,
}
```

Export from `lib.rs`.

`app.rs`:

```rust
    pub async fn task_spawn(&self, actor: &Actor, cmd: TaskSpawn) -> Result<Vec<TaskDetail>, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = task_spawn_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }
```

```rust
async fn task_spawn_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: TaskSpawn,
    now: DateTime<Utc>,
) -> Result<Vec<TaskDetail>, AppError> {
    if cmd.titles.is_empty() {
        return Err(AppError::Validation {
            field: "title".into(),
            message: "must not be empty".into(),
        });
    }
    let parent = require_live_task(store, &cmd.parent_display_id).await?;
    let project = store
        .get_project(parent.project_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "project".into(),
            id: parent.project_id.to_string(),
        })?;
    let mut children = Vec::new();
    for title in cmd.titles {
        let child = task_create_inner(
            store,
            actor,
            TaskCreate {
                project_slug: project.slug.clone(),
                title,
                column: None,
                urgent: false,
            },
            now,
        )
        .await?;
        link_add_inner(
            store,
            actor,
            LinkAdd {
                task_display_id: parent.display_id.clone(),
                kind: LinkKind::BlockedBy,
                value: child.display_id.clone(),
                revision: None,
            },
            now,
        )
        .await?;
        children.push(child);
    }
    Ok(children)
}
```

Import `TaskSpawn`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-application --test spawn -- --nocapture`

Expected: PASS. If `blocks` is empty, print `parent` and fix the blocked-by direction (parent must be blocked by the child so `blocked_by` is child ids and `blocks` on the parent lists children — confirm `block_lists` orientation). If `blocks` is on the child instead, the issue still wants `task show` parent to list children via `blocks`. That means the link lives on the parent as blocked-by child, and the reverse index fills `parent.blocks`. If the current model puts `blocks` on the blocker, invert: link the child `blocked-by` parent would be wrong. Keep parent `blocked-by` child as specified; assert whichever of `blocked_by` / `blocks` the existing index fills, but the acceptance text says parent `blocks` lists children. If that fails, check `block_lists` and keep the product meaning: parent is blocked until children finish.

- [ ] **Step 5: Commit**

```bash
git add crates/application/src/commands.rs crates/application/src/lib.rs crates/application/src/app.rs crates/application/tests/spawn.rs
git commit -m "feat: spawn child tasks that block the parent"
```

---

### Task 2: CLI `tb task spawn`

**Files:**
- Modify: `crates/cli/src/args.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/tests/cli_json.rs`

**Interfaces:**
- Consumes: `App::task_spawn`
- Produces: `TaskCommand::Spawn { display_id: String, titles: Vec<String> }` with `--title` required, `num_args = 1..` via repeated `--title`
- `--json` prints `{ ok, entities: [child, ...] }`
- Human: one `Created TASK-n … [todo]` line per child (reuse `created_task`)

- [ ] **Step 1: Write the failing CLI test**

```rust
#[test]
fn task_spawn_json_creates_children_and_blocks() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "Parent"])
        .assert()
        .success();
    let v = json_ok(
        &dir,
        &["task", "spawn", "TASK-1", "--title", "API", "--title", "UI"],
    );
    assert_eq!(v["entities"].as_array().unwrap().len(), 2);
    assert_eq!(v["entities"][0]["display_id"], "TASK-2");
    assert_eq!(v["entities"][0]["column"], "todo");
    assert_eq!(v["entities"][0]["worktree_path"], serde_json::Value::Null);
    let parent = json_ok(&dir, &["task", "show", "TASK-1"]);
    let blocks = parent["entity"]["blocks"].as_array().unwrap();
    let blocked_by = parent["entity"]["blocked_by"].as_array().unwrap();
    assert!(blocks.iter().any(|id| id == "TASK-2"));
    assert!(blocks.iter().any(|id| id == "TASK-3"));
    assert!(blocked_by.iter().any(|id| id == "TASK-2"));
    assert_eq!(parent["entity"]["column"], "todo");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-cli --test cli_json task_spawn -- --nocapture`

Expected: FAIL — unrecognized subcommand `spawn`.

- [ ] **Step 3: Write minimal implementation**

`args.rs` after `Create`:

```rust
    /// Create child tasks and block the parent on them
    Spawn {
        display_id: String,
        #[arg(long = "title", required = true, num_args = 1..)]
        titles: Vec<String>,
    },
```

`task_cmd` arm:

```rust
        TaskCommand::Spawn { display_id, titles } => {
            let children = app
                .task_spawn(
                    actor,
                    TaskSpawn {
                        parent_display_id: display_id,
                        titles,
                    },
                )
                .await
                .map_err(|err| output::print_error(&err, json))?;
            output::print_entities(json, &children, || {
                for child in &children {
                    output::created_task(child);
                }
            });
        }
```

Import `TaskSpawn`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-cli --test cli_json task_spawn -- --nocapture`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/args.rs crates/cli/src/main.rs crates/cli/tests/cli_json.rs
git commit -m "feat(cli): add tb task spawn"
```

---

## Self-review

**1. Spec coverage**
- spawn one / many titles — Task 1 + 2
- parent blocked-by child — `link_add_inner`
- children todo/idle, no worktree/branch copy — Task 1 assertions
- `task show` `blocks` — Task 1 + 2
- no column movement — parent stays InProgress / todo

**2. Placeholder scan:** none

**3. Type consistency:** `TaskSpawn` used in App and CLI.
