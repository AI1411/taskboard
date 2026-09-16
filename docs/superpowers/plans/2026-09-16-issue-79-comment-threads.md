# Comment / reply threads separate from notes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give each card an append-only comment thread so people can reply without fighting the spec note, and expose the latest comment on waiting cards as `reply` before `run continue`.

**Architecture:** New `comments` table (UUID id, actor, body, created_at). `App::comment_add` / `comment_list` are append-only. `TaskDetail.comments` is chronological. `TaskSummary.reply` / `TaskDetail.reply` is the latest comment body when `display_status` is `waiting`, otherwise `None`. CLI `comment add` / `comment list`. Inspector shows a plain-text thread (time · actor · text). No markdown preview, no inspector composer, no `EntityType::Comment` (skip undo/activity to keep the surface small).

**Tech Stack:** Existing Rust crates, SQLite migrations, clap, React inspector, Vitest.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- Finishing a run does not move the card
- Titles are never task identifiers
- Comments must not overwrite `note_markdown`
- No markdown preview
- Inbox membership unchanged
- CLI `--json` is snake_case; HTTP JSON is camelCase
- Prefer CLI/domain; inspector is display-only

## File map

- Modify: `crates/core/src/models.rs` — `Comment`, `reply`, `comments`
- Modify: `crates/core/src/lib.rs` — export `Comment`
- Modify: `crates/store-sqlite/migrations/0001_init.sql` — create `comments` for new DBs
- Create: `crates/store-sqlite/migrations/0002_comments.sql` — create `comments` for existing DBs
- Modify: `crates/store-sqlite/src/migrations.rs`, `db.rs`, `store.rs`
- Modify: `crates/application/src/{store.rs,commands.rs,lib.rs,app.rs}`
- Create: `crates/application/tests/comments.rs`
- Modify: `crates/cli/src/{args.rs,main.rs,output.rs}`
- Modify: `crates/cli/tests/cli_json.rs`
- Modify: `crates/api/src/dto.rs`, `crates/desktop-commands/src/dto.rs`
- Modify: `packages/types/src/index.ts`, `packages/ui/src/{fakeTransport.ts,summary.ts,Inspector.tsx,Inspector.test.tsx}`

---

### Task 1: Comment model, migration, store

**Files:**
- Modify: `crates/core/src/models.rs`
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/store-sqlite/migrations/0001_init.sql`
- Create: `crates/store-sqlite/migrations/0002_comments.sql`
- Modify: `crates/store-sqlite/src/migrations.rs`
- Modify: `crates/store-sqlite/src/db.rs`
- Modify: `crates/application/src/store.rs`
- Modify: `crates/store-sqlite/src/store.rs`

**Interfaces:**
- Consumes: `ActorKind`, existing `Store` / `open_db`
- Produces:
  - `pub struct Comment { id: Uuid, task_id: Uuid, actor_kind: ActorKind, actor_label: String, body: String, created_at: DateTime<Utc> }`
  - `TaskSummary.reply: Option<String>`
  - `TaskDetail.reply: Option<String>` and `TaskDetail.comments: Vec<Comment>`
  - `Store::insert_comment(&Comment)` / `Store::list_comments(task_id) -> Vec<Comment>` ordered `created_at ASC, id ASC`
  - `open_db` applies `0002_comments.sql` when `comments` is missing

- [ ] **Step 1: Write the failing db test**

In `crates/store-sqlite/src/db.rs` tests, change `open_db_applies_full_schema` expected tables to include `"comments"` (alphabetically between `activities` and `counters`). Add:

```rust
    #[tokio::test]
    async fn open_db_adds_comments_to_existing_schema() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        sqlx::query("DROP TABLE comments").execute(&pool).await.unwrap();
        pool.close().await;
        let pool = open_db(tmp.path()).await.unwrap();
        let name: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'comments'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert_eq!(name.as_deref(), Some("comments"));
        pool.close().await;
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-store-sqlite open_db_applies_full_schema open_db_adds_comments -- --nocapture`

Expected: FAIL — `comments` table missing / assertion `activities, counters, ...` does not include comments.

- [ ] **Step 3: Write minimal implementation**

Add to `crates/core/src/models.rs` after `Link`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Comment {
    pub id: Uuid,
    pub task_id: Uuid,
    pub actor_kind: ActorKind,
    pub actor_label: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}
```

Add `pub reply: Option<String>` to `TaskSummary` and `TaskDetail`. Add `pub comments: Vec<Comment>` to `TaskDetail` after `runs`.

Export `Comment` from `crates/core/src/lib.rs`.

Append to `0001_init.sql`:

```sql
CREATE TABLE comments (
    id BLOB NOT NULL PRIMARY KEY,
    task_id BLOB NOT NULL REFERENCES tasks (id),
    actor_kind TEXT NOT NULL,
    actor_label TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_comments_task_created ON comments (task_id, created_at, id);
```

Create `crates/store-sqlite/migrations/0002_comments.sql` with the same `CREATE TABLE IF NOT EXISTS comments` and `CREATE INDEX IF NOT EXISTS`.

In `migrations.rs`:

```rust
pub const INIT_SQL: &str = include_str!("../migrations/0001_init.sql");
pub const COMMENTS_SQL: &str = include_str!("../migrations/0002_comments.sql");
```

In `db.rs` `open_db`, after the 0001 block:

```rust
    if !comments_table_exists(&pool).await? {
        sqlx::raw_sql(COMMENTS_SQL)
            .execute(&pool)
            .await
            .map_err(map_sqlx)?;
        write_log(&log_path, &cfg.log_level, "info", "applied migration 0002_comments");
    }
```

Add `comments_table_exists` mirroring `projects_table_exists`.

Add Store trait methods and sqlite impl (same column/order pattern as `list_links`).

Every `TaskSummary {` / `TaskDetail {` construction in the repo must set `reply` (`None` until Task 2) and `comments: Vec::new()` on details.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p taskboard-store-sqlite --lib`

Expected: PASS after all struct literals compile.

- [ ] **Step 5: Commit**

```bash
git add crates/core crates/store-sqlite crates/application/src/app.rs crates/application/src/store.rs
git commit -m "feat: add comments table and store methods"
```

---

### Task 2: App comment add/list and waiting `reply`

**Files:**
- Modify: `crates/application/src/commands.rs`, `lib.rs`, `app.rs`
- Create: `crates/application/tests/comments.rs`

**Interfaces:**
- Consumes: `Store::insert_comment` / `list_comments`, `Actor`
- Produces:
  - `pub struct CommentAdd { pub task_display_id: String, pub body: String }`
  - `App::comment_add(&self, actor: &Actor, cmd: CommentAdd) -> Result<Comment, AppError>`
  - `App::comment_list(&self, task_display_id: &str) -> Result<Vec<Comment>, AppError>`
  - Body trimmed; empty → `validation_error` field `text`; over 4000 chars → `validation_error` field `text`
  - Does not change `note_markdown` or task revision
  - `to_task_summary_from_runs` / `load_task_detail` set `reply` to the last comment body when `display_status == Waiting`, else `None`
  - `load_task_detail` loads `comments`

- [ ] **Step 1: Write the failing tests**

Create `crates/application/tests/comments.rs` with the same `TestApp` helper as `run_continue.rs`, then:

```rust
#[tokio::test]
async fn comment_add_does_not_change_note() {
    let app = seeded_task().await;
    app.task_note_set(&cli_actor(), "TASK-1", "# Spec".into(), None)
        .await
        .unwrap();
    let comment = app
        .comment_add(
            &cli_actor(),
            CommentAdd {
                task_display_id: "TASK-1".into(),
                body: "please use TDD".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(comment.body, "please use TDD");
    assert_eq!(comment.actor_label, "local-cli");
    let listed = app.comment_list("TASK-1").await.unwrap();
    assert_eq!(listed.len(), 1);
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.note_markdown, "# Spec");
    assert_eq!(shown.comments.len(), 1);
    assert_eq!(shown.reply, None);
}

#[tokio::test]
async fn waiting_task_exposes_latest_comment_as_reply() {
    let app = seeded_task().await;
    app.run_start(
        &cli_actor(),
        RunStart {
            task_display_id: "TASK-1".into(),
            agent: "cursor".into(),
            session_id: None,
        },
    )
    .await
    .unwrap();
    app.run_wait(
        &cli_actor(),
        RunWait {
            run_display_id: "RUN-1".into(),
            reason: "need spec".into(),
            revision: None,
        },
    )
    .await
    .unwrap();
    app.comment_add(
        &cli_actor(),
        CommentAdd {
            task_display_id: "TASK-1".into(),
            body: "first".into(),
        },
    )
    .await
    .unwrap();
    app.comment_add(
        &cli_actor(),
        CommentAdd {
            task_display_id: "TASK-1".into(),
            body: "use TDD".into(),
        },
    )
    .await
    .unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.display_status, CardDisplayStatus::Waiting);
    assert_eq!(shown.reply.as_deref(), Some("use TDD"));
    let listed = app.task_list("renai-sim").await.unwrap();
    assert_eq!(listed[0].reply.as_deref(), Some("use TDD"));
}

#[tokio::test]
async fn blank_comment_is_validation_error() {
    let app = seeded_task().await;
    let err = app
        .comment_add(
            &cli_actor(),
            CommentAdd {
                task_display_id: "TASK-1".into(),
                body: "  ".into(),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), "validation_error");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-application --test comments -- --nocapture`

Expected: FAIL — `CommentAdd` / `comment_add` missing.

- [ ] **Step 3: Write minimal implementation**

Add `CommentAdd` and export it. Implement `comment_add` / `comment_list`. Helper:

```rust
fn reply_for(status: CardDisplayStatus, comments: &[Comment]) -> Option<String> {
    if status != CardDisplayStatus::Waiting {
        return None;
    }
    comments.last().map(|comment| comment.body.clone())
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p taskboard-application --test comments --test tasks --test notes_runs`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat: add comment add/list and waiting reply"
```

---

### Task 3: CLI `comment add` / `comment list`

**Files:**
- Modify: `crates/cli/src/args.rs`, `main.rs`, `output.rs`
- Modify: `crates/cli/tests/cli_json.rs`

**Interfaces:**
- Consumes: `CommentAdd`, `comment_add`, `comment_list`
- Produces:
  - `tb comment add TASK-n --text "..."`
  - `tb comment list TASK-n`
  - JSON snake_case `entity` (add) / `entities` (list)
  - Human add: `Commented TASK-n`
  - Human list: `TIME  ACTOR  TEXT`

- [ ] **Step 1: Write the failing CLI tests**

```rust
#[test]
fn comment_add_list_json_keeps_note() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "Fix"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["note", "set", "TASK-1", "--text", "# Spec"])
        .assert()
        .success();
    let added = json_ok(&dir, &["comment", "add", "TASK-1", "--text", "use TDD"]);
    assert_eq!(added["entity"]["body"], "use TDD");
    let listed = json_ok(&dir, &["comment", "list", "TASK-1"]);
    assert_eq!(listed["entities"].as_array().unwrap().len(), 1);
    let shown = json_ok(&dir, &["task", "show", "TASK-1"]);
    assert_eq!(shown["entity"]["note_markdown"], "# Spec");
    assert_eq!(shown["entity"]["comments"][0]["body"], "use TDD");
    assert_eq!(shown["entity"]["reply"], serde_json::Value::Null);
}

#[test]
fn comment_reply_on_waiting_task_show() {
    let dir = tempfile::tempdir().unwrap();
    tb_in(&dir)
        .args(["project", "add", "--name", "Renai Sim"])
        .assert()
        .success();
    tb_in(&dir)
        .args(["task", "create", "--project", "renai-sim", "--title", "Fix"])
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
    json_ok(&dir, &["comment", "add", "TASK-1", "--text", "here is spec"]);
    let shown = json_ok(&dir, &["task", "show", "TASK-1"]);
    assert_eq!(shown["entity"]["reply"], "here is spec");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-cli --test cli_json comment_ -- --nocapture`

Expected: FAIL — unrecognized subcommand `comment`.

- [ ] **Step 3: Write minimal implementation**

Add `Command::Comment(CommentCommand)` with `Add { display_id, text }` and `List { display_id }`. Dispatch and `print_comment_list`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p taskboard-cli --test cli_json --test help`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat: add tb comment add and comment list"
```

---

### Task 4: Inspector plain thread + DTOs/types

**Files:**
- Modify: `crates/api/src/dto.rs`, `crates/desktop-commands/src/dto.rs`
- Modify: `packages/types/src/index.ts`
- Modify: `packages/ui/src/{fakeTransport.ts,summary.ts,Inspector.tsx,Inspector.test.tsx}`

**Interfaces:**
- Consumes: `TaskDetail.comments`, `Comment`
- Produces: camelCase DTOs; inspector heading `Comments` listing `{createdAt} · {actorLabel} · {body}` as plain text (no markdown)

- [ ] **Step 1: Write the failing inspector test**

```tsx
  it("shows a plain comment thread", async () => {
    const transport = fakeTransport();
    const project = await transport.projectAdd({ name: "Alpha" });
    const task = await transport.taskCreate(project.slug, { title: "Talk", column: "todo" });
    task.comments = [
      {
        id: "c1",
        taskId: task.id,
        actorKind: "cli",
        actorLabel: "alice",
        body: "use TDD",
        createdAt: "2026-09-16T12:00:00Z",
      },
    ];
    render(<TaskboardApp transport={transport} />);
    await userEvent.click(await screen.findByText("Talk"));
    expect(await screen.findByText(/2026-09-16T12:00:00Z · alice · use TDD/)).toBeTruthy();
    expect(screen.queryByText(/marked/i)).toBeNull();
  });
```

`fakeTransport.taskShow` must return the mutated `task.comments`. Seed comments on the stored detail.

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --filter @taskboard/ui test Inspector`

Expected: FAIL — Comments heading / thread text missing.

- [ ] **Step 3: Write minimal implementation**

Add `Comment` type and fields. Map DTOs. Inspector block after Note:

```tsx
        <div>
          <h3 className={styles.heading}>Comments</h3>
          <ul className={styles.list}>
            {props.task.comments.map((comment) => (
              <li key={comment.id}>
                {comment.createdAt} · {comment.actorLabel} · {comment.body}
              </li>
            ))}
          </ul>
        </div>
```

- [ ] **Step 4: Run tests**

Run: `pnpm --filter @taskboard/ui test Inspector` and `cargo test -p taskboard-api --test routes`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat: show plain comment threads in the inspector"
```

---

## Self-review

| Acceptance | Task |
| --- | --- |
| `tb comment add` / `list` with `--json` / `--actor` | Task 3 |
| Comments do not overwrite the note | Tasks 2 + 3 |
| Inspector chronological thread | Task 4 |
| Waiting cards expose latest comment as `reply` | Task 2 |
| Plain text, no preview | Task 4 |

Out of scope: markdown preview, inbox changes, inspector composer, `EntityType::Comment` undo.

User already chose sequential inline execution.
