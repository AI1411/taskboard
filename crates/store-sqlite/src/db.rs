use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use taskboard_application::AppError;

use crate::config::load_config;
use crate::migrations::{
    CHECKS_SQL, COMMENTS_SQL, INIT_SQL, LOOKUP_INDEXES_SQL, TASK_WORKSPACE_SQL,
};

const SCHEMA_VERSION: i64 = 5;
const LOG_MAX_BYTES: u64 = 1_048_576;

/// Opens `{data_dir}/taskboard.sqlite3`, applying `0001_init.sql` when `projects` is missing.
pub async fn open_db(data_dir: &Path) -> Result<SqlitePool, AppError> {
    open_db_owned(data_dir.to_path_buf()).await
}

/// Owned-path entry so callers inside `async_trait` can await a `'static` future.
pub(crate) async fn open_db_owned(data_dir: PathBuf) -> Result<SqlitePool, AppError> {
    fs::create_dir_all(&data_dir).map_err(map_io)?;
    let logs_dir = data_dir.join("logs");
    fs::create_dir_all(&logs_dir).map_err(map_io)?;
    let log_path = create_log_file(&logs_dir)?;

    let cfg = load_config(&data_dir);
    write_log(&log_path, &cfg.log_level, "info", "opening database");

    let db_path = data_dir.join("taskboard.sqlite3");
    let db_existed = db_path.exists();

    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_millis(cfg.busy_timeout_ms));

    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .map_err(map_sqlx)?;

    migrate(data_dir, pool.clone(), db_existed, log_path, cfg.log_level).await?;

    Ok(pool)
}

async fn migrate(
    data_dir: PathBuf,
    pool: SqlitePool,
    db_existed: bool,
    log_path: PathBuf,
    log_level: String,
) -> Result<(), AppError> {
    let mut version = read_user_version(&pool).await?;
    if version == 0 {
        let inferred = infer_version(&pool).await?;
        if inferred == 0 {
            if db_existed {
                backup_pre_migration(&data_dir, &pool, 1).await?;
            }
            apply_sql_version(&pool, INIT_SQL, SCHEMA_VERSION).await?;
            write_log(&log_path, &log_level, "info", "applied migration 0001_init");
            return Ok(());
        }
        set_user_version(&pool, inferred).await?;
        version = inferred;
    }

    while version < SCHEMA_VERSION {
        let next = version + 1;
        if db_existed {
            backup_pre_migration(&data_dir, &pool, next).await?;
        }
        apply_step(&pool, next).await?;
        let message = format!("applied migration {next:04}");
        write_log(&log_path, &log_level, "info", &message);
        version = next;
    }
    Ok(())
}

async fn apply_step(pool: &SqlitePool, version: i64) -> Result<(), AppError> {
    match version {
        2 => apply_sql_version(pool, COMMENTS_SQL, 2).await,
        3 => apply_sql_version(pool, CHECKS_SQL, 3).await,
        4 => apply_workspace_step(pool).await,
        5 => apply_sql_version(pool, LOOKUP_INDEXES_SQL, 5).await,
        other => Err(AppError::Io(format!("unknown migration {other}"))),
    }
}

async fn apply_workspace_step(pool: &SqlitePool) -> Result<(), AppError> {
    let add_worktree = !column_exists(pool, "tasks", "worktree_path").await?;
    let add_branch = !column_exists(pool, "tasks", "branch").await?;
    if add_worktree && add_branch {
        return apply_sql_version(pool, TASK_WORKSPACE_SQL, 4).await;
    }
    let mut tx = pool.begin().await.map_err(map_sqlx)?;
    if add_worktree {
        sqlx::query("ALTER TABLE tasks ADD COLUMN worktree_path TEXT")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
    }
    if add_branch {
        sqlx::query("ALTER TABLE tasks ADD COLUMN branch TEXT")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx)?;
    }
    sqlx::query("PRAGMA user_version = 4")
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx)?;
    tx.commit().await.map_err(map_sqlx)?;
    Ok(())
}

async fn apply_sql_version(
    pool: &SqlitePool,
    sql: &'static str,
    version: i64,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await.map_err(map_sqlx)?;
    if let Err(err) = sqlx::raw_sql(sql).execute(&mut *tx).await {
        let _ = tx.rollback().await;
        return Err(map_sqlx(err));
    }
    let stamp = pragma_user_version(version);
    if let Err(err) = sqlx::query(stamp).execute(&mut *tx).await {
        let _ = tx.rollback().await;
        return Err(map_sqlx(err));
    }
    tx.commit().await.map_err(map_sqlx)?;
    Ok(())
}

fn pragma_user_version(version: i64) -> &'static str {
    match version {
        1 => "PRAGMA user_version = 1",
        2 => "PRAGMA user_version = 2",
        3 => "PRAGMA user_version = 3",
        4 => "PRAGMA user_version = 4",
        5 => "PRAGMA user_version = 5",
        9 => "PRAGMA user_version = 9",
        _ => "PRAGMA user_version = 0",
    }
}

async fn read_user_version(pool: &SqlitePool) -> Result<i64, AppError> {
    sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await
        .map_err(map_sqlx)
}

async fn set_user_version(pool: &SqlitePool, version: i64) -> Result<(), AppError> {
    sqlx::query(pragma_user_version(version))
        .execute(pool)
        .await
        .map_err(map_sqlx)?;
    Ok(())
}

async fn infer_version(pool: &SqlitePool) -> Result<i64, AppError> {
    if !projects_table_exists(pool).await? {
        return Ok(0);
    }
    if !comments_table_exists(pool).await? {
        return Ok(1);
    }
    if !checks_table_exists(pool).await? {
        return Ok(2);
    }
    if !column_exists(pool, "tasks", "worktree_path").await?
        || !column_exists(pool, "tasks", "branch").await?
    {
        return Ok(3);
    }
    if !index_exists(pool, "idx_runs_task_id").await? {
        return Ok(4);
    }
    Ok(SCHEMA_VERSION)
}

async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> Result<bool, AppError> {
    let sql = format!("SELECT name FROM pragma_table_info('{table}') WHERE name = ?");
    let name: Option<String> = sqlx::query_scalar(&sql)
        .bind(column)
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx)?;
    Ok(name.is_some())
}

async fn checks_table_exists(pool: &SqlitePool) -> Result<bool, AppError> {
    let name: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'checks'",
    )
    .fetch_optional(pool)
    .await
    .map_err(map_sqlx)?;
    Ok(name.is_some())
}

async fn comments_table_exists(pool: &SqlitePool) -> Result<bool, AppError> {
    let name: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'comments'",
    )
    .fetch_optional(pool)
    .await
    .map_err(map_sqlx)?;
    Ok(name.is_some())
}

async fn index_exists(pool: &SqlitePool, name: &str) -> Result<bool, AppError> {
    let found: Option<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'index' AND name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await
            .map_err(map_sqlx)?;
    Ok(found.is_some())
}

async fn projects_table_exists(pool: &SqlitePool) -> Result<bool, AppError> {
    let name: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'projects'",
    )
    .fetch_optional(pool)
    .await
    .map_err(map_sqlx)?;
    Ok(name.is_some())
}

async fn backup_pre_migration(
    data_dir: &Path,
    pool: &SqlitePool,
    version: i64,
) -> Result<(), AppError> {
    let backups = data_dir.join("backups");
    fs::create_dir_all(&backups).map_err(map_io)?;
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let dest = backups.join(format!("pre-migration-{version:04}-{stamp}.sqlite3"));
    let dest_sql = dest.to_string_lossy().replace('\'', "''");
    let vacuum = format!("VACUUM INTO '{dest_sql}'");
    sqlx::query(&vacuum).execute(pool).await.map_err(map_sqlx)?;
    Ok(())
}

fn create_log_file(logs_dir: &Path) -> Result<PathBuf, AppError> {
    let path = logs_dir.join("taskboard.log");
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(map_io)?;
    Ok(path)
}

fn rotate_log_if_needed(path: &Path) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    if meta.len() < LOG_MAX_BYTES {
        return;
    }
    let rotated = path.with_file_name("taskboard.log.1");
    let _ = fs::remove_file(&rotated);
    let _ = fs::rename(path, &rotated);
}

fn write_log(path: &Path, configured_level: &str, message_level: &str, message: &str) {
    rotate_log_if_needed(path);
    if message.contains("note_markdown")
        || message.contains("repo_path")
        || message.contains("summary")
    {
        return;
    }
    if !log_enabled(configured_level, message_level) {
        return;
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let ts = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let _ = writeln!(file, "{ts} {message_level} {message}");
}

fn log_enabled(configured: &str, message_level: &str) -> bool {
    rank(message_level) <= rank(configured)
}

fn rank(level: &str) -> u8 {
    match level {
        "error" => 1,
        "warn" => 2,
        "info" => 3,
        "debug" => 4,
        _ => 3,
    }
}

fn map_io(err: std::io::Error) -> AppError {
    AppError::Io(err.to_string())
}

pub(crate) fn map_sqlx(err: sqlx::Error) -> AppError {
    if matches!(err, sqlx::Error::PoolTimedOut) {
        return AppError::DatabaseBusy;
    }
    if let sqlx::Error::Database(db_err) = &err {
        let code = db_err.code();
        if matches!(code.as_deref(), Some("5" | "6")) {
            return AppError::DatabaseBusy;
        }
        let msg = db_err.message();
        if msg.contains("database is locked") || msg.contains("database is busy") {
            return AppError::DatabaseBusy;
        }
    }
    AppError::Io(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn open_db_creates_schema() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        let n: i64 = sqlx::query_scalar("select count(*) from counters")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 4);
        pool.close().await;
    }

    #[tokio::test]
    async fn open_db_creates_log_file() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        assert!(tmp.path().join("logs/taskboard.log").is_file());
        pool.close().await;
    }

    #[tokio::test]
    async fn open_db_enables_wal_and_foreign_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
        let fk: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(fk, 1);
        let timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(timeout, 5000);
        pool.close().await;
    }

    #[tokio::test]
    async fn open_db_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        pool.close().await;
        let pool = open_db(tmp.path()).await.unwrap();
        let n: i64 = sqlx::query_scalar("select count(*) from counters")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 4);
        pool.close().await;
    }

    #[tokio::test]
    async fn first_open_skips_pre_migration_backup() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        pool.close().await;
        let backups = tmp.path().join("backups");
        assert!(!backups.exists() || fs::read_dir(&backups).unwrap().next().is_none());
    }

    #[tokio::test]
    async fn open_db_backs_up_existing_db_before_0001() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("taskboard.sqlite3");
        {
            let options = SqliteConnectOptions::new()
                .filename(&db)
                .create_if_missing(true);
            let pool = SqlitePool::connect_with(options).await.unwrap();
            sqlx::query("CREATE TABLE leftover (id INTEGER PRIMARY KEY)")
                .execute(&pool)
                .await
                .unwrap();
            pool.close().await;
        }
        let pool = open_db(tmp.path()).await.unwrap();
        pool.close().await;
        let backups = tmp.path().join("backups");
        let entries: Vec<_> = fs::read_dir(&backups)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].starts_with("pre-migration-0001-"));
        assert!(entries[0].ends_with(".sqlite3"));
    }

    #[test]
    fn init_sql_matches_migrations_file() {
        assert_eq!(INIT_SQL, include_str!("../migrations/0001_init.sql"));
    }

    #[tokio::test]
    async fn open_db_applies_full_schema() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            tables,
            [
                "activities",
                "checks",
                "comments",
                "counters",
                "links",
                "projects",
                "runs",
                "tasks",
            ]
        );
        pool.close().await;
    }

    #[tokio::test]
    async fn open_db_adds_lookup_indexes() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        for name in [
            "idx_runs_task_id",
            "idx_links_task_id",
            "idx_links_kind_value",
        ] {
            let found: Option<String> = sqlx::query_scalar(
                "SELECT name FROM sqlite_master WHERE type = 'index' AND name = ?",
            )
            .bind(name)
            .fetch_optional(&pool)
            .await
            .unwrap();
            assert_eq!(found.as_deref(), Some(name));
        }
        pool.close().await;
    }

    #[tokio::test]
    async fn open_db_adds_lookup_indexes_to_existing_schema() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        sqlx::query("DROP INDEX IF EXISTS idx_runs_task_id")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DROP INDEX IF EXISTS idx_links_task_id")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DROP INDEX IF EXISTS idx_links_kind_value")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("PRAGMA user_version = 0")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
        let pool = open_db(tmp.path()).await.unwrap();
        let found: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'index' AND name = 'idx_runs_task_id'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert_eq!(found.as_deref(), Some("idx_runs_task_id"));
        pool.close().await;
    }

    #[tokio::test]
    async fn open_db_adds_worktree_columns_to_existing_schema() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        sqlx::query("ALTER TABLE tasks DROP COLUMN worktree_path")
            .execute(&pool)
            .await
            .unwrap();
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
        let name: Option<String> = sqlx::query_scalar(
            "SELECT name FROM pragma_table_info('tasks') WHERE name = 'worktree_path'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert_eq!(name.as_deref(), Some("worktree_path"));
        pool.close().await;
    }

    #[tokio::test]
    async fn open_db_adds_checks_to_existing_schema() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        sqlx::query("DROP TABLE checks")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("PRAGMA user_version = 0")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;
        let pool = open_db(tmp.path()).await.unwrap();
        let name: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'checks'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert_eq!(name.as_deref(), Some("checks"));
        pool.close().await;
    }

    #[tokio::test]
    async fn open_db_adds_comments_to_existing_schema() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        sqlx::query("DROP TABLE comments")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("PRAGMA user_version = 0")
            .execute(&pool)
            .await
            .unwrap();
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

    #[tokio::test]
    async fn failed_statement_inside_transaction_does_not_commit_partial_schema() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("test.sqlite3");
        let pool = SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .filename(&db)
                .create_if_missing(true),
        )
        .await
        .unwrap();

        let mut tx = pool.begin().await.unwrap();
        let broken = "CREATE TABLE counters (name TEXT PRIMARY KEY);\nNOT VALID SQL;";
        let result = sqlx::raw_sql(broken).execute(&mut *tx).await;
        assert!(result.is_err());
        tx.rollback().await.unwrap();

        let counters: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'counters'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert!(counters.is_none());
        pool.close().await;
    }

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
        assert!(!backups.exists() || fs::read_dir(&backups).unwrap().next().is_none());
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

    #[tokio::test]
    async fn failed_version_step_keeps_user_version() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        let err = apply_sql_version(&pool, "NOT VALID SQL;", 9)
            .await
            .unwrap_err();
        assert!(!err.to_string().is_empty());
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(version, 5);
        pool.close().await;
    }

    #[tokio::test]
    async fn open_db_rotates_a_full_log() {
        let tmp = tempfile::tempdir().unwrap();
        let logs = tmp.path().join("logs");
        fs::create_dir_all(&logs).unwrap();
        fs::write(logs.join("taskboard.log"), vec![b'x'; 1_048_576]).unwrap();
        let pool = open_db(tmp.path()).await.unwrap();
        pool.close().await;
        let active = fs::read_to_string(logs.join("taskboard.log")).unwrap();
        assert!(active.contains("opening database"));
        assert!(active.len() < 1_048_576);
        assert!(logs.join("taskboard.log.1").metadata().unwrap().len() >= 1_048_576);
    }

    #[test]
    fn write_log_skips_forbidden_substrings() {
        let tmp = tempfile::tempdir().unwrap();
        let log_path = tmp.path().join("taskboard.log");
        write_log(&log_path, "info", "info", "safe message");
        write_log(&log_path, "info", "info", "contains note_markdown field");
        write_log(&log_path, "info", "info", "contains repo_path field");
        write_log(&log_path, "info", "info", "contains summary field");

        let contents = fs::read_to_string(&log_path).unwrap();
        assert!(contents.contains("safe message"));
        assert!(!contents.contains("note_markdown"));
        assert!(!contents.contains("repo_path"));
        assert!(!contents.contains("summary field"));
    }
}
