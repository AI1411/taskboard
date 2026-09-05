use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use taskboard_core::{
    Activity, ActorKind, Column, EntityType, Link, Project, Run, Task, TaskDetail, TaskSummary,
};
use uuid::Uuid;

use crate::error::AppError;

/// Row used to insert into `activities`.
#[derive(Debug, Clone, PartialEq)]
pub struct NewActivity {
    pub id: Uuid,
    pub sequence: i64,
    pub actor_kind: ActorKind,
    pub actor_label: String,
    pub operation: String,
    pub entity_type: EntityType,
    pub entity_id: Uuid,
    pub previous_revision: Option<i64>,
    pub before_json: Option<serde_json::Value>,
    pub after_json: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trash {
    pub projects: Vec<Project>,
    pub tasks: Vec<TaskSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncDelta {
    pub sequence: i64,
    pub projects: Vec<Project>,
    pub tasks: Vec<TaskDetail>,
    pub runs: Vec<Run>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UndoResult {
    pub entity_type: EntityType,
    pub entity: serde_json::Value,
}

/// Persistence surface implemented by `taskboard-store-sqlite`.
///
/// Get/list/insert/update (and soft-delete/restore where the schema supports it)
/// cover projects, tasks, links, and runs so later use cases can land without
/// rewriting this trait. Links and runs have no `deleted_at`; they are hard-deleted.
#[async_trait]
pub trait Store: Send + Sync {
    async fn next_display_n(&mut self, counter: &str) -> Result<i64, AppError>;
    async fn next_activity_sequence(&mut self) -> Result<i64, AppError>;
    async fn insert_activity(&mut self, row: NewActivity) -> Result<(), AppError>;

    async fn get_project(&mut self, id: Uuid) -> Result<Option<Project>, AppError>;
    async fn get_project_by_slug(
        &mut self,
        slug: &str,
        include_deleted: bool,
    ) -> Result<Option<Project>, AppError>;
    async fn list_projects(&mut self, include_archived: bool) -> Result<Vec<Project>, AppError>;
    async fn list_deleted_projects(&mut self) -> Result<Vec<Project>, AppError>;
    async fn insert_project(&mut self, project: &Project) -> Result<(), AppError>;
    async fn update_project(&mut self, project: &Project) -> Result<(), AppError>;
    async fn soft_delete_project(
        &mut self,
        id: Uuid,
        deleted_at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn restore_project(&mut self, id: Uuid) -> Result<(), AppError>;
    async fn rewrite_project_sort_orders(&mut self, ordered_ids: &[Uuid]) -> Result<(), AppError>;

    async fn get_task(&mut self, id: Uuid) -> Result<Option<Task>, AppError>;
    async fn get_task_by_display_id(
        &mut self,
        display_id: &str,
        include_deleted: bool,
    ) -> Result<Option<Task>, AppError>;
    async fn list_tasks(&mut self, project_id: Uuid) -> Result<Vec<Task>, AppError>;
    async fn list_deleted_tasks(&mut self) -> Result<Vec<Task>, AppError>;
    async fn insert_task(&mut self, task: &Task) -> Result<(), AppError>;
    async fn update_task(&mut self, task: &Task) -> Result<(), AppError>;
    async fn soft_delete_task(
        &mut self,
        id: Uuid,
        deleted_at: DateTime<Utc>,
    ) -> Result<(), AppError>;
    async fn restore_task(&mut self, id: Uuid) -> Result<(), AppError>;

    /// Rewrite live task positions in `column` to `0..n-1` following `ordered_ids`.
    ///
    /// The SQLite implementation must use two phases to satisfy the unique
    /// live-position index: first set `position = -(old_position + 1)` for each
    /// affected row, then assign the final `0..n-1` values.
    async fn rewrite_task_positions(
        &mut self,
        project_id: Uuid,
        column: Column,
        ordered_ids: &[Uuid],
    ) -> Result<(), AppError>;

    async fn get_link(&mut self, id: Uuid) -> Result<Option<Link>, AppError>;
    async fn list_links(&mut self, task_id: Uuid) -> Result<Vec<Link>, AppError>;
    async fn insert_link(&mut self, link: &Link) -> Result<(), AppError>;
    async fn update_link(&mut self, link: &Link) -> Result<(), AppError>;
    async fn delete_link(&mut self, id: Uuid) -> Result<(), AppError>;

    async fn get_run(&mut self, id: Uuid) -> Result<Option<Run>, AppError>;
    async fn get_run_by_display_id(&mut self, display_id: &str) -> Result<Option<Run>, AppError>;
    async fn list_runs(&mut self, task_id: Uuid) -> Result<Vec<Run>, AppError>;
    async fn insert_run(&mut self, run: &Run) -> Result<(), AppError>;
    async fn update_run(&mut self, run: &Run) -> Result<(), AppError>;
    async fn delete_run(&mut self, id: Uuid) -> Result<(), AppError>;

    async fn activity_head(&mut self) -> Result<i64, AppError>;
    async fn latest_activity_for(&mut self, entity_id: Uuid) -> Result<Option<Activity>, AppError>;
    async fn latest_undoable(&mut self) -> Result<Option<Activity>, AppError>;
    async fn list_activities_after(&mut self, sequence: i64) -> Result<Vec<Activity>, AppError>;
    async fn list_recent_activities(
        &mut self,
        entity_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Activity>, AppError>;

    async fn purge_expired(
        &mut self,
        now: DateTime<Utc>,
        retention_days: i64,
    ) -> Result<u64, AppError>;
    async fn backup_to(&mut self, dest: &Path) -> Result<(), AppError>;
    async fn replace_from(&mut self, src: &Path) -> Result<(), AppError>;
    async fn begin(&mut self) -> Result<(), AppError>;
    async fn commit(&mut self) -> Result<(), AppError>;
    async fn rollback(&mut self) -> Result<(), AppError>;
}
