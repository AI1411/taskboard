use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};
use taskboard_application::{AppError, NewActivity, Store};
use taskboard_core::{Activity, ActorKind, Column, EntityType, Link, Project, Run, Task};
use uuid::Uuid;

use crate::db::map_sqlx;

const PROJECT_COLUMNS: &str = "id, slug, name, repo_path, archived, note_markdown, sort_order, revision, created_at, updated_at, deleted_at";
const ACTIVITY_COLUMNS: &str = "id, sequence, actor_kind, actor_label, operation, entity_type, entity_id, previous_revision, before_json, after_json, created_at";

macro_rules! run {
    ($self:expr, $query:expr, $method:ident) => {
        if let Some(conn) = $self.conn.as_mut() {
            $query.$method(&mut **conn).await
        } else {
            $query.$method(&$self.pool).await
        }
    };
}

pub struct SqliteStore {
    pool: SqlitePool,
    conn: Option<sqlx::pool::PoolConnection<sqlx::Sqlite>>,
}

impl SqliteStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool, conn: None }
    }
}

fn not_impl(what: &str) -> AppError {
    AppError::Io(format!("{what} is not implemented"))
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

    async fn get_task(&mut self, _id: Uuid) -> Result<Option<Task>, AppError> {
        Err(not_impl("get_task"))
    }

    async fn get_task_by_display_id(
        &mut self,
        _display_id: &str,
        _include_deleted: bool,
    ) -> Result<Option<Task>, AppError> {
        Err(not_impl("get_task_by_display_id"))
    }

    async fn list_tasks(&mut self, _project_id: Uuid) -> Result<Vec<Task>, AppError> {
        Err(not_impl("list_tasks"))
    }

    async fn list_deleted_tasks(&mut self) -> Result<Vec<Task>, AppError> {
        Err(not_impl("list_deleted_tasks"))
    }

    async fn insert_task(&mut self, _task: &Task) -> Result<(), AppError> {
        Err(not_impl("insert_task"))
    }

    async fn update_task(&mut self, _task: &Task) -> Result<(), AppError> {
        Err(not_impl("update_task"))
    }

    async fn soft_delete_task(
        &mut self,
        _id: Uuid,
        _deleted_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        Err(not_impl("soft_delete_task"))
    }

    async fn restore_task(&mut self, _id: Uuid) -> Result<(), AppError> {
        Err(not_impl("restore_task"))
    }

    async fn rewrite_task_positions(
        &mut self,
        _project_id: Uuid,
        _column: Column,
        _ordered_ids: &[Uuid],
    ) -> Result<(), AppError> {
        Err(not_impl("rewrite_task_positions"))
    }

    async fn get_link(&mut self, _id: Uuid) -> Result<Option<Link>, AppError> {
        Err(not_impl("get_link"))
    }

    async fn list_links(&mut self, _task_id: Uuid) -> Result<Vec<Link>, AppError> {
        Err(not_impl("list_links"))
    }

    async fn insert_link(&mut self, _link: &Link) -> Result<(), AppError> {
        Err(not_impl("insert_link"))
    }

    async fn update_link(&mut self, _link: &Link) -> Result<(), AppError> {
        Err(not_impl("update_link"))
    }

    async fn delete_link(&mut self, _id: Uuid) -> Result<(), AppError> {
        Err(not_impl("delete_link"))
    }

    async fn get_run(&mut self, _id: Uuid) -> Result<Option<Run>, AppError> {
        Err(not_impl("get_run"))
    }

    async fn get_run_by_display_id(&mut self, _display_id: &str) -> Result<Option<Run>, AppError> {
        Err(not_impl("get_run_by_display_id"))
    }

    async fn list_runs(&mut self, _task_id: Uuid) -> Result<Vec<Run>, AppError> {
        Err(not_impl("list_runs"))
    }

    async fn insert_run(&mut self, _run: &Run) -> Result<(), AppError> {
        Err(not_impl("insert_run"))
    }

    async fn update_run(&mut self, _run: &Run) -> Result<(), AppError> {
        Err(not_impl("update_run"))
    }

    async fn delete_run(&mut self, _id: Uuid) -> Result<(), AppError> {
        Err(not_impl("delete_run"))
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
        _now: DateTime<Utc>,
        _retention_days: i64,
    ) -> Result<u64, AppError> {
        Err(not_impl("purge_expired"))
    }

    async fn backup_to(&mut self, _dest: &Path) -> Result<(), AppError> {
        Err(not_impl("backup_to"))
    }

    async fn replace_from(&mut self, _src: &Path) -> Result<(), AppError> {
        Err(not_impl("replace_from"))
    }

    async fn begin(&mut self) -> Result<(), AppError> {
        if self.conn.is_some() {
            return Err(AppError::Io("transaction already active".into()));
        }
        let mut conn = self.pool.acquire().await.map_err(map_sqlx)?;
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
        let mut store = SqliteStore::new(pool.clone());
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
}
