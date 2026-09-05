use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use taskboard_application::AppError;

use crate::config::load_config;
use crate::migrations::INIT_SQL;

/// Opens `{data_dir}/taskboard.sqlite3`, applying `0001_init.sql` when `projects` is missing.
pub async fn open_db(data_dir: &Path) -> Result<SqlitePool, AppError> {
    fs::create_dir_all(data_dir).map_err(map_io)?;
    let logs_dir = data_dir.join("logs");
    fs::create_dir_all(&logs_dir).map_err(map_io)?;
    let log_path = create_log_file(&logs_dir)?;

    let cfg = load_config(data_dir);
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

    if !projects_table_exists(&pool).await? {
        if db_existed {
            backup_pre_migration(data_dir, &pool).await?;
        }
        sqlx::raw_sql(INIT_SQL)
            .execute(&pool)
            .await
            .map_err(map_sqlx)?;
        write_log(
            &log_path,
            &cfg.log_level,
            "info",
            "applied migration 0001_init",
        );
    }

    Ok(pool)
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

async fn backup_pre_migration(data_dir: &Path, pool: &SqlitePool) -> Result<(), AppError> {
    let backups = data_dir.join("backups");
    fs::create_dir_all(&backups).map_err(map_io)?;
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
    let dest = backups.join(format!("pre-migration-0001-{stamp}.sqlite3"));
    let dest_sql = dest.to_string_lossy().replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{dest_sql}'"))
        .execute(pool)
        .await
        .map_err(map_sqlx)?;
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

fn write_log(path: &Path, configured_level: &str, message_level: &str, message: &str) {
    // Diagnostic logs must never include note bodies, run summaries, or repository paths.
    if message.contains("note_markdown") || message.contains("repo_path") {
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

fn map_sqlx(err: sqlx::Error) -> AppError {
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
        assert_eq!(n, 3);
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
        assert_eq!(n, 3);
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
}
