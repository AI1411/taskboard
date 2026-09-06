use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, SqlitePool};
use taskboard_application::{AppError, NewActivity, Store};
use taskboard_core::{
    Activity, ActorKind, Column, EntityType, Link, LinkKind, Project, Run, RunStatus, Task,
};
use uuid::Uuid;

use crate::db::map_sqlx;

const PROJECT_COLUMNS: &str = "id, slug, name, repo_path, archived, note_markdown, sort_order, revision, created_at, updated_at, deleted_at";
const TASK_COLUMNS: &str = "id, display_id, project_id, title, column, urgent, note_markdown, position, revision, created_at, updated_at, deleted_at";
const LINK_COLUMNS: &str = "id, task_id, kind, value, sort_order";
const RUN_COLUMNS: &str = "id, display_id, task_id, agent, session_id, status, message, waiting_reason, summary, started_at, ended_at, revision, created_at, updated_at";
const ACTIVITY_COLUMNS: &str = "id, sequence, actor_kind, actor_label, operation, entity_type, entity_id, previous_revision, before_json, after_json, created_at";

macro_rules! run {
    ($self:expr, $query:expr, $method:ident) => {
        if let Some(conn) = $self.conn.as_mut() {
            $query.$method(&mut **conn).await
        } else {
            $query
                .$method($self.pool.as_ref().expect("sqlite pool is closed"))
                .await
        }
    };
}

pub struct SqliteStore {
    pool: Option<SqlitePool>,
    conn: Option<sqlx::pool::PoolConnection<sqlx::Sqlite>>,
    data_dir: PathBuf,
}

impl SqliteStore {
    pub fn new(pool: SqlitePool, data_dir: impl Into<PathBuf>) -> Self {
        Self {
            pool: Some(pool),
            conn: None,
            data_dir: data_dir.into(),
        }
    }

    pub fn with_data_dir(pool: SqlitePool, data_dir: impl Into<PathBuf>) -> Self {
        Self::new(pool, data_dir)
    }

    async fn purge_task_row(&mut self, task_id: Uuid) -> Result<(), AppError> {
        let bytes = uuid_bytes(task_id);
        let null_links = sqlx::query(
            "UPDATE activities SET before_json = NULL, after_json = NULL WHERE entity_id IN (SELECT id FROM links WHERE task_id = ?)",
        )
        .bind(&bytes);
        run!(self, null_links, execute).map_err(map_sqlx)?;
        let null_runs = sqlx::query(
            "UPDATE activities SET before_json = NULL, after_json = NULL WHERE entity_id IN (SELECT id FROM runs WHERE task_id = ?)",
        )
        .bind(&bytes);
        run!(self, null_runs, execute).map_err(map_sqlx)?;
        let null_task = sqlx::query(
            "UPDATE activities SET before_json = NULL, after_json = NULL WHERE entity_id = ?",
        )
        .bind(&bytes);
        run!(self, null_task, execute).map_err(map_sqlx)?;
        let delete_links = sqlx::query("DELETE FROM links WHERE task_id = ?").bind(&bytes);
        run!(self, delete_links, execute).map_err(map_sqlx)?;
        let delete_runs = sqlx::query("DELETE FROM runs WHERE task_id = ?").bind(&bytes);
        run!(self, delete_runs, execute).map_err(map_sqlx)?;
        let delete_task = sqlx::query("DELETE FROM tasks WHERE id = ?").bind(&bytes);
        run!(self, delete_task, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn purge_project_row(&mut self, project_id: Uuid) -> Result<(), AppError> {
        let bytes = uuid_bytes(project_id);
        let null_project = sqlx::query(
            "UPDATE activities SET before_json = NULL, after_json = NULL WHERE entity_id = ?",
        )
        .bind(&bytes);
        run!(self, null_project, execute).map_err(map_sqlx)?;
        let delete_project = sqlx::query("DELETE FROM projects WHERE id = ?").bind(&bytes);
        run!(self, delete_project, execute).map_err(map_sqlx)?;
        Ok(())
    }
}

fn fmt_dt(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_dt(raw: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| AppError::Io(err.to_string()))
}

fn uuid_from_blob(bytes: &[u8]) -> Result<Uuid, AppError> {
    Uuid::from_slice(bytes).map_err(|err| AppError::Io(err.to_string()))
}

fn uuid_bytes(id: Uuid) -> Vec<u8> {
    id.as_bytes().to_vec()
}

fn sidecar(live: &Path, suffix: &str) -> PathBuf {
    let mut name = live.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

fn copy_over_sqlite(live: &Path, src: &Path) -> Result<(), AppError> {
    let tmp = live.with_file_name(".import-incoming.sqlite3");
    if tmp.exists() {
        fs::remove_file(&tmp).map_err(|err| AppError::Io(err.to_string()))?;
    }
    fs::copy(src, &tmp).map_err(|err| AppError::Io(err.to_string()))?;
    fs::rename(&tmp, live).map_err(|err| AppError::Io(err.to_string()))?;
    let _ = fs::remove_file(sidecar(live, "-wal"));
    let _ = fs::remove_file(sidecar(live, "-shm"));
    Ok(())
}

fn not_taskboard() -> AppError {
    AppError::Io("file is not a Taskboard database".into())
}

async fn assert_incoming_is_taskboard(src: PathBuf) -> Result<(), AppError> {
    let options = SqliteConnectOptions::new()
        .filename(&src)
        .create_if_missing(false)
        .read_only(true);
    let pool = match SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
    {
        Ok(pool) => pool,
        Err(err) => return Err(map_sqlx(err)),
    };
    let name: Result<Option<String>, _> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'projects'",
    )
    .fetch_optional(&pool)
    .await;
    let name = match name {
        Ok(name) => name,
        Err(err) => {
            pool.close().await;
            return Err(map_sqlx(err));
        }
    };
    if name.is_none() {
        pool.close().await;
        return Err(not_taskboard());
    }
    let sql = format!("SELECT {PROJECT_COLUMNS} FROM projects LIMIT 0");
    let probe = sqlx::query(&sql).fetch_optional(&pool).await;
    pool.close().await;
    match probe {
        Ok(_) => Ok(()),
        Err(_) => Err(not_taskboard()),
    }
}

fn map_constraint(err: sqlx::Error, slug: &str) -> AppError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.message().contains("UNIQUE") {
            return AppError::DuplicateSlug {
                slug: slug.to_string(),
            };
        }
    }
    map_sqlx(err)
}

fn enum_str<T: serde::Serialize>(value: T) -> Result<String, AppError> {
    match serde_json::to_value(value).map_err(|err| AppError::Io(err.to_string()))? {
        serde_json::Value::String(s) => Ok(s),
        other => Err(AppError::Io(format!("expected enum string, got {other}"))),
    }
}

fn parse_enum<T: serde::de::DeserializeOwned>(raw: &str) -> Result<T, AppError> {
    serde_json::from_value(serde_json::Value::String(raw.to_string()))
        .map_err(|err| AppError::Io(err.to_string()))
}

fn project_from_row(row: &SqliteRow) -> Result<Project, AppError> {
    let id: Vec<u8> = row.try_get("id").map_err(map_sqlx)?;
    let created_at: String = row.try_get("created_at").map_err(map_sqlx)?;
    let updated_at: String = row.try_get("updated_at").map_err(map_sqlx)?;
    let deleted_at: Option<String> = row.try_get("deleted_at").map_err(map_sqlx)?;
    let archived: i64 = row.try_get("archived").map_err(map_sqlx)?;
    Ok(Project {
        id: uuid_from_blob(&id)?,
        slug: row.try_get("slug").map_err(map_sqlx)?,
        name: row.try_get("name").map_err(map_sqlx)?,
        repo_path: row.try_get("repo_path").map_err(map_sqlx)?,
        archived: archived != 0,
        note_markdown: row.try_get("note_markdown").map_err(map_sqlx)?,
        sort_order: row.try_get("sort_order").map_err(map_sqlx)?,
        revision: row.try_get("revision").map_err(map_sqlx)?,
        created_at: parse_dt(&created_at)?,
        updated_at: parse_dt(&updated_at)?,
        deleted_at: deleted_at.as_deref().map(parse_dt).transpose()?,
    })
}

fn activity_from_row(row: &SqliteRow) -> Result<Activity, AppError> {
    let id: Vec<u8> = row.try_get("id").map_err(map_sqlx)?;
    let entity_id: Vec<u8> = row.try_get("entity_id").map_err(map_sqlx)?;
    let actor_kind: String = row.try_get("actor_kind").map_err(map_sqlx)?;
    let entity_type: String = row.try_get("entity_type").map_err(map_sqlx)?;
    let before_json: Option<String> = row.try_get("before_json").map_err(map_sqlx)?;
    let after_json: Option<String> = row.try_get("after_json").map_err(map_sqlx)?;
    let created_at: String = row.try_get("created_at").map_err(map_sqlx)?;
    Ok(Activity {
        id: uuid_from_blob(&id)?,
        sequence: row.try_get("sequence").map_err(map_sqlx)?,
        actor_kind: parse_enum::<ActorKind>(&actor_kind)?,
        actor_label: row.try_get("actor_label").map_err(map_sqlx)?,
        operation: row.try_get("operation").map_err(map_sqlx)?,
        entity_type: parse_enum::<EntityType>(&entity_type)?,
        entity_id: uuid_from_blob(&entity_id)?,
        previous_revision: row.try_get("previous_revision").map_err(map_sqlx)?,
        before_json: before_json
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|err| AppError::Io(err.to_string()))?,
        after_json: after_json
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|err| AppError::Io(err.to_string()))?,
        created_at: parse_dt(&created_at)?,
    })
}

fn task_from_row(row: &SqliteRow) -> Result<Task, AppError> {
    let id: Vec<u8> = row.try_get("id").map_err(map_sqlx)?;
    let project_id: Vec<u8> = row.try_get("project_id").map_err(map_sqlx)?;
    let column: String = row.try_get("column").map_err(map_sqlx)?;
    let urgent: i64 = row.try_get("urgent").map_err(map_sqlx)?;
    let created_at: String = row.try_get("created_at").map_err(map_sqlx)?;
    let updated_at: String = row.try_get("updated_at").map_err(map_sqlx)?;
    let deleted_at: Option<String> = row.try_get("deleted_at").map_err(map_sqlx)?;
    Ok(Task {
        id: uuid_from_blob(&id)?,
        display_id: row.try_get("display_id").map_err(map_sqlx)?,
        project_id: uuid_from_blob(&project_id)?,
        title: row.try_get("title").map_err(map_sqlx)?,
        column: parse_enum(&column)?,
        urgent: urgent != 0,
        note_markdown: row.try_get("note_markdown").map_err(map_sqlx)?,
        position: row.try_get("position").map_err(map_sqlx)?,
        revision: row.try_get("revision").map_err(map_sqlx)?,
        created_at: parse_dt(&created_at)?,
        updated_at: parse_dt(&updated_at)?,
        deleted_at: deleted_at.as_deref().map(parse_dt).transpose()?,
    })
}

fn link_from_row(row: &SqliteRow) -> Result<Link, AppError> {
    let id: Vec<u8> = row.try_get("id").map_err(map_sqlx)?;
    let task_id: Vec<u8> = row.try_get("task_id").map_err(map_sqlx)?;
    let kind: String = row.try_get("kind").map_err(map_sqlx)?;
    Ok(Link {
        id: uuid_from_blob(&id)?,
        task_id: uuid_from_blob(&task_id)?,
        kind: parse_enum::<LinkKind>(&kind)?,
        value: row.try_get("value").map_err(map_sqlx)?,
        sort_order: row.try_get("sort_order").map_err(map_sqlx)?,
    })
}

fn run_from_row(row: &SqliteRow) -> Result<Run, AppError> {
    let id: Vec<u8> = row.try_get("id").map_err(map_sqlx)?;
    let task_id: Vec<u8> = row.try_get("task_id").map_err(map_sqlx)?;
    let status: String = row.try_get("status").map_err(map_sqlx)?;
    let started_at: String = row.try_get("started_at").map_err(map_sqlx)?;
    let ended_at: Option<String> = row.try_get("ended_at").map_err(map_sqlx)?;
    let created_at: String = row.try_get("created_at").map_err(map_sqlx)?;
    let updated_at: String = row.try_get("updated_at").map_err(map_sqlx)?;
    Ok(Run {
        id: uuid_from_blob(&id)?,
        display_id: row.try_get("display_id").map_err(map_sqlx)?,
        task_id: uuid_from_blob(&task_id)?,
        agent: row.try_get("agent").map_err(map_sqlx)?,
        session_id: row.try_get("session_id").map_err(map_sqlx)?,
        status: parse_enum::<RunStatus>(&status)?,
        message: row.try_get("message").map_err(map_sqlx)?,
        waiting_reason: row.try_get("waiting_reason").map_err(map_sqlx)?,
        summary: row.try_get("summary").map_err(map_sqlx)?,
        started_at: parse_dt(&started_at)?,
        ended_at: ended_at.as_deref().map(parse_dt).transpose()?,
        revision: row.try_get("revision").map_err(map_sqlx)?,
        created_at: parse_dt(&created_at)?,
        updated_at: parse_dt(&updated_at)?,
    })
}

#[async_trait]
impl Store for SqliteStore {
    async fn next_display_n(&mut self, counter: &str) -> Result<i64, AppError> {
        let query = sqlx::query_scalar(
            "UPDATE counters SET value = value + 1 WHERE name = ? RETURNING value",
        )
        .bind(counter);
        let value: Option<i64> = run!(self, query, fetch_optional).map_err(map_sqlx)?;
        value.ok_or_else(|| AppError::Io(format!("unknown counter `{counter}`")))
    }

    async fn next_activity_sequence(&mut self) -> Result<i64, AppError> {
        self.next_display_n("activity").await
    }

    async fn insert_activity(&mut self, row: NewActivity) -> Result<(), AppError> {
        let query = sqlx::query(
            "INSERT INTO activities (id, sequence, actor_kind, actor_label, operation, entity_type, entity_id, previous_revision, before_json, after_json, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid_bytes(row.id))
        .bind(row.sequence)
        .bind(enum_str(row.actor_kind)?)
        .bind(&row.actor_label)
        .bind(&row.operation)
        .bind(enum_str(row.entity_type)?)
        .bind(uuid_bytes(row.entity_id))
        .bind(row.previous_revision)
        .bind(row.before_json.as_ref().map(|v| v.to_string()))
        .bind(row.after_json.as_ref().map(|v| v.to_string()))
        .bind(fmt_dt(row.created_at));
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn get_project(&mut self, id: Uuid) -> Result<Option<Project>, AppError> {
        let sql = format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE id = ?");
        let query = sqlx::query(&sql).bind(uuid_bytes(id));
        let row = run!(self, query, fetch_optional).map_err(map_sqlx)?;
        row.as_ref().map(project_from_row).transpose()
    }

    async fn get_project_by_slug(
        &mut self,
        slug: &str,
        include_deleted: bool,
    ) -> Result<Option<Project>, AppError> {
        let sql = if include_deleted {
            format!(
                "SELECT {PROJECT_COLUMNS} FROM projects WHERE slug = ? ORDER BY CASE WHEN deleted_at IS NULL THEN 0 ELSE 1 END, deleted_at DESC LIMIT 1"
            )
        } else {
            format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE slug = ? AND deleted_at IS NULL")
        };
        let query = sqlx::query(&sql).bind(slug);
        let row = run!(self, query, fetch_optional).map_err(map_sqlx)?;
        row.as_ref().map(project_from_row).transpose()
    }

    async fn list_projects(&mut self, include_archived: bool) -> Result<Vec<Project>, AppError> {
        let sql = format!(
            "SELECT {PROJECT_COLUMNS} FROM projects WHERE deleted_at IS NULL AND (archived = 0 OR ?) ORDER BY sort_order ASC"
        );
        let query = sqlx::query(&sql).bind(if include_archived { 1i64 } else { 0 });
        let rows = run!(self, query, fetch_all).map_err(map_sqlx)?;
        rows.iter().map(project_from_row).collect()
    }

    async fn list_deleted_projects(&mut self) -> Result<Vec<Project>, AppError> {
        let sql = format!(
            "SELECT {PROJECT_COLUMNS} FROM projects WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC"
        );
        let query = sqlx::query(&sql);
        let rows = run!(self, query, fetch_all).map_err(map_sqlx)?;
        rows.iter().map(project_from_row).collect()
    }

    async fn insert_project(&mut self, project: &Project) -> Result<(), AppError> {
        let query = sqlx::query(
            "INSERT INTO projects (id, slug, name, repo_path, archived, note_markdown, sort_order, revision, created_at, updated_at, deleted_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid_bytes(project.id))
        .bind(&project.slug)
        .bind(&project.name)
        .bind(&project.repo_path)
        .bind(if project.archived { 1i64 } else { 0 })
        .bind(&project.note_markdown)
        .bind(project.sort_order)
        .bind(project.revision)
        .bind(fmt_dt(project.created_at))
        .bind(fmt_dt(project.updated_at))
        .bind(project.deleted_at.map(fmt_dt));
        run!(self, query, execute).map_err(|err| map_constraint(err, &project.slug))?;
        Ok(())
    }

    async fn update_project(&mut self, project: &Project) -> Result<(), AppError> {
        let query = sqlx::query(
            "UPDATE projects SET slug = ?, name = ?, repo_path = ?, archived = ?, note_markdown = ?, sort_order = ?, revision = ?, created_at = ?, updated_at = ?, deleted_at = ? WHERE id = ?",
        )
        .bind(&project.slug)
        .bind(&project.name)
        .bind(&project.repo_path)
        .bind(if project.archived { 1i64 } else { 0 })
        .bind(&project.note_markdown)
        .bind(project.sort_order)
        .bind(project.revision)
        .bind(fmt_dt(project.created_at))
        .bind(fmt_dt(project.updated_at))
        .bind(project.deleted_at.map(fmt_dt))
        .bind(uuid_bytes(project.id));
        run!(self, query, execute).map_err(|err| map_constraint(err, &project.slug))?;
        Ok(())
    }

    async fn soft_delete_project(
        &mut self,
        id: Uuid,
        deleted_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let query = sqlx::query("UPDATE projects SET deleted_at = ? WHERE id = ?")
            .bind(fmt_dt(deleted_at))
            .bind(uuid_bytes(id));
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn restore_project(&mut self, id: Uuid) -> Result<(), AppError> {
        let query =
            sqlx::query("UPDATE projects SET deleted_at = NULL WHERE id = ?").bind(uuid_bytes(id));
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn rewrite_project_sort_orders(&mut self, ordered_ids: &[Uuid]) -> Result<(), AppError> {
        for (index, id) in ordered_ids.iter().enumerate() {
            let query = sqlx::query("UPDATE projects SET sort_order = ? WHERE id = ?")
                .bind(index as i64)
                .bind(uuid_bytes(*id));
            run!(self, query, execute).map_err(map_sqlx)?;
        }
        Ok(())
    }

    async fn get_task(&mut self, id: Uuid) -> Result<Option<Task>, AppError> {
        let sql = format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = ?");
        let query = sqlx::query(&sql).bind(uuid_bytes(id));
        let row = run!(self, query, fetch_optional).map_err(map_sqlx)?;
        row.as_ref().map(task_from_row).transpose()
    }

    async fn get_task_by_display_id(
        &mut self,
        display_id: &str,
        include_deleted: bool,
    ) -> Result<Option<Task>, AppError> {
        let sql = if include_deleted {
            format!(
                "SELECT {TASK_COLUMNS} FROM tasks WHERE display_id = ? ORDER BY CASE WHEN deleted_at IS NULL THEN 0 ELSE 1 END, deleted_at DESC LIMIT 1"
            )
        } else {
            format!("SELECT {TASK_COLUMNS} FROM tasks WHERE display_id = ? AND deleted_at IS NULL")
        };
        let query = sqlx::query(&sql).bind(display_id);
        let row = run!(self, query, fetch_optional).map_err(map_sqlx)?;
        row.as_ref().map(task_from_row).transpose()
    }

    async fn list_tasks(&mut self, project_id: Uuid) -> Result<Vec<Task>, AppError> {
        let sql = format!(
            "SELECT {TASK_COLUMNS} FROM tasks WHERE project_id = ? AND deleted_at IS NULL ORDER BY urgent DESC, position ASC"
        );
        let query = sqlx::query(&sql).bind(uuid_bytes(project_id));
        let rows = run!(self, query, fetch_all).map_err(map_sqlx)?;
        rows.iter().map(task_from_row).collect()
    }

    async fn list_deleted_tasks(&mut self) -> Result<Vec<Task>, AppError> {
        let sql = format!(
            "SELECT {TASK_COLUMNS} FROM tasks WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC"
        );
        let query = sqlx::query(&sql);
        let rows = run!(self, query, fetch_all).map_err(map_sqlx)?;
        rows.iter().map(task_from_row).collect()
    }

    async fn insert_task(&mut self, task: &Task) -> Result<(), AppError> {
        let query = sqlx::query(
            "INSERT INTO tasks (id, display_id, project_id, title, column, urgent, note_markdown, position, revision, created_at, updated_at, deleted_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid_bytes(task.id))
        .bind(&task.display_id)
        .bind(uuid_bytes(task.project_id))
        .bind(&task.title)
        .bind(enum_str(task.column)?)
        .bind(if task.urgent { 1i64 } else { 0 })
        .bind(&task.note_markdown)
        .bind(task.position)
        .bind(task.revision)
        .bind(fmt_dt(task.created_at))
        .bind(fmt_dt(task.updated_at))
        .bind(task.deleted_at.map(fmt_dt));
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn update_task(&mut self, task: &Task) -> Result<(), AppError> {
        let query = sqlx::query(
            "UPDATE tasks SET display_id = ?, project_id = ?, title = ?, column = ?, urgent = ?, note_markdown = ?, position = ?, revision = ?, created_at = ?, updated_at = ?, deleted_at = ? WHERE id = ?",
        )
        .bind(&task.display_id)
        .bind(uuid_bytes(task.project_id))
        .bind(&task.title)
        .bind(enum_str(task.column)?)
        .bind(if task.urgent { 1i64 } else { 0 })
        .bind(&task.note_markdown)
        .bind(task.position)
        .bind(task.revision)
        .bind(fmt_dt(task.created_at))
        .bind(fmt_dt(task.updated_at))
        .bind(task.deleted_at.map(fmt_dt))
        .bind(uuid_bytes(task.id));
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn soft_delete_task(
        &mut self,
        id: Uuid,
        deleted_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let query = sqlx::query("UPDATE tasks SET deleted_at = ? WHERE id = ?")
            .bind(fmt_dt(deleted_at))
            .bind(uuid_bytes(id));
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn restore_task(&mut self, id: Uuid) -> Result<(), AppError> {
        let query =
            sqlx::query("UPDATE tasks SET deleted_at = NULL WHERE id = ?").bind(uuid_bytes(id));
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn rewrite_task_positions(
        &mut self,
        project_id: Uuid,
        column: Column,
        ordered_ids: &[Uuid],
    ) -> Result<(), AppError> {
        let column = enum_str(column)?;
        let negate = sqlx::query(
            "UPDATE tasks SET position = -(position + 1) WHERE project_id = ? AND column = ? AND deleted_at IS NULL",
        )
        .bind(uuid_bytes(project_id))
        .bind(&column);
        run!(self, negate, execute).map_err(map_sqlx)?;
        for (index, id) in ordered_ids.iter().enumerate() {
            let query = sqlx::query("UPDATE tasks SET position = ? WHERE id = ?")
                .bind(index as i64)
                .bind(uuid_bytes(*id));
            run!(self, query, execute).map_err(map_sqlx)?;
        }
        Ok(())
    }

    async fn get_link(&mut self, id: Uuid) -> Result<Option<Link>, AppError> {
        let sql = format!("SELECT {LINK_COLUMNS} FROM links WHERE id = ?");
        let query = sqlx::query(&sql).bind(uuid_bytes(id));
        let row = run!(self, query, fetch_optional).map_err(map_sqlx)?;
        row.as_ref().map(link_from_row).transpose()
    }

    async fn list_links(&mut self, task_id: Uuid) -> Result<Vec<Link>, AppError> {
        let sql =
            format!("SELECT {LINK_COLUMNS} FROM links WHERE task_id = ? ORDER BY sort_order ASC");
        let query = sqlx::query(&sql).bind(uuid_bytes(task_id));
        let rows = run!(self, query, fetch_all).map_err(map_sqlx)?;
        rows.iter().map(link_from_row).collect()
    }

    async fn insert_link(&mut self, link: &Link) -> Result<(), AppError> {
        let query = sqlx::query(
            "INSERT INTO links (id, task_id, kind, value, sort_order) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(uuid_bytes(link.id))
        .bind(uuid_bytes(link.task_id))
        .bind(enum_str(link.kind)?)
        .bind(&link.value)
        .bind(link.sort_order);
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn update_link(&mut self, link: &Link) -> Result<(), AppError> {
        let query = sqlx::query(
            "UPDATE links SET task_id = ?, kind = ?, value = ?, sort_order = ? WHERE id = ?",
        )
        .bind(uuid_bytes(link.task_id))
        .bind(enum_str(link.kind)?)
        .bind(&link.value)
        .bind(link.sort_order)
        .bind(uuid_bytes(link.id));
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn delete_link(&mut self, id: Uuid) -> Result<(), AppError> {
        let query = sqlx::query("DELETE FROM links WHERE id = ?").bind(uuid_bytes(id));
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn get_run(&mut self, id: Uuid) -> Result<Option<Run>, AppError> {
        let sql = format!("SELECT {RUN_COLUMNS} FROM runs WHERE id = ?");
        let query = sqlx::query(&sql).bind(uuid_bytes(id));
        let row = run!(self, query, fetch_optional).map_err(map_sqlx)?;
        row.as_ref().map(run_from_row).transpose()
    }

    async fn get_run_by_display_id(&mut self, display_id: &str) -> Result<Option<Run>, AppError> {
        let sql = format!("SELECT {RUN_COLUMNS} FROM runs WHERE display_id = ?");
        let query = sqlx::query(&sql).bind(display_id);
        let row = run!(self, query, fetch_optional).map_err(map_sqlx)?;
        row.as_ref().map(run_from_row).transpose()
    }

    async fn list_runs(&mut self, task_id: Uuid) -> Result<Vec<Run>, AppError> {
        let sql = format!(
            "SELECT {RUN_COLUMNS} FROM runs WHERE task_id = ? ORDER BY started_at DESC, display_id DESC"
        );
        let query = sqlx::query(&sql).bind(uuid_bytes(task_id));
        let rows = run!(self, query, fetch_all).map_err(map_sqlx)?;
        rows.iter().map(run_from_row).collect()
    }

    async fn insert_run(&mut self, run: &Run) -> Result<(), AppError> {
        let query = sqlx::query(
            "INSERT INTO runs (id, display_id, task_id, agent, session_id, status, message, waiting_reason, summary, started_at, ended_at, revision, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid_bytes(run.id))
        .bind(&run.display_id)
        .bind(uuid_bytes(run.task_id))
        .bind(&run.agent)
        .bind(&run.session_id)
        .bind(enum_str(run.status)?)
        .bind(&run.message)
        .bind(&run.waiting_reason)
        .bind(&run.summary)
        .bind(fmt_dt(run.started_at))
        .bind(run.ended_at.map(fmt_dt))
        .bind(run.revision)
        .bind(fmt_dt(run.created_at))
        .bind(fmt_dt(run.updated_at));
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn update_run(&mut self, run: &Run) -> Result<(), AppError> {
        let query = sqlx::query(
            "UPDATE runs SET display_id = ?, task_id = ?, agent = ?, session_id = ?, status = ?, message = ?, waiting_reason = ?, summary = ?, started_at = ?, ended_at = ?, revision = ?, created_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&run.display_id)
        .bind(uuid_bytes(run.task_id))
        .bind(&run.agent)
        .bind(&run.session_id)
        .bind(enum_str(run.status)?)
        .bind(&run.message)
        .bind(&run.waiting_reason)
        .bind(&run.summary)
        .bind(fmt_dt(run.started_at))
        .bind(run.ended_at.map(fmt_dt))
        .bind(run.revision)
        .bind(fmt_dt(run.created_at))
        .bind(fmt_dt(run.updated_at))
        .bind(uuid_bytes(run.id));
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn delete_run(&mut self, id: Uuid) -> Result<(), AppError> {
        let query = sqlx::query("DELETE FROM runs WHERE id = ?").bind(uuid_bytes(id));
        run!(self, query, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn activity_head(&mut self) -> Result<i64, AppError> {
        let query = sqlx::query_scalar("SELECT COALESCE(MAX(sequence), 0) FROM activities");
        run!(self, query, fetch_one).map_err(map_sqlx)
    }

    async fn latest_activity_for(&mut self, entity_id: Uuid) -> Result<Option<Activity>, AppError> {
        let sql = format!(
            "SELECT {ACTIVITY_COLUMNS} FROM activities WHERE entity_id = ? ORDER BY sequence DESC LIMIT 1"
        );
        let query = sqlx::query(&sql).bind(uuid_bytes(entity_id));
        let row = run!(self, query, fetch_optional).map_err(map_sqlx)?;
        row.as_ref().map(activity_from_row).transpose()
    }

    async fn latest_undoable(&mut self) -> Result<Option<Activity>, AppError> {
        let sql = format!(
            "SELECT {ACTIVITY_COLUMNS} FROM activities WHERE operation != 'undo' ORDER BY sequence DESC LIMIT 1"
        );
        let query = sqlx::query(&sql);
        let row = run!(self, query, fetch_optional).map_err(map_sqlx)?;
        row.as_ref().map(activity_from_row).transpose()
    }

    async fn list_activities_after(&mut self, sequence: i64) -> Result<Vec<Activity>, AppError> {
        let sql = format!(
            "SELECT {ACTIVITY_COLUMNS} FROM activities WHERE sequence > ? ORDER BY sequence ASC"
        );
        let query = sqlx::query(&sql).bind(sequence);
        let rows = run!(self, query, fetch_all).map_err(map_sqlx)?;
        rows.iter().map(activity_from_row).collect()
    }

    async fn list_recent_activities(
        &mut self,
        entity_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Activity>, AppError> {
        let sql = format!(
            "SELECT {ACTIVITY_COLUMNS} FROM activities WHERE entity_id = ? ORDER BY sequence DESC LIMIT ?"
        );
        let query = sqlx::query(&sql).bind(uuid_bytes(entity_id)).bind(limit);
        let rows = run!(self, query, fetch_all).map_err(map_sqlx)?;
        rows.iter().map(activity_from_row).collect()
    }

    async fn purge_expired(
        &mut self,
        now: DateTime<Utc>,
        retention_days: i64,
    ) -> Result<u64, AppError> {
        let cutoff = now - chrono::Duration::days(retention_days);
        let cutoff_s = fmt_dt(cutoff);
        let mut task_ids = Vec::new();
        let task_query =
            sqlx::query("SELECT id FROM tasks WHERE deleted_at IS NOT NULL AND deleted_at <= ?")
                .bind(&cutoff_s);
        let task_rows = run!(self, task_query, fetch_all).map_err(map_sqlx)?;
        for row in &task_rows {
            let id: Vec<u8> = row.try_get("id").map_err(map_sqlx)?;
            task_ids.push(uuid_from_blob(&id)?);
        }

        let project_query =
            sqlx::query("SELECT id FROM projects WHERE deleted_at IS NOT NULL AND deleted_at <= ?")
                .bind(&cutoff_s);
        let project_rows = run!(self, project_query, fetch_all).map_err(map_sqlx)?;
        let mut project_ids = Vec::new();
        for row in &project_rows {
            let id: Vec<u8> = row.try_get("id").map_err(map_sqlx)?;
            project_ids.push(uuid_from_blob(&id)?);
        }

        for project_id in &project_ids {
            let child_query = sqlx::query("SELECT id FROM tasks WHERE project_id = ?")
                .bind(uuid_bytes(*project_id));
            let child_rows = run!(self, child_query, fetch_all).map_err(map_sqlx)?;
            for row in &child_rows {
                let id: Vec<u8> = row.try_get("id").map_err(map_sqlx)?;
                let task_id = uuid_from_blob(&id)?;
                if !task_ids.contains(&task_id) {
                    task_ids.push(task_id);
                }
            }
        }

        let mut purged = 0u64;
        for task_id in task_ids {
            self.purge_task_row(task_id).await?;
            purged += 1;
        }
        for project_id in project_ids {
            self.purge_project_row(project_id).await?;
            purged += 1;
        }
        Ok(purged)
    }

    async fn backup_to(&mut self, dest: &Path) -> Result<(), AppError> {
        let dest = dest.to_path_buf();
        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|err| AppError::Io(err.to_string()))?;
            }
        }
        if dest.exists() {
            fs::remove_file(&dest).map_err(|err| AppError::Io(err.to_string()))?;
        }
        let checkpoint = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)");
        run!(self, checkpoint, execute).map_err(map_sqlx)?;
        let dest_sql = dest.to_string_lossy().replace('\'', "''");
        let sql = format!("VACUUM INTO '{dest_sql}'");
        let vacuum = sqlx::query(&sql);
        run!(self, vacuum, execute).map_err(map_sqlx)?;
        Ok(())
    }

    async fn validate_import(&mut self, src: &Path) -> Result<(), AppError> {
        let src = src.to_path_buf();
        assert_incoming_is_taskboard(src).await
    }

    fn pre_import_path(&self) -> Result<PathBuf, AppError> {
        if self.data_dir.as_os_str().is_empty() {
            return Err(AppError::Io(
                "data directory is unknown; cannot import".into(),
            ));
        }
        let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");
        Ok(self
            .data_dir
            .join("backups")
            .join(format!("pre-import-{stamp}.sqlite3")))
    }

    async fn close_pool(&mut self) -> Result<(), AppError> {
        if self.conn.is_some() {
            return Err(AppError::Io("cannot import during a transaction".into()));
        }
        if let Some(pool) = self.pool.take() {
            pool.close().await;
        }
        Ok(())
    }

    async fn reopen_pool(&mut self) -> Result<(), AppError> {
        let data_dir = self.data_dir.clone();
        if data_dir.as_os_str().is_empty() {
            return Err(AppError::Io(
                "data directory is unknown; cannot import".into(),
            ));
        }
        let db_path = data_dir.join("taskboard.sqlite3");
        let cfg = crate::load_config(&data_dir);
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true)
            .busy_timeout(Duration::from_millis(cfg.busy_timeout_ms));
        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .map_err(map_sqlx)?;
        self.pool = Some(pool);
        Ok(())
    }

    async fn replace_from(&mut self, src: &Path) -> Result<(), AppError> {
        let src = src.to_path_buf();
        if self.conn.is_some() {
            return Err(AppError::Io("cannot import during a transaction".into()));
        }
        let data_dir = self.data_dir.clone();
        if data_dir.as_os_str().is_empty() {
            return Err(AppError::Io(
                "data directory is unknown; cannot import".into(),
            ));
        }
        copy_over_sqlite(&data_dir.join("taskboard.sqlite3"), &src)
    }

    async fn begin(&mut self) -> Result<(), AppError> {
        if self.conn.is_some() {
            return Err(AppError::Io("transaction already active".into()));
        }
        let mut conn = self
            .pool
            .as_ref()
            .ok_or_else(|| AppError::Io("database pool is closed".into()))?
            .acquire()
            .await
            .map_err(map_sqlx)?;
        sqlx::query("BEGIN IMMEDIATE")
            .execute(&mut *conn)
            .await
            .map_err(map_sqlx)?;
        self.conn = Some(conn);
        Ok(())
    }

    async fn commit(&mut self) -> Result<(), AppError> {
        let mut conn = self
            .conn
            .take()
            .ok_or_else(|| AppError::Io("no active transaction".into()))?;
        let result = sqlx::query("COMMIT")
            .execute(&mut *conn)
            .await
            .map_err(map_sqlx);
        if result.is_err() {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        }
        result?;
        Ok(())
    }

    async fn rollback(&mut self) -> Result<(), AppError> {
        let Some(mut conn) = self.conn.take() else {
            return Ok(());
        };
        sqlx::query("ROLLBACK")
            .execute(&mut *conn)
            .await
            .map_err(map_sqlx)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[tokio::test]
    async fn stores_uuid_as_blob16_and_timestamps_with_z() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = crate::open_db(tmp.path()).await.unwrap();
        let mut store = SqliteStore::new(pool.clone(), tmp.path());
        let now = Utc.with_ymd_and_hms(2026, 9, 5, 12, 0, 0).unwrap();
        let project = Project {
            id: Uuid::now_v7(),
            slug: "renai-sim".into(),
            name: "Renai Sim".into(),
            repo_path: None,
            archived: false,
            note_markdown: String::new(),
            sort_order: 0,
            revision: 1,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        store.insert_project(&project).await.unwrap();

        let id_type: String = sqlx::query_scalar("SELECT typeof(id) FROM projects")
            .fetch_one(&pool)
            .await
            .unwrap();
        let id_len: i64 = sqlx::query_scalar("SELECT length(id) FROM projects")
            .fetch_one(&pool)
            .await
            .unwrap();
        let created_at: String = sqlx::query_scalar("SELECT created_at FROM projects")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(id_type, "blob");
        assert_eq!(id_len, 16);
        assert_eq!(created_at, "2026-09-05T12:00:00.000Z");
        pool.close().await;
    }

    #[tokio::test]
    async fn link_and_run_insert_update_get_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let pool = crate::open_db(tmp.path()).await.unwrap();
        let mut store = SqliteStore::new(pool.clone(), tmp.path());
        let now = Utc.with_ymd_and_hms(2026, 9, 5, 12, 0, 0).unwrap();
        let project = Project {
            id: Uuid::now_v7(),
            slug: "renai-sim".into(),
            name: "Renai Sim".into(),
            repo_path: None,
            archived: false,
            note_markdown: String::new(),
            sort_order: 0,
            revision: 1,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        store.insert_project(&project).await.unwrap();
        let task = Task {
            id: Uuid::now_v7(),
            display_id: "TASK-1".into(),
            project_id: project.id,
            title: "Fix".into(),
            column: Column::Todo,
            urgent: false,
            note_markdown: String::new(),
            position: 0,
            revision: 1,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        store.insert_task(&task).await.unwrap();

        let mut link = Link {
            id: Uuid::now_v7(),
            task_id: task.id,
            kind: LinkKind::Url,
            value: "https://example.com".into(),
            sort_order: 0,
        };
        store.insert_link(&link).await.unwrap();
        link.value = "https://example.com/updated".into();
        store.update_link(&link).await.unwrap();
        let loaded = store.get_link(link.id).await.unwrap().unwrap();
        assert_eq!(loaded.value, "https://example.com/updated");
        store.delete_link(link.id).await.unwrap();
        assert!(store.get_link(link.id).await.unwrap().is_none());

        let mut run = Run {
            id: Uuid::now_v7(),
            display_id: "RUN-1".into(),
            task_id: task.id,
            agent: "codex".into(),
            session_id: Some("abc".into()),
            status: RunStatus::Running,
            message: None,
            waiting_reason: None,
            summary: None,
            started_at: now,
            ended_at: None,
            revision: 1,
            created_at: now,
            updated_at: now,
        };
        store.insert_run(&run).await.unwrap();
        run.status = RunStatus::Completed;
        run.summary = Some("done".into());
        run.ended_at = Some(now);
        store.update_run(&run).await.unwrap();
        let loaded = store.get_run_by_display_id("RUN-1").await.unwrap().unwrap();
        assert_eq!(loaded.status, RunStatus::Completed);
        assert_eq!(store.get_run(run.id).await.unwrap().unwrap().id, run.id);
        store.delete_run(run.id).await.unwrap();
        assert!(store.get_run(run.id).await.unwrap().is_none());
        pool.close().await;
    }
}
