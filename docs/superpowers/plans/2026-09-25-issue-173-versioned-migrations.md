# Versioned migrations and locked import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Schema upgrades use `PRAGMA user_version`, each step is one transaction with a pre-migration backup, import migrates through `open_db`, and import refuses `data_dir/taskboard.lock`.

**Architecture:** `open_db` reads `user_version`. A stored `0` is inferred once from tables and columns, then stamped. Later steps run only when the version is behind. `0001_init.sql` already creates the current schema, so a database with no `projects` table applies that file once and stamps version 5. `reopen_pool` calls `open_db`. `tb serve`, `tb mcp`, and the desktop app hold an exclusive flock on `taskboard.lock`; import takes the same lock or returns `conflict`. `logs/taskboard.log` rotates at 1 MiB.

**Tech Stack:** Rust, SQLite, sqlx, `libc` flock.

## Global Constraints

- Four columns stay `todo | in-progress | in-review | done`
- One local SQLite database
- `run finish` does not move the card
- HTTP is for people; agents use CLI or MCP
- No new product features
- `write_log` is not renamed here
- Do not invent a backup format
- A brand-new database does not write a pre-migration backup
- Backup names stay `backups/pre-migration-000N-<utc>.sqlite3`
- Current schema version is 5

User already chose sequential inline execution.

## File map

- Create: `crates/store-sqlite/src/lock.rs`
- Modify: `crates/store-sqlite/src/db.rs`
- Modify: `crates/store-sqlite/src/lib.rs`
- Modify: `crates/store-sqlite/src/store.rs`
- Modify: `crates/store-sqlite/Cargo.toml`
- Modify: `crates/application/src/error.rs`
- Modify: `crates/application/src/store.rs`
- Modify: `crates/application/src/app/mod.rs`
- Modify: `crates/application/tests/concurrency.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `CHANGELOG.md`

---

### Task 1: user_version migrations

**Files:**
- Modify: `crates/store-sqlite/src/db.rs`
- Test: `crates/store-sqlite/src/db.rs`

**Interfaces:**
- Consumes: `INIT_SQL`, `COMMENTS_SQL`, `CHECKS_SQL`, `LOOKUP_INDEXES_SQL`
- Produces: `open_db` leaves `PRAGMA user_version` at `5`. A stored `0` is inferred once. Step 4 adds `branch` even when `worktree_path` already exists. Each applied step is one transaction. Before a step on an existing file, `VACUUM INTO backups/pre-migration-000N-*.sqlite3`.

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn open_db_stamps_user_version() {
    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(version, 5);
    pool.close().await;
}

#[tokio::test]
async fn legacy_schema_stamps_version_without_a_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    sqlx::query("PRAGMA user_version = 0")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let pool = open_db(tmp.path()).await.unwrap();
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(version, 5);
    pool.close().await;
    let backups = tmp.path().join("backups");
    assert!(!backups.exists() || std::fs::read_dir(&backups).unwrap().next().is_none());
}

#[tokio::test]
async fn open_db_adds_missing_branch_when_worktree_path_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    sqlx::query("ALTER TABLE tasks DROP COLUMN branch")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("PRAGMA user_version = 0")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    let pool = open_db(tmp.path()).await.unwrap();
    let name: Option<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('tasks') WHERE name = 'branch'")
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert_eq!(name.as_deref(), Some("branch"));
    pool.close().await;
}
```

- [ ] **Step 2: Run** `cargo test -p taskboard-store-sqlite --lib open_db_stamps_user_version open_db_adds_missing_branch -- --test-threads=1`

Expected: FAIL. `user_version` is 0. `branch` stays missing because the probe only checks `worktree_path`.

- [ ] **Step 3: Implement**

`SCHEMA_VERSION` is 5. Infer only when `user_version` is 0:

- no `projects` → 0, apply `INIT_SQL`, stamp 5 (backup `0001` only when the file already existed)
- no `comments` → 1
- no `checks` → 2
- missing `worktree_path` or `branch` → 3
- missing `idx_runs_task_id` → 4
- otherwise → 5

Stamp an inferred version above 0 with `PRAGMA user_version` and no backup. For each step `version + 1` through 5, `VACUUM INTO` when the file existed, then one transaction: the step SQL plus `PRAGMA user_version = next`. Step 4 adds only the missing workspace columns. A failed statement rolls the transaction back, so `user_version` stays put.

Legacy tests that drop a table, column, or index must set `PRAGMA user_version = 0` before reopening. A stored 5 is trusted.

- [ ] **Step 4: Run** the same tests plus `cargo test -p taskboard-store-sqlite --lib`

Expected: PASS.

- [ ] **Step 5: Commit** `fix(store): version migrations and repair a missing branch column`

### Task 2: Import migrates, and the data-dir lock blocks a second process

**Files:**
- Create: `crates/store-sqlite/src/lock.rs`
- Modify: `crates/application/src/error.rs`
- Modify: `crates/application/src/store.rs`
- Modify: `crates/application/src/app/mod.rs`
- Modify: `crates/store-sqlite/src/store.rs`
- Modify: `crates/application/tests/concurrency.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `apps/desktop/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `open_db`, `AppError::code`
- Produces: `try_acquire_data_lock(data_dir) -> Result<DataLock, AppError>`. `AppError::DatabaseInUse` uses code `conflict` and message `database is in use; close tb serve or the desktop app before import`. `Store::try_acquire_data_lock`. `App::hold_data_lock`. `reopen_pool` calls `open_db`.

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn backup_import_migrates_an_old_backup() {
    let app = seeded_task().await;
    let dest = app.path().join("export.sqlite3");
    app.backup_export(&dest).await.unwrap();
    let status = std::process::Command::new("sqlite3")
        .arg(&dest)
        .arg("DROP TABLE checks; PRAGMA user_version = 0;")
        .status()
        .unwrap();
    assert!(status.success());
    app.backup_import(&dest).await.unwrap();
    app.check_add(
        &cli_actor(),
        taskboard_application::CheckAdd {
            task_display_id: "TASK-1".into(),
            text: "repro".into(),
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn backup_import_conflicts_when_the_data_lock_is_held() {
    let app = seeded_task().await;
    let dest = app.path().join("export.sqlite3");
    app.backup_export(&dest).await.unwrap();
    let _held = taskboard_store_sqlite::try_acquire_data_lock(app.path()).unwrap();
    let err = app.backup_import(&dest).await.unwrap_err();
    assert_eq!(err.code(), "conflict");
    assert!(err.to_string().contains("desktop"));
}
```

- [ ] **Step 2: Run** `cargo test -p taskboard-application --test concurrency backup_import_migrates backup_import_conflicts -- --test-threads=1`

Expected: the migrate test fails because `reopen_pool` does not apply schema. The lock test does not compile until `try_acquire_data_lock` exists; after it does, import still succeeds while the lock is held.

- [ ] **Step 3: Implement**

`reopen_pool` uses `open_db`. `try_acquire_data_lock` opens `data_dir/taskboard.lock` and `flock(LOCK_EX | LOCK_NB)`. `SqliteStore` keeps the guard. `backup_import` acquires it before replacing the file. If this store already holds it, import continues. `tb serve`, `tb mcp`, and desktop setup call `hold_data_lock` and keep the guard for the process.

- [ ] **Step 4: Run** the two tests, then `cargo test -p taskboard-application --test concurrency -- --test-threads=1`

Expected: PASS.

- [ ] **Step 5: Commit** `fix(store): migrate on import and refuse a held data lock`

### Task 3: Rotate the log

**Files:**
- Modify: `crates/store-sqlite/src/db.rs`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: `write_log`
- Produces: when `logs/taskboard.log` is at least 1 MiB, rename it to `taskboard.log.1` (replacing the previous `.1`) before the next line.

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
async fn open_db_rotates_a_full_log() {
    let tmp = tempfile::tempdir().unwrap();
    let logs = tmp.path().join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("taskboard.log"), vec![b'x'; 1_048_576]).unwrap();
    let pool = open_db(tmp.path()).await.unwrap();
    pool.close().await;
    let active = std::fs::read_to_string(logs.join("taskboard.log")).unwrap();
    assert!(active.contains("opening database"));
    assert!(active.len() < 1_048_576);
    assert!(logs.join("taskboard.log.1").metadata().unwrap().len() >= 1_048_576);
}
```

- [ ] **Step 2: Run** `cargo test -p taskboard-store-sqlite --lib open_db_rotates_a_full_log`

Expected: FAIL. `.1` is missing and the active file is still about 1 MiB.

- [ ] **Step 3: Implement** `rotate_log_if_needed` at the start of `write_log`. Cap is `1_048_576` bytes. One rotated file.

- [ ] **Step 4: Run** `cargo test -p taskboard-store-sqlite --lib` and `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`

Expected: PASS.

- [ ] **Step 5: Commit** `fix(store): rotate taskboard.log at 1 MiB`

## Self-review

- Spec: user_version, per-step transaction, backup before the version bump, one-time inference, import via `open_db`, advisory lock, log rotation. `write_log` rename stays out.
- `0001` stays the cumulative file. Fresh databases stamp 5 in that one transaction instead of replaying later `ALTER`s.
- A stored version above 0 is not re-inferred. Tests that simulate a legacy file set `user_version` back to 0.
