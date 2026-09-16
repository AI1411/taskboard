# Activity CLI with since

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give agents a read-only `tb activity` poll so they can see board operations outside their own session.

**Architecture:** `App::activity_list` reads `Store::list_activities_after` (already ordered by sequence). It maps each row to an `ActivityEntry` with a human `target` (`TASK-n` / `RUN-n` / project slug). `--project` and `--task` filter in memory after resolving the entity. The command never writes and does not call undo.

**Tech Stack:** Existing Rust crates, clap, `list_activities_after`.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Titles are never task identifiers
- Read-only: no activity insert, no undo change
- CLI `--json` is snake_case
- Prefer CLI/domain; no inspector changes (out of scope: #73)

## File map

- Modify: `crates/core/src/models.rs` — `ActivityEntry`
- Modify: `crates/core/src/lib.rs` — export
- Modify: `crates/application/src/{commands.rs,lib.rs,app.rs}`
- Create: `crates/application/tests/activity_list.rs`
- Modify: `crates/cli/src/{args.rs,main.rs,output.rs}`
- Modify: `crates/cli/tests/{cli_json.rs,help.rs}`

---

### Task 1: `App::activity_list`

**Files:**
- Modify: `crates/core/src/models.rs`, `lib.rs`
- Modify: `crates/application/src/commands.rs`, `lib.rs`, `app.rs`
- Create: `crates/application/tests/activity_list.rs`

**Interfaces:**
- Consumes: `Store::list_activities_after`, `get_project` / `get_task` / `get_run` / `get_link`
- Produces:
  - `pub struct ActivityQuery { pub after: i64, pub project: Option<String>, pub task_display_id: Option<String> }`
  - `pub struct ActivityEntry { pub sequence: i64, pub created_at: DateTime<Utc>, pub actor: String, pub operation: String, pub target: String }`
  - `App::activity_list(&self, query: ActivityQuery) -> Result<Vec<ActivityEntry>, AppError>`
  - Default `after` is `0` (all rows with `sequence > 0`, i.e. everything)
  - Target: project → `slug`; task → `display_id`; run → `display_id`; link → parent task `display_id` (fallback link uuid)
  - `--project` keeps rows whose resolved project slug matches (project itself, or task/run/link in that project)
  - `--task` keeps rows for that task or its runs/links
  - Missing project/task filter target → `not_found`
  - Does not begin a write transaction

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn activity_list_returns_create_rows_in_sequence() {
    let app = seeded_task().await;
    let rows = app
        .activity_list(ActivityQuery {
            after: 0,
            project: None,
            task_display_id: None,
        })
        .await
        .unwrap();
    assert!(rows.len() >= 2);
    assert_eq!(rows[0].operation, "project.add");
    assert_eq!(rows[0].target, "renai-sim");
    assert_eq!(rows[1].operation, "task.create");
    assert_eq!(rows[1].target, "TASK-1");
    assert_eq!(rows[1].actor, "local-cli");
}

#[tokio::test]
async fn activity_list_filters_after_project_and_task() {
    let app = seeded_task().await;
    app.project_add(
        &cli_actor(),
        ProjectAdd {
            name: "Other".into(),
            repo_path: None,
            slug: None,
        },
    )
    .await
    .unwrap();
    let after_create = app.activity_head().await.unwrap();
    app.task_note_set(&cli_actor(), "TASK-1", "# Spec".into(), None)
        .await
        .unwrap();
    let after = app
        .activity_list(ActivityQuery {
            after: after_create,
            project: None,
            task_display_id: None,
        })
        .await
        .unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].operation, "task.note.set");
    let by_task = app
        .activity_list(ActivityQuery {
            after: 0,
            project: None,
            task_display_id: Some("TASK-1".into()),
        })
        .await
        .unwrap();
    assert!(by_task.iter().all(|row| row.target == "TASK-1"));
    let by_project = app
        .activity_list(ActivityQuery {
            after: 0,
            project: Some("other".into()),
            task_display_id: None,
        })
        .await
        .unwrap();
    assert!(by_project.iter().all(|row| row.target == "other"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-application --test activity_list -- --nocapture`

Expected: FAIL — `ActivityQuery` / `activity_list` missing.

- [ ] **Step 3: Write minimal implementation**

Add `ActivityEntry` (serde snake_case). Implement `activity_list` as a read: `list_activities_after(query.after)`, map + filter.

- [ ] **Step 4: Run tests**

Run: `cargo test -p taskboard-application --test activity_list --test projects`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat: add activity list query for polling"
```

---

### Task 2: CLI `tb activity`

**Files:**
- Modify: `crates/cli/src/args.rs`, `main.rs`, `output.rs`
- Modify: `crates/cli/tests/cli_json.rs`, `help.rs`

**Interfaces:**
- Consumes: `ActivityQuery`, `activity_list`
- Produces: `tb activity [--after N] [--project slug] [--task TASK-n]`; JSON `entities`; human `SEQUENCE  TIME  ACTOR  OPERATION  TARGET`

- [ ] **Step 1: Write the failing CLI tests**

```rust
#[test]
fn activity_json_lists_and_filters() {
    let dir = tempfile::tempdir().unwrap();
    json_ok(&dir, &["project", "add", "--name", "Renai Sim"]);
    json_ok(&dir, &["task", "create", "--project", "renai-sim", "--title", "Fix"]);
    let all = json_ok(&dir, &["activity"]);
    let entities = all["entities"].as_array().unwrap();
    assert!(entities.iter().any(|row| row["operation"] == "task.create"));
    assert!(entities.iter().any(|row| row["target"] == "TASK-1"));
    let filtered = json_ok(&dir, &["activity", "--task", "TASK-1"]);
    assert!(filtered["entities"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["target"] == "TASK-1"));
}
```

Add `activity` to `help.rs` top-level help.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-cli --test cli_json activity_ -- --nocapture`

Expected: FAIL — unrecognized subcommand `activity`.

- [ ] **Step 3: Write minimal implementation**

Add `Command::Activity { after, project, task }`. Dispatch. `print_activity_list`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p taskboard-cli --test cli_json --test help`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat: add tb activity for read-only polling"
```

---

## Self-review

| Acceptance | Task |
| --- | --- |
| Lists sequence, time, actor, operation, target | Tasks 1–2 |
| `--after`, `--project`, `--task` | Tasks 1–2 |
| `--json` snake_case | Task 2 |
| Does not mutate / change undo | Task 1 (read-only) |

Out of scope: undo changes, inspector activity labels, HTTP for agents, OS notifications.

User already chose sequential inline execution.
