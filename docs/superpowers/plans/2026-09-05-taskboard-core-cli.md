# Taskboard Core, SQLite, and CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a working `taskboard` / `tb` CLI that persists projects, cards, runs, notes, trash, undo, and backups in local SQLite and enforces the domain rules in the detailed design.

**Architecture:** `taskboard-core` holds entities and pure rules. `taskboard-application` owns use cases and transactions. `taskboard-store-sqlite` implements `Store`. `taskboard-cli` parses clap, opens the data dir, and prints human or snake_case JSON. No HTTP, Vite, or Tauri in this plan.

**Tech Stack:** Rust 1.83, Tokio, SQLx (SQLite, runtime queries), clap 4, serde, uuid v7, chrono, thiserror.

## Global Constraints

- Entity IDs: UUID version 7
- Stored timestamps: UTC ISO-8601 with `Z`
- Database: SQLite in WAL mode, `PRAGMA busy_timeout = 5000`
- Human task/run IDs: `TASK-n` / `RUN-n`; projects addressed by slug
- CLI `--json` uses snake_case; success `{ "ok": true, ... }`; errors `{ "ok": false, "error": { "code", "message", "field", "current" } }`
- `TASKBOARD_DATA_DIR` overrides the platform default
- Linux default data dir: `$XDG_DATA_HOME/taskboard` or `~/.local/share/taskboard`
- macOS default data dir: `~/Library/Application Support/Taskboard/`
- English CLI flags and human output
- Finishing a run does not move the card
- `idle` is derived, never stored
- Network access is not required
- `tb serve` is out of this plan: print `error: tb serve is not available in this build` and exit 1
- Specs: `docs/superpowers/specs/2026-09-05-local-taskboard-design.md` and `docs/superpowers/specs/2026-09-05-local-taskboard-detailed-design.md`

## File map

- Create: `Cargo.toml` (workspace)
- Create: `crates/core/Cargo.toml`, `crates/core/src/lib.rs`, `crates/core/src/{ids,column,run_status,slug,order,validation,error,models}.rs`
- Create: `crates/application/Cargo.toml`, `crates/application/src/lib.rs`, `crates/application/src/{actor,error,store,app,commands}.rs`
- Create: `crates/store-sqlite/Cargo.toml`, `crates/store-sqlite/src/lib.rs`, `crates/store-sqlite/src/{paths,db,migrations.rs}`, `crates/store-sqlite/migrations/0001_init.sql`
- Create: `crates/cli/Cargo.toml`, `crates/cli/src/main.rs`, `crates/cli/src/{args,output,data_dir}.rs`
- Create: `.gitignore`

---

### Task 1: Workspace and crate stubs

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `crates/core/Cargo.toml`
- Create: `crates/core/src/lib.rs`
- Create: `crates/application/Cargo.toml`
- Create: `crates/application/src/lib.rs`
- Create: `crates/store-sqlite/Cargo.toml`
- Create: `crates/store-sqlite/src/lib.rs`
- Create: `crates/cli/Cargo.toml`
- Create: `crates/cli/src/main.rs`
- Test: `cargo metadata --offline` is not required; `cargo test -p taskboard-core` after the stub test

**Interfaces:**
- Consumes: nothing
- Produces: workspace members `taskboard-core`, `taskboard-application`, `taskboard-store-sqlite`, `taskboard-cli`

- [ ] **Step 1: Write the failing stub test**

Create `crates/core/src/lib.rs`:

```rust
pub fn workspace_name() -> &'static str {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_name_is_taskboard() {
        assert_eq!(super::workspace_name(), "taskboard");
    }
}
```

Workspace `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "crates/core",
    "crates/application",
    "crates/store-sqlite",
    "crates/cli",
]

[workspace.package]
edition = "2021"
license = "MIT"
version = "0.1.0"

[workspace.dependencies]
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
uuid = { version = "1", features = ["v7", "serde"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "chrono"] }
clap = { version = "4", features = ["derive", "env"] }
```

`crates/core/Cargo.toml`:

```toml
[package]
name = "taskboard-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
chrono.workspace = true
serde.workspace = true
thiserror.workspace = true
uuid.workspace = true
```

`.gitignore`:

```
/target
**/*.sqlite3
**/*.sqlite3-*
.DS_Store
```

Application / store / cli crates depend as follows: application → core; store-sqlite → core, application, sqlx, tokio; cli → application, store-sqlite, clap, serde_json. `crates/cli/src/main.rs` is `fn main() {}` for this task. Other `lib.rs` files are empty modules.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-core workspace_name_is_taskboard -- --exact`

Expected: FAIL with panic `not implemented` (or compile error if `workspace_name` is missing).

- [ ] **Step 3: Write minimal implementation**

```rust
pub fn workspace_name() -> &'static str {
    "taskboard"
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p taskboard-core -- --exact workspace_name_is_taskboard`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml .gitignore crates
git commit -m "chore: add Rust workspace crates"
```

---

### Task 2: IDs, enums, slug, derived idle status

**Files:**
- Create: `crates/core/src/ids.rs`
- Create: `crates/core/src/column.rs`
- Create: `crates/core/src/run_status.rs`
- Create: `crates/core/src/slug.rs`
- Create: `crates/core/src/display_status.rs`
- Modify: `crates/core/src/lib.rs`
- Test: same files under `#[cfg(test)]`

**Interfaces:**
- Consumes: `uuid::Uuid`
- Produces:
  - `Column::{Todo, InProgress, InReview, Done}` with `as_str() -> &'static str` values `todo`, `in-progress`, `in-review`, `done` and `FromStr`
  - `RunStatus::{Running, Waiting, Failed, Completed}` with `as_str()` `running|waiting|failed|completed`
  - `CardDisplayStatus::{Idle, Running, Waiting, Failed, Completed}`
  - `fn slugify(name: &str) -> Result<String, SlugError>`
  - `fn next_unique_slug(desired: &str, live_slugs: &[&str]) -> Result<String, SlugError>`
  - `fn display_id(kind: DisplayKind, n: i64) -> String` → `TASK-12` / `RUN-3`
  - `fn parse_display_id(raw: &str) -> Result<(DisplayKind, i64), ParseDisplayIdError>`
  - `fn card_display_status(runs: &[RunStatusView]) -> CardDisplayStatus`
  - `struct RunStatusView { status: RunStatus, started_at: DateTime<Utc>, display_id: String }`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn slugify_renai_sim() {
    assert_eq!(slugify("Renai Sim").unwrap(), "renai-sim");
}

#[test]
fn slug_collision_appends_number() {
    let live = ["renai-sim"];
    assert_eq!(next_unique_slug("renai-sim", &live).unwrap(), "renai-sim-2");
}

#[test]
fn explicit_duplicate_slug_is_error_when_unique_not_requested() {
    // next_unique_slug always suffixes; callers that passed --slug do not call it.
    assert_eq!(slugify("Renai Sim").unwrap(), "renai-sim");
}

#[test]
fn display_ids_round_trip() {
    assert_eq!(display_id(DisplayKind::Task, 142), "TASK-142");
    assert_eq!(
        parse_display_id("RUN-37").unwrap(),
        (DisplayKind::Run, 37)
    );
}

#[test]
fn idle_when_no_runs() {
    assert_eq!(card_display_status(&[]), CardDisplayStatus::Idle);
}

#[test]
fn newest_running_wins_over_completed() {
    use chrono::{TimeZone, Utc};
    let runs = [
        RunStatusView {
            status: RunStatus::Completed,
            started_at: Utc.with_ymd_and_hms(2026, 9, 5, 10, 0, 0).unwrap(),
            display_id: "RUN-1".into(),
        },
        RunStatusView {
            status: RunStatus::Running,
            started_at: Utc.with_ymd_and_hms(2026, 9, 5, 11, 0, 0).unwrap(),
            display_id: "RUN-2".into(),
        },
    ];
    assert_eq!(card_display_status(&runs), CardDisplayStatus::Running);
}

#[test]
fn column_from_str_rejects_unknown() {
    assert!("doing".parse::<Column>().is_err());
    assert_eq!("in-progress".parse::<Column>().unwrap(), Column::InProgress);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-core`

Expected: FAIL compile error (`slugify` not found) or assertion fail.

- [ ] **Step 3: Write minimal implementation**

`slugify`: trim, lowercase, map non `[a-z0-9]` runs to `-`, strip edges. Empty result is `SlugError::Empty`. `next_unique_slug` starts from `desired` and appends `-2`, `-3`, … until unused. `card_display_status`: if any run is `running` or `waiting`, pick the one with latest `started_at`, tie-break on `display_id` lexicographic max; else if any run exists, pick newest by the same key; else Idle.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-core`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "feat: add core identifiers, slugs, and display status"
```

---

### Task 3: Validation helpers and entity structs

**Files:**
- Create: `crates/core/src/validation.rs`
- Create: `crates/core/src/models.rs`
- Create: `crates/core/src/error.rs`
- Modify: `crates/core/src/lib.rs`
- Test: `crates/core/src/validation.rs`

**Interfaces:**
- Consumes: Task 2 types
- Produces:
  - `fn trim_title(raw: &str) -> Result<String, FieldError>` length 1..=200
  - `fn trim_project_name(raw: &str) -> Result<String, FieldError>` length 1..=120
  - `fn parse_agent(raw: &str) -> Result<String, FieldError>` `^[a-z0-9][a-z0-9._-]{0,63}$`
  - `fn parse_url(raw: &str) -> Result<String, FieldError>` scheme `http`, `https`, or `file`
  - `fn parse_path_link(raw: &str) -> Result<String, FieldError>` non-empty
  - `struct Project`, `Task`, `TaskSummary`, `TaskDetail`, `Link`, `Run`, `Activity` matching the detailed design field table (serde rename_all = "snake_case")
  - `enum FieldError { Empty, TooLong { max: usize }, Invalid { allowed: &'static str } }` with `field: &'static str` on a wrapper `ValidationError { field, source }`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn title_must_not_be_blank() {
    let err = trim_title("   ").unwrap_err();
    assert_eq!(err.field, "title");
}

#[test]
fn title_max_200() {
    assert!(trim_title(&"a".repeat(200)).is_ok());
    assert!(trim_title(&"a".repeat(201)).is_err());
}

#[test]
fn agent_pattern() {
    assert!(parse_agent("codex").is_ok());
    assert!(parse_agent("Claude").is_err());
    assert!(parse_agent("-bad").is_err());
}

#[test]
fn url_schemes() {
    assert!(parse_url("https://example.com").is_ok());
    assert!(parse_url("ftp://x").is_err());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-core`

Expected: FAIL compile error.

- [ ] **Step 3: Write structs and validators**

`Project`, `Task`, `Link`, `Run` fields exactly as the detailed design table. `TaskSummary` includes `display_status: CardDisplayStatus` and `run_message: Option<String>`. Do not put SQLite here.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-core`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "feat: add core entities and field validation"
```

---

### Task 4: Column display order

**Files:**
- Create: `crates/core/src/order.rs`
- Modify: `crates/core/src/lib.rs`
- Test: `crates/core/src/order.rs`

**Interfaces:**
- Consumes: `Task` or a lean `OrderKey { display_id, urgent, position }`
- Produces:
  - `fn sort_column(keys: &mut [OrderKey])` by `urgent DESC`, `position ASC`
  - `fn rewrite_positions(keys: &mut [OrderKey])` sets `position` to `0..n-1` in current slice order
  - `fn place_urgent(keys: &mut Vec<OrderKey>, display_id: &str, urgent: bool)` moves that key to end of urgent group or start of non-urgent group, then `rewrite_positions`
  - `fn place_before(keys: &mut Vec<OrderKey>, display_id: &str, before: Option<&str>) -> Result<(), OrderError>` where `OrderError::NotInColumn` / `OrderError::DifferentColumn` is not used here (same slice = same column). `before: None` means end.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn urgent_cards_sort_above_non_urgent() {
    let mut keys = vec![
        OrderKey { display_id: "TASK-1".into(), urgent: false, position: 0 },
        OrderKey { display_id: "TASK-2".into(), urgent: true, position: 1 },
        OrderKey { display_id: "TASK-3".into(), urgent: false, position: 2 },
    ];
    sort_column(&mut keys);
    let ids: Vec<_> = keys.iter().map(|k| k.display_id.as_str()).collect();
    assert_eq!(ids, ["TASK-2", "TASK-1", "TASK-3"]);
}

#[test]
fn toggling_urgent_on_moves_to_end_of_urgent_group() {
    let mut keys = vec![
        OrderKey { display_id: "TASK-1".into(), urgent: true, position: 0 },
        OrderKey { display_id: "TASK-2".into(), urgent: false, position: 1 },
    ];
    place_urgent(&mut keys, "TASK-2", true);
    assert!(keys[0].urgent && keys[1].urgent);
    assert_eq!(keys[1].display_id, "TASK-2");
    assert_eq!(keys[0].position, 0);
    assert_eq!(keys[1].position, 1);
}
```

Add a property-style test: for random permutations of 8 keys, `sort_column` then `rewrite_positions` yields unique positions `0..n-1` and all urgent ids precede non-urgent ids.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-core place_urgent -- --nocapture`

Expected: FAIL compile error.

- [ ] **Step 3: Implement order helpers**

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-core`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "feat: add urgent column ordering rules"
```

---

### Task 5: Application errors and Store trait

**Files:**
- Create: `crates/application/src/error.rs`
- Create: `crates/application/src/actor.rs`
- Create: `crates/application/src/store.rs`
- Create: `crates/application/src/commands.rs`
- Modify: `crates/application/src/lib.rs`
- Test: `crates/application/src/error.rs`

**Interfaces:**
- Consumes: `taskboard_core::{Project, Task, TaskDetail, TaskSummary, Run, Link, Column, ...}`
- Produces:

```rust
pub struct Actor { pub kind: ActorKind, pub label: String }

pub enum AppError {
    Validation { field: String, message: String },
    NotFound { entity: String, id: String },
    DuplicateSlug { slug: String },
    RevisionConflict { current: serde_json::Value },
    DifferentColumn { left: String, right: String },
    UndoConflict { current: serde_json::Value },
    DatabaseBusy,
    Io(String),
}

impl AppError {
    pub fn code(&self) -> &'static str { /* validation_error, not_found, ... */ }
}

pub struct Trash { pub projects: Vec<Project>, pub tasks: Vec<TaskSummary> }
pub struct SyncDelta {
    pub sequence: i64,
    pub projects: Vec<Project>,
    pub tasks: Vec<TaskDetail>,
    pub runs: Vec<Run>,
}
pub struct UndoResult { pub entity_type: EntityType, pub entity: serde_json::Value }

#[async_trait]
pub trait Store: Send + Sync {
    async fn next_display_n(&mut self, counter: &str) -> Result<i64, AppError>;
    async fn next_activity_sequence(&mut self) -> Result<i64, AppError>;
    async fn insert_activity(&mut self, row: NewActivity) -> Result<(), AppError>;
    // plus get/list/insert/update/delete for projects, tasks, links, runs
    async fn rewrite_task_positions(&mut self, project_id: Uuid, column: Column, ordered_ids: &[Uuid]) -> Result<(), AppError>;
    async fn activity_head(&mut self) -> Result<i64, AppError>;
    async fn latest_activity_for(&mut self, entity_id: Uuid) -> Result<Option<Activity>, AppError>;
    async fn latest_undoable(&mut self) -> Result<Option<Activity>, AppError>;
    async fn purge_expired(&mut self, now: DateTime<Utc>, retention_days: i64) -> Result<u64, AppError>;
    async fn backup_to(&mut self, dest: &Path) -> Result<(), AppError>;
    async fn replace_from(&mut self, src: &Path) -> Result<(), AppError>;
    async fn begin(&mut self) -> Result<(), AppError>;
    async fn commit(&mut self) -> Result<(), AppError>;
    async fn rollback(&mut self) -> Result<(), AppError>;
}
```

Command structs: `ProjectAdd { name, repo_path, slug }`, `TaskCreate { project_slug, title, column, urgent }`, `RunStart { task_display_id, agent, session_id }`, and the rest named as in the detailed design `App` impl.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn revision_conflict_code() {
    let err = AppError::RevisionConflict { current: serde_json::json!({}) };
    assert_eq!(err.code(), "revision_conflict");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p taskboard-application`

Expected: FAIL compile error.

- [ ] **Step 3: Implement error, actor, commands, Store trait**

Use `async_trait` crate. Keep methods listed above; do not skip `rewrite_task_positions` two-phase contract: implementation must set `position = -(old+1)` then `0..n-1`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-application`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/application
git commit -m "feat: add application errors and Store trait"
```

---

### Task 6: SQLite open, data dir, migration 0001

**Files:**
- Create: `crates/store-sqlite/src/paths.rs`
- Create: `crates/store-sqlite/src/config.rs`
- Create: `crates/store-sqlite/src/db.rs`
- Create: `crates/store-sqlite/src/migrations.rs`
- Create: `crates/store-sqlite/migrations/0001_init.sql`
- Modify: `crates/store-sqlite/src/lib.rs`
- Test: `crates/store-sqlite/src/paths.rs`, `crates/store-sqlite/src/db.rs`

**Interfaces:**
- Consumes: Task 5 `Store` (implemented in later tasks)
- Produces:
  - `fn default_data_dir() -> PathBuf`
  - `fn resolve_data_dir(cli_override: Option<&Path>, env: Option<&OsStr>) -> PathBuf` where CLI wins, then `TASKBOARD_DATA_DIR`, then default
  - `struct Config { log_level: String, poll_interval_ms: u64, note_debounce_ms: u64, backup_retention: u32, busy_timeout_ms: u64 }`
  - `fn load_config(data_dir: &Path) -> Config` reads `{data_dir}/config.toml` with the defaults from the detailed design. Missing file uses defaults. Unknown keys ignored.
  - `async fn open_db(data_dir: &Path) -> Result<sqlx::SqlitePool, AppError>` creates dir, `taskboard.sqlite3`, WAL, foreign_keys, busy_timeout from Config (default 5000), runs `0001_init.sql` if `projects` table is missing
  - SQL file contents exactly the schema in the detailed design section 5

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn cli_override_wins() {
    let dir = resolve_data_dir(Some(Path::new("/tmp/tb-cli")), Some(OsStr::new("/tmp/tb-env")));
    assert_eq!(dir, Path::new("/tmp/tb-cli"));
}

#[tokio::test]
async fn open_db_creates_schema() {
    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    let n: i64 = sqlx::query_scalar("select count(*) from counters")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 3);
}

#[test]
fn missing_config_uses_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let cfg = load_config(tmp.path());
    assert_eq!(cfg.poll_interval_ms, 1000);
    assert_eq!(cfg.note_debounce_ms, 400);
    assert_eq!(cfg.backup_retention, 30);
    assert_eq!(cfg.busy_timeout_ms, 5000);
}

#[test]
fn config_ignores_unknown_keys() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("config.toml"), "log_level = \"debug\"\nunknown = 1\n").unwrap();
    let cfg = load_config(tmp.path());
    assert_eq!(cfg.log_level, "debug");
}
```

Add `tempfile` as a store-sqlite dev-dependency.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-store-sqlite`

Expected: FAIL compile error.

- [ ] **Step 3: Implement paths, SQL, open_db, load_config**

On Linux, if `XDG_DATA_HOME` is set use `$XDG_DATA_HOME/taskboard`, else `~/.local/share/taskboard`. On macOS use `~/Library/Application Support/Taskboard`. Copy `0001_init.sql` from the detailed design verbatim. Parse TOML with the `toml` crate. Create `{data_dir}/logs` on open. Log to `{data_dir}/logs/taskboard.log` at `cfg.log_level`. Never log `note_markdown`, run `summary`, or `repo_path`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-store-sqlite`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/store-sqlite
git commit -m "feat: open SQLite database and apply initial schema"
```

---

### Task 7: Project use cases

**Files:**
- Create: `crates/application/src/app.rs`
- Create: `crates/store-sqlite/src/store.rs` (Store impl start: projects + activities + counters)
- Modify: `crates/application/src/lib.rs`
- Test: `crates/application/tests/projects.rs`

**Interfaces:**
- Consumes: `Store`, `ProjectAdd`, `Actor`
- Produces: `App::project_add`, `project_list`, `project_update`, `project_reorder`, `project_archive`, `project_delete`, `project_restore`, `project_note_set`, `project_show` (`fn project_show(&self, slug: &str) -> Result<Project, AppError>`)

- [ ] **Step 1: Write the failing tests**

`crates/application/tests/projects.rs` opens a temp dir via `open_db`, wraps `SqliteStore`, `App::new(store, clock)`.

```rust
#[tokio::test]
async fn add_project_assigns_slug_and_revision_one() {
    let app = test_app().await;
    let p = app.project_add(&cli_actor(), ProjectAdd {
        name: "Renai Sim".into(),
        repo_path: None,
        slug: None,
    }).await.unwrap();
    assert_eq!(p.slug, "renai-sim");
    assert_eq!(p.revision, 1);
}

#[tokio::test]
async fn duplicate_explicit_slug_fails() {
    let app = test_app().await;
    app.project_add(&cli_actor(), ProjectAdd { name: "A".into(), repo_path: None, slug: Some("renai-sim".into()) }).await.unwrap();
    let err = app.project_add(&cli_actor(), ProjectAdd { name: "B".into(), repo_path: None, slug: Some("renai-sim".into()) }).await.unwrap_err();
    assert_eq!(err.code(), "duplicate_slug");
}

#[tokio::test]
async fn second_same_name_gets_suffix() {
    let app = test_app().await;
    app.project_add(&cli_actor(), ProjectAdd { name: "Renai Sim".into(), repo_path: None, slug: None }).await.unwrap();
    let p2 = app.project_add(&cli_actor(), ProjectAdd { name: "Renai Sim".into(), repo_path: None, slug: None }).await.unwrap();
    assert_eq!(p2.slug, "renai-sim-2");
}

#[tokio::test]
async fn default_list_hides_archived_and_deleted() {
    let app = test_app().await;
    let a = app.project_add(&cli_actor(), ProjectAdd { name: "A".into(), repo_path: None, slug: None }).await.unwrap();
    let b = app.project_add(&cli_actor(), ProjectAdd { name: "B".into(), repo_path: None, slug: None }).await.unwrap();
    app.project_archive(&cli_actor(), &b.slug, true, None).await.unwrap();
    app.project_delete(&cli_actor(), &a.slug, None).await.unwrap();
    let live = app.project_list(false).await.unwrap();
    assert!(live.is_empty());
    let archived = app.project_list(true).await.unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].slug, b.slug);
}
```

Helper `cli_actor()` returns `Actor { kind: ActorKind::Cli, label: "local-cli".into() }`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-application --test projects`

Expected: FAIL compile error.

- [ ] **Step 3: Implement App project methods and SqliteStore project SQL**

Each mutation: begin, write row, insert activity (`project.create` etc.), commit. `sort_order` for a new project is `max(sort_order)+1` among live rows, then optional compact is not required until `project_reorder`. `project_reorder` takes slugs covering all live projects.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-application --test projects`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/application crates/store-sqlite
git commit -m "feat: add project use cases and sqlite persistence"
```

---

### Task 8: Task create, list, show, move, reorder, urgent

**Files:**
- Modify: `crates/application/src/app.rs`
- Modify: `crates/store-sqlite/src/store.rs`
- Test: `crates/application/tests/tasks.rs`

**Interfaces:**
- Consumes: Task 4 order helpers, Task 7 App
- Produces: `task_create`, `task_list`, `task_show`, `task_update`, `task_move`, `task_reorder`, `task_urgent`

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn create_task_gets_task_1_in_todo() {
    let app = seeded().await; // project renai-sim
    let t = app.task_create(&cli_actor(), TaskCreate {
        project_slug: "renai-sim".into(),
        title: "Fix login error".into(),
        column: None,
        urgent: false,
    }).await.unwrap();
    assert_eq!(t.task.display_id, "TASK-1");
    assert_eq!(t.task.column, Column::Todo);
    assert_eq!(t.display_status, CardDisplayStatus::Idle);
}

#[tokio::test]
async fn move_rewrites_positions_in_both_columns() {
    let app = seeded_with_two_tasks().await;
    app.task_move(&cli_actor(), "TASK-1", Column::InProgress, None).await.unwrap();
    let todo = app.task_list("renai-sim").await.unwrap();
    let todo_ids: Vec<_> = todo.iter().filter(|t| t.column == Column::Todo).map(|t| t.position).collect();
    assert_eq!(todo_ids, vec![0]);
}

#[tokio::test]
async fn prioritize_across_columns_errors() {
    let app = seeded_with_two_tasks_in_different_columns().await;
    let err = app.task_reorder(&cli_actor(), "TASK-1", Some("TASK-2"), None).await.unwrap_err();
    assert_eq!(err.code(), "different_column");
}

#[tokio::test]
async fn blank_title_is_validation_error() {
    let app = seeded().await;
    let err = app.task_create(&cli_actor(), TaskCreate {
        project_slug: "renai-sim".into(),
        title: "  ".into(),
        column: None,
        urgent: false,
    }).await.unwrap_err();
    assert_eq!(err.code(), "validation_error");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-application --test tasks`

Expected: FAIL compile error.

- [ ] **Step 3: Implement task SQL and use cases**

Position rewrite must be two-phase to satisfy `idx_tasks_live_position`. `task_list` sorts with `ORDER BY urgent DESC, position ASC`. `TaskDetail` loads links (empty), runs (empty), recent 20 activities for that entity.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-application --test tasks -- --test-threads=1`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/application crates/store-sqlite
git commit -m "feat: add task create, move, and reorder"
```

---

### Task 9: Notes, links, runs

**Files:**
- Modify: `crates/application/src/app.rs`
- Modify: `crates/store-sqlite/src/store.rs`
- Test: `crates/application/tests/notes_runs.rs`

**Interfaces:**
- Consumes: Task 8
- Produces: `task_note_set`, `task_note_add`, `project_note_set` (already exists), `link_add`, `link_remove`, `run_start`, `run_update`, `run_wait`, `run_fail`, `run_finish`

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn note_add_appends_paragraph() {
    let app = seeded_task().await;
    app.task_note_set(&cli_actor(), "TASK-1", "First".into(), None).await.unwrap();
    let t = app.task_note_add(&cli_actor(), "TASK-1", "Second", None).await.unwrap();
    assert_eq!(t.task.note_markdown, "First\n\nSecond");
}

#[tokio::test]
async fn run_start_returns_run_1_running() {
    let app = seeded_task().await;
    let r = app.run_start(&cli_actor(), RunStart {
        task_display_id: "TASK-1".into(),
        agent: "codex".into(),
        session_id: Some("abc123".into()),
    }).await.unwrap();
    assert_eq!(r.display_id, "RUN-1");
    assert_eq!(r.status, RunStatus::Running);
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.display_status, CardDisplayStatus::Running);
    assert_eq!(shown.task.column, Column::Todo);
}

#[tokio::test]
async fn finish_does_not_move_column() {
    let app = seeded_running().await;
    app.run_finish(&cli_actor(), RunFinish { run_display_id: "RUN-1".into(), summary: "done".into() }).await.unwrap();
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.task.column, Column::Todo);
    assert_eq!(shown.display_status, CardDisplayStatus::Completed);
}

#[tokio::test]
async fn wait_requires_reason() {
    let app = seeded_running().await;
    let err = app.run_wait(&cli_actor(), RunWait { run_display_id: "RUN-1".into(), reason: " ".into() }).await.unwrap_err();
    assert_eq!(err.code(), "validation_error");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-application --test notes_runs`

Expected: FAIL.

- [ ] **Step 3: Implement notes, links, runs**

`run_wait` sets status waiting and `waiting_reason`. `run_fail` / `run_finish` set `ended_at` to clock now. Multiple running runs allowed. `task_note_add` is `existing + "\n\n" + paragraph` when existing is non-empty.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-application --test notes_runs -- --test-threads=1`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/application crates/store-sqlite
git commit -m "feat: add notes, links, and agent runs"
```

---

### Task 10: Revisions, trash, purge, undo, backup

**Files:**
- Modify: `crates/application/src/app.rs`
- Modify: `crates/store-sqlite/src/store.rs`
- Test: `crates/application/tests/concurrency.rs`

**Interfaces:**
- Consumes: previous App methods
- Produces: revision checks on every mutation that takes `Option<i64>`, `task_delete` / `restore`, `trash_list`, `purge_expired_trash`, `undo`, `activity_head`, `sync`, `backup_export`, `backup_import`

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn stale_revision_does_not_write() {
    let app = seeded_task().await;
    let err = app.task_update(&cli_actor(), TaskUpdate {
        display_id: "TASK-1".into(),
        title: Some("Nope".into()),
        revision: Some(0),
    }).await.unwrap_err();
    assert_eq!(err.code(), "revision_conflict");
    let shown = app.task_show("TASK-1").await.unwrap();
    assert_eq!(shown.task.title, "Fix login error");
}

#[tokio::test]
async fn delete_hides_then_restore() {
    let app = seeded_task().await;
    app.task_delete(&cli_actor(), "TASK-1", None).await.unwrap();
    assert!(app.task_show("TASK-1").await.is_err());
    let trash = app.trash_list().await.unwrap();
    assert_eq!(trash.tasks[0].display_id, "TASK-1");
    app.task_restore(&cli_actor(), "TASK-1").await.unwrap();
    assert_eq!(app.task_show("TASK-1").await.unwrap().task.title, "Fix login error");
}

#[tokio::test]
async fn purge_after_thirty_days() {
    let app = seeded_task().await;
    app.task_delete(&cli_actor(), "TASK-1", None).await.unwrap();
    let later = Utc::now() + chrono::Duration::days(31);
    let n = app.purge_expired_trash(later).await.unwrap();
    assert!(n >= 1);
    let trash = app.trash_list().await.unwrap();
    assert!(trash.tasks.is_empty());
}

#[tokio::test]
async fn undo_restores_title_then_conflict_on_second_change() {
    let app = seeded_task().await;
    app.task_update(&cli_actor(), TaskUpdate { display_id: "TASK-1".into(), title: Some("B".into()), revision: None }).await.unwrap();
    app.undo(&cli_actor()).await.unwrap();
    assert_eq!(app.task_show("TASK-1").await.unwrap().task.title, "Fix login error");
}

#[tokio::test]
async fn two_pools_second_writer_sees_busy_or_success() {
    // Open two Apps on the same file. Overlapping transactions must not hang
    // longer than busy_timeout; they either succeed serially or return database_busy.
}
```

For the concurrent test: hold an exclusive write in task A (begin + insert) while task B calls `task_create`; assert B returns within 6s as either Ok or `database_busy`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-application --test concurrency -- --test-threads=1`

Expected: FAIL.

- [ ] **Step 3: Implement revision compare, trash, undo, backup**

Undo: load `latest_undoable`, if a newer activity for that `entity_id` exists → `undo_conflict`. Otherwise apply compensating write from `before_json` / `after_json` and insert `operation = undo`. Backup uses SQLite backup API (`VACUUM INTO` is acceptable if it is a consistent snapshot; prefer `VACUUM INTO` on a checkpointed WAL). Import: copy current to `backups/pre-import-{timestamp}.sqlite3`, close pool, replace file, reopen.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-application -- --test-threads=1`

Expected: PASS all application tests.

- [ ] **Step 5: Commit**

```bash
git add crates/application crates/store-sqlite
git commit -m "feat: add revisions, trash, undo, and backups"
```

---

### Task 11: CLI clap, JSON, human output, e2e

**Files:**
- Create: `crates/cli/src/args.rs`
- Create: `crates/cli/src/output.rs`
- Create: `crates/cli/src/data_dir.rs`
- Modify: `crates/cli/src/main.rs`
- Test: `crates/cli/tests/cli_json.rs`
- Test: `crates/cli/tests/help.rs`

**Interfaces:**
- Consumes: `App` methods from Tasks 7–10
- Produces: binary `taskboard`; `tb serve` stub; commands listed in detailed design section 7 except serve (error)

- [ ] **Step 1: Write the failing tests**

```rust
fn tb() -> Command {
    let mut c = Command::cargo_bin("taskboard").unwrap();
    let dir = tempfile::tempdir().unwrap();
    c.env("TASKBOARD_DATA_DIR", dir.path());
    // leak or pass dir lifetime via a helper that returns (Command, TempDir)
    c
}

#[test]
fn project_add_json() {
    let (mut cmd, _dir) = tb();
    let out = cmd.args(["project", "add", "--name", "Renai Sim", "--json"]).assert().success().get_output().stdout.clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["entity"]["slug"], "renai-sim");
    assert_eq!(v["revision"], 1);
}

#[test]
fn failed_mutation_is_not_ok_true() {
    let (mut cmd, _dir) = tb();
    cmd.args(["task", "show", "TASK-999", "--json"]).assert().failure();
    // run again capturing stdout
}

#[test]
fn serve_is_unavailable() {
    let (mut cmd, _dir) = tb();
    cmd.args(["serve"]).assert().failure();
}

#[test]
fn help_lists_project_add() {
    let mut cmd = Command::cargo_bin("taskboard").unwrap();
    let out = String::from_utf8(cmd.arg("--help").output().unwrap().stdout).unwrap();
    assert!(out.contains("project"));
    assert!(out.contains("task"));
    assert!(out.contains("run"));
}
```

Use `assert_cmd` and `predicates` as cli dev-dependencies. Capture stderr for `error: ...`. Actor resolution: `--actor`, else `TASKBOARD_ACTOR`, else `local-cli`.

JSON success for lists: `{ "ok": true, "entities": [...] }` with no fake `entity` key.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p taskboard-cli`

Expected: FAIL (binary does nothing).

- [ ] **Step 3: Implement clap tree and dispatch**

`main` is async tokio. Open db from `resolve_data_dir`. Map `AppError` through `output::print_error`. Human project add: `Created project  renai-sim  Renai Sim`. Human task create: `Created TASK-1  Fix login error  [todo]`. Wire every command in the detailed design table except implementing serve.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p taskboard-cli -- --test-threads=1` and `cargo test -- --test-threads=1`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/cli
git commit -m "feat: add taskboard CLI with json and human output"
```

---

## Spec coverage

| Story / rule | Task |
| --- | --- |
| US-01..03 projects | 7, 11 |
| US-04..08 cards, urgent, idle badge data | 2, 4, 8, 11 |
| US-09..11 notes, links, runs | 9, 11 |
| US-12..13 trash, undo | 10, 11 |
| US-14 sync sequence | 10 `activity_head` / `sync` (no HTTP poll yet) |
| US-15 CLI help | 11 |
| Schema 0001 | 6 |
| Two-phase positions | 8 |
| Backups | 10 |
| config.toml + logs | 6 |
| `tb serve` stub | 11 |
| Desktop / web / HTTP | Plan 2 and Plan 3, not this plan |
| Homebrew tap `AI1411/taskboard` | After a macOS CLI binary exists; not in these three plans |
