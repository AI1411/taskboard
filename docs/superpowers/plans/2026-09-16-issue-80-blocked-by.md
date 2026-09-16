# Task blocked-by relationships

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Store directed `blocked-by` relationships between cards, reject cycles, filter `--blocked` / `--ready`, and show `Blocked by` / `Blocks` on the card face without a fifth column or auto-moves.

**Architecture:** Reuse the existing `links` table. Add `LinkKind::BlockedBy` (`blocked_by`) whose `value` is the blocker `TASK-n`. `App::link_add` validates the target exists, rejects self-edges and cycles, and does not change columns. `TaskSummary.blocked_by` / `blocks` are display-id lists computed from all blocked-by links. A blocker is **active** when the target is live and not in `done`. `--blocked` keeps cards with an active blocker. `--ready` keeps idle or todo cards with no active blocker. Removing the link (existing `link remove`) clears the block.

**Tech Stack:** Existing Rust crates, clap, SQLite `links.kind` TEXT (no CHECK), React card face, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Titles are never task identifiers
- Columns do not auto-move when a blocker finishes
- Inbox membership unchanged
- Existing URL/path links stay valid and unchanged
- CLI `--json` is snake_case; HTTP JSON is camelCase
- Prefer CLI/domain; inspector already lists links — card face is the new UI

## File map

- Modify: `crates/core/src/models.rs` — `LinkKind::BlockedBy`, `TaskSummary.blocked_by` / `blocks`
- Modify: `crates/application/src/{store.rs,commands.rs,app.rs}`
- Modify: `crates/store-sqlite/src/store.rs` — `list_all_links`
- Create: `crates/application/tests/blocked_by.rs`
- Modify: `crates/cli/src/{args.rs,main.rs}`
- Modify: `crates/cli/tests/cli_json.rs`
- Modify: `crates/api/src/dto.rs`, `crates/desktop-commands/src/dto.rs`
- Modify: `packages/types/src/index.ts`, `packages/ui/src/{summary.ts,fakeTransport.ts,Card.tsx,Card.test.tsx}`

---

### Task 1: LinkKind, summaries, cycle-safe `link_add`

**Files:**
- Modify: `crates/core/src/models.rs`
- Modify: `crates/application/src/store.rs`
- Modify: `crates/store-sqlite/src/store.rs`
- Modify: `crates/application/src/commands.rs`
- Modify: `crates/application/src/app.rs`
- Modify: `crates/api/src/dto.rs`, `crates/desktop-commands/src/dto.rs`
- Create: `crates/application/tests/blocked_by.rs`

**Interfaces:**
- Consumes: `Link`, `LinkAdd`, `list_links`, `require_live_task`
- Produces:
  - `LinkKind::BlockedBy` serializes as `blocked_by`
  - `TaskSummary.blocked_by: Vec<String>` and `TaskSummary.blocks: Vec<String>` (display ids)
  - `Store::list_all_links() -> Vec<Link>`
  - `TaskListQuery.blocked: bool` and `TaskListQuery.ready: bool` (mutually exclusive at CLI; both false = no extra filter)
  - Active blocker = live target whose `column != Done`
  - Cycle / self-edge → `validation_error` field `blocked_by`
  - Missing blocker → `not_found` entity `task`
  - URL/path `link_add` unchanged

- [ ] **Step 1: Write the failing tests**

Create `crates/application/tests/blocked_by.rs` using the same `TestApp` helper as `comments.rs` (two tasks `TASK-1` / `TASK-2` plus a third when testing cycles):

```rust
#[tokio::test]
async fn blocked_by_stores_directed_link_and_does_not_move() {
    let app = seeded_two_tasks().await;
    let detail = app
        .link_add(
            &cli_actor(),
            LinkAdd {
                task_display_id: "TASK-2".into(),
                kind: LinkKind::BlockedBy,
                value: "TASK-1".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(detail.column, Column::Todo);
    assert_eq!(detail.links[0].kind, LinkKind::BlockedBy);
    assert_eq!(detail.links[0].value, "TASK-1");
    let listed = app.task_list("renai-sim").await.unwrap();
    let one = listed.iter().find(|t| t.display_id == "TASK-1").unwrap();
    let two = listed.iter().find(|t| t.display_id == "TASK-2").unwrap();
    assert_eq!(two.blocked_by, vec!["TASK-1".to_string()]);
    assert_eq!(one.blocks, vec!["TASK-2".to_string()]);
    assert!(one.blocked_by.is_empty());
}

#[tokio::test]
async fn blocked_by_cycle_is_validation_error() {
    let app = seeded_two_tasks().await;
    app.link_add(
        &cli_actor(),
        LinkAdd {
            task_display_id: "TASK-2".into(),
            kind: LinkKind::BlockedBy,
            value: "TASK-1".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let err = app
        .link_add(
            &cli_actor(),
            LinkAdd {
                task_display_id: "TASK-1".into(),
                kind: LinkKind::BlockedBy,
                value: "TASK-2".into(),
                revision: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}

#[tokio::test]
async fn task_query_blocked_and_ready() {
    let app = seeded_two_tasks().await;
    app.link_add(
        &cli_actor(),
        LinkAdd {
            task_display_id: "TASK-2".into(),
            kind: LinkKind::BlockedBy,
            value: "TASK-1".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    let blocked = app
        .task_query(TaskListQuery {
            project: Some("renai-sim".into()),
            blocked: true,
            ..TaskListQuery::default()
        })
        .await
        .unwrap();
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0].display_id, "TASK-2");
    let ready = app
        .task_query(TaskListQuery {
            project: Some("renai-sim".into()),
            ready: true,
            ..TaskListQuery::default()
        })
        .await
        .unwrap();
    assert!(ready.iter().any(|t| t.display_id == "TASK-1"));
    assert!(ready.iter().all(|t| t.display_id != "TASK-2"));
}

#[tokio::test]
async fn removing_blocked_by_clears_block_without_moving() {
    let app = seeded_two_tasks().await;
    let added = app
        .link_add(
            &cli_actor(),
            LinkAdd {
                task_display_id: "TASK-2".into(),
                kind: LinkKind::BlockedBy,
                value: "TASK-1".into(),
                revision: None,
            },
        )
        .await
        .unwrap();
    let link_id = added.links[0].id;
    let removed = app.link_remove(&cli_actor(), link_id, None).await.unwrap();
    assert!(removed.links.is_empty());
    assert_eq!(removed.column, Column::Todo);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-application --test blocked_by -- --nocapture`

Expected: FAIL — `LinkKind::BlockedBy` / `blocked_by` field missing.

- [ ] **Step 3: Write minimal implementation**

Add `BlockedBy` to `LinkKind`. Add `blocked_by` / `blocks` on `TaskSummary` (and DTO constructors). Add `list_all_links` (same row mapping as `list_links`, no task filter, order `task_id, sort_order, id`).

In `link_add_inner`, for `BlockedBy`:

```rust
        LinkKind::BlockedBy => {
            let blocker = require_live_task(store, &cmd.value).await?;
            if blocker.display_id == task.display_id {
                return Err(AppError::Validation {
                    field: "blocked_by".into(),
                    message: "a task cannot block itself".into(),
                });
            }
            let links = store.list_all_links().await?;
            if would_cycle(&links, &task.display_id, &blocker.display_id, store).await? {
                return Err(AppError::Validation {
                    field: "blocked_by".into(),
                    message: "blocked-by cycle".into(),
                });
            }
            blocker.display_id
        }
```

Cycle walk: from the proposed blocker, follow existing `blocked_by` values (resolve each value to that task's outgoing blocked-by links). If you reach the blocked card, reject. Build a `HashMap<String, Vec<String>>` of `task.display_id -> blocker display ids` after loading tasks for link.task_id.

`to_task_summary_from_runs` also takes `blocked_by` / `blocks` slices (or compute in a helper `block_lists(task, all_links, tasks_by_id)`). Active blocker = target live and `column != Done`.

`task_query`: load `list_all_links` once; after building each summary, filter `--blocked` / `--ready`. Ready = `(display_status == Idle || column == Todo) && blocked_by has no active blocker`.

Every `TaskSummary {` literal must set the new vecs.

- [ ] **Step 4: Run tests**

Run: `cargo test -p taskboard-application --test blocked_by --test notes_runs --test tasks`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat: store blocked-by links and reject cycles"
```

---

### Task 2: CLI `--blocked-by`, `--blocked`, `--ready`

**Files:**
- Modify: `crates/cli/src/args.rs`, `main.rs`
- Modify: `crates/cli/tests/cli_json.rs`

**Interfaces:**
- Consumes: `LinkKind::BlockedBy`, `TaskListQuery.{blocked,ready}`
- Produces:
  - `tb link add TASK-n --blocked-by TASK-m`
  - `tb task list --blocked` / `tb task list --ready`
  - `--blocked` conflicts with `--ready`
  - URL/path add still works

- [ ] **Step 1: Write the failing CLI tests**

```rust
#[test]
fn link_add_blocked_by_json_and_filters() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "Blocker"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "Blocked"])
        .assert()
        .success();
    let added = json_ok(&dir, &["link", "add", "TASK-2", "--blocked-by", "TASK-1"]);
    assert_eq!(added["entity"]["links"][0]["kind"], "blocked_by");
    assert_eq!(added["entity"]["links"][0]["value"], "TASK-1");
    assert_eq!(added["entity"]["column"], "todo");
    let blocked = json_ok(&dir, &["task", "list", "--project", "renai-sim", "--blocked"]);
    assert_eq!(blocked["entities"].as_array().unwrap().len(), 1);
    assert_eq!(blocked["entities"][0]["display_id"], "TASK-2");
    assert_eq!(blocked["entities"][0]["blocked_by"][0], "TASK-1");
    let ready = json_ok(&dir, &["task", "list", "--project", "renai-sim", "--ready"]);
    let ids: Vec<_> = ready["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["display_id"].as_str().unwrap().to_string())
        .collect();
    assert!(ids.contains(&"TASK-1".to_string()));
    assert!(!ids.contains(&"TASK-2".to_string()));
}

#[test]
fn link_add_blocked_by_cycle_is_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "A"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "B"])
        .assert()
        .success();
    json_ok(&dir, &["link", "add", "TASK-2", "--blocked-by", "TASK-1"]);
    let out = tb_in(&dir)
        .args(["link", "add", "TASK-1", "--blocked-by", "TASK-2", "--json"])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "validation_error");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-cli --test cli_json link_add_blocked -- --nocapture`

Expected: FAIL — unrecognized argument `--blocked-by`.

- [ ] **Step 3: Write minimal implementation**

Add `--blocked-by` to the clap `target` group. Dispatch `LinkKind::BlockedBy`. Add `--blocked` / `--ready` on `task list` with `conflicts_with` each other. Thread into `TaskListQuery`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p taskboard-cli --test cli_json --test help`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat: add tb link add --blocked-by and task list filters"
```

---

### Task 3: Card face `Blocked by` / `Blocks`

**Files:**
- Modify: `packages/types/src/index.ts`
- Modify: `packages/ui/src/{summary.ts,fakeTransport.ts,Card.tsx,Card.test.tsx}`

**Interfaces:**
- Consumes: `TaskSummary.blockedBy`, `TaskSummary.blocks`
- Produces: card face text `Blocked by TASK-8` and `Blocks TASK-14` (comma-join multiple ids)

- [ ] **Step 1: Write the failing card test**

```tsx
  it("shows Blocked by and Blocks labels", () => {
    const { getByText, rerender } = render(
      <Card task={summary({ blockedBy: ["TASK-8"] })} selected={false} />,
    );
    expect(getByText("Blocked by TASK-8")).toBeTruthy();
    rerender(<Card task={summary({ blocks: ["TASK-14"] })} selected={false} />);
    expect(getByText("Blocks TASK-14")).toBeTruthy();
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @taskboard/ui test Card`

Expected: FAIL — type / text missing.

- [ ] **Step 3: Write minimal implementation**

Add `blockedBy` / `blocks` string arrays on `TaskSummary`. Render both lines when non-empty. Seed empty arrays in `summary()` and `fakeTransport`.

- [ ] **Step 4: Run tests**

Run: `pnpm --filter @taskboard/ui test Card` and `cargo test -p taskboard-api --test routes`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat: show blocked-by relationships on the card face"
```

---

## Self-review

| Acceptance | Task |
| --- | --- |
| `tb link add TASK-n --blocked-by TASK-m`; cycles error | Tasks 1–2 |
| Both cards show `Blocked by` / `Blocks` | Tasks 1 + 3 |
| `tb task list --blocked` / `--ready` | Tasks 1–2 |
| Removing the link clears the block; columns stay put | Task 1 |
| URL/path links unchanged | Tasks 1–2 |

Out of scope: fifth column, auto-move on blocker finish, inbox membership, opening URL/path links.

User already chose sequential inline execution.
