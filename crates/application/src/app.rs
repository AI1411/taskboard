use std::path::Path;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;

use taskboard_core::{
    card_display_status, display_id, next_unique_slug, parse_agent, parse_path_link, parse_url,
    place_before, place_urgent, rewrite_positions, slugify, sort_column, trim_project_name,
    trim_title, Activity, Column, DisplayKind, EntityType, FieldError, Link, LinkKind, OrderError,
    OrderKey, Project, Run, RunStatus, RunStatusView, SlugError, Task, TaskDetail, TaskSummary,
    ValidationError,
};

use crate::actor::Actor;
use crate::commands::{
    LinkAdd, ProjectAdd, ProjectUpdate, RunFail, RunFinish, RunStart, RunUpdate, RunWait,
    TaskCreate, TaskUpdate,
};
use crate::error::AppError;
use crate::store::{NewActivity, Store, SyncDelta, Trash, UndoResult};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

pub struct App {
    store: Mutex<Box<dyn Store>>,
    clock: Box<dyn Clock>,
}

impl App {
    pub fn new(store: impl Store + 'static, clock: impl Clock + 'static) -> Self {
        Self {
            store: Mutex::new(Box::new(store)),
            clock: Box::new(clock),
        }
    }

    pub async fn project_add(&self, actor: &Actor, cmd: ProjectAdd) -> Result<Project, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = project_add_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn project_list(&self, include_archived: bool) -> Result<Vec<Project>, AppError> {
        let mut store = self.store.lock().await;
        store.list_projects(include_archived).await
    }

    pub async fn project_show(&self, slug: &str) -> Result<Project, AppError> {
        let mut store = self.store.lock().await;
        require_live_project(&mut **store, slug).await
    }

    pub async fn project_update(
        &self,
        actor: &Actor,
        cmd: ProjectUpdate,
    ) -> Result<Project, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = project_update_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn project_reorder(
        &self,
        actor: &Actor,
        slugs_in_order: &[String],
    ) -> Result<Vec<Project>, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = project_reorder_inner(store, actor, slugs_in_order, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn project_archive(
        &self,
        actor: &Actor,
        slug: &str,
        archived: bool,
        revision: Option<i64>,
    ) -> Result<Project, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = project_archive_inner(store, actor, slug, archived, revision, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn project_delete(
        &self,
        actor: &Actor,
        slug: &str,
        revision: Option<i64>,
    ) -> Result<Project, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = project_delete_inner(store, actor, slug, revision, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn project_restore(&self, actor: &Actor, slug: &str) -> Result<Project, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = project_restore_inner(store, actor, slug, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn project_note_set(
        &self,
        actor: &Actor,
        slug: &str,
        markdown: String,
        revision: Option<i64>,
    ) -> Result<Project, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = project_note_set_inner(store, actor, slug, markdown, revision, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn task_create(
        &self,
        actor: &Actor,
        cmd: TaskCreate,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = task_create_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn task_list(&self, project_slug: &str) -> Result<Vec<TaskSummary>, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let project = require_live_project(store, project_slug).await?;
        let tasks = store.list_tasks(project.id).await?;
        let mut summaries = Vec::with_capacity(tasks.len());
        for task in tasks {
            summaries.push(to_task_summary(store, task).await?);
        }
        Ok(summaries)
    }

    pub async fn task_show(&self, display_id: &str) -> Result<TaskDetail, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let task = require_live_task(store, display_id).await?;
        load_task_detail(store, task).await
    }

    pub async fn task_update(
        &self,
        actor: &Actor,
        cmd: TaskUpdate,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = task_update_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn task_move(
        &self,
        actor: &Actor,
        display_id: &str,
        column: Column,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = task_move_inner(store, actor, display_id, column, revision, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn task_reorder(
        &self,
        actor: &Actor,
        display_id: &str,
        before_display_id: Option<&str>,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result =
            task_reorder_inner(store, actor, display_id, before_display_id, revision, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn task_urgent(
        &self,
        actor: &Actor,
        display_id: &str,
        urgent: bool,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = task_urgent_inner(store, actor, display_id, urgent, revision, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn task_note_set(
        &self,
        actor: &Actor,
        display_id: &str,
        markdown: String,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = task_note_set_inner(store, actor, display_id, markdown, revision, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn task_note_add(
        &self,
        actor: &Actor,
        display_id: &str,
        paragraph: &str,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = task_note_add_inner(store, actor, display_id, paragraph, revision, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn link_add(&self, actor: &Actor, cmd: LinkAdd) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = link_add_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn link_remove(
        &self,
        actor: &Actor,
        link_id: Uuid,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = link_remove_inner(store, actor, link_id, revision, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn run_start(&self, actor: &Actor, cmd: RunStart) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = run_start_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn run_update(&self, actor: &Actor, cmd: RunUpdate) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = run_update_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn run_wait(&self, actor: &Actor, cmd: RunWait) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = run_wait_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn run_fail(&self, actor: &Actor, cmd: RunFail) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = run_fail_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn run_finish(&self, actor: &Actor, cmd: RunFinish) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = run_finish_inner(store, actor, cmd, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn task_delete(
        &self,
        actor: &Actor,
        display_id: &str,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = task_delete_inner(store, actor, display_id, revision, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn task_restore(
        &self,
        actor: &Actor,
        display_id: &str,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = task_restore_inner(store, actor, display_id, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn trash_list(&self) -> Result<Trash, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let projects = store.list_deleted_projects().await?;
        let deleted_tasks = store.list_deleted_tasks().await?;
        let mut tasks = Vec::with_capacity(deleted_tasks.len());
        for task in deleted_tasks {
            tasks.push(to_task_summary(store, task).await?);
        }
        Ok(Trash { projects, tasks })
    }

    pub async fn purge_expired_trash(&self, now: DateTime<Utc>) -> Result<u64, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = store.purge_expired(now, 30).await;
        commit_or_rollback(store, result).await
    }

    pub async fn undo(&self, actor: &Actor) -> Result<UndoResult, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = undo_inner(store, actor, now).await;
        commit_or_rollback(store, result).await
    }

    pub async fn activity_head(&self) -> Result<i64, AppError> {
        let mut store = self.store.lock().await;
        store.activity_head().await
    }

    pub async fn sync(&self, after: i64) -> Result<SyncDelta, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        sync_inner(store, after).await
    }

    pub async fn backup_export(&self, dest: &Path) -> Result<(), AppError> {
        let mut store = self.store.lock().await;
        store.backup_to(dest).await
    }

    pub async fn backup_import(&self, src: &Path) -> Result<(), AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.validate_import(src).await?;
        let pre = store.pre_import_path()?;
        store.backup_to(&pre).await?;
        store.close_pool().await?;
        let copied = store.replace_from(src).await;
        let reopened = store.reopen_pool().await;
        match (copied, reopened) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(err), Ok(())) => Err(err),
            (Ok(()), Err(err)) => Err(err),
            (Err(err), Err(_)) => Err(err),
        }
    }
}

async fn commit_or_rollback<T>(
    store: &mut dyn Store,
    result: Result<T, AppError>,
) -> Result<T, AppError> {
    match result {
        Ok(value) => match store.commit().await {
            Ok(()) => Ok(value),
            Err(err) => {
                let _ = store.rollback().await;
                Err(err)
            }
        },
        Err(err) => {
            let _ = store.rollback().await;
            Err(err)
        }
    }
}

async fn project_add_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: ProjectAdd,
    now: DateTime<Utc>,
) -> Result<Project, AppError> {
    let name = trim_project_name(&cmd.name).map_err(map_validation)?;
    let live = store.list_projects(true).await?;
    let slug = if let Some(explicit) = cmd.slug.as_deref() {
        let slug = slugify_field("slug", explicit)?;
        if live.iter().any(|project| project.slug == slug) {
            return Err(AppError::DuplicateSlug { slug });
        }
        slug
    } else {
        let live_slugs: Vec<&str> = live.iter().map(|project| project.slug.as_str()).collect();
        next_unique_slug(&name, &live_slugs).map_err(map_slug_error)?
    };
    let sort_order = live
        .iter()
        .map(|project| project.sort_order)
        .max()
        .map(|max| max + 1)
        .unwrap_or(0);
    let project = Project {
        id: Uuid::now_v7(),
        slug,
        name,
        repo_path: cmd.repo_path,
        archived: false,
        note_markdown: String::new(),
        sort_order,
        revision: 1,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    store.insert_project(&project).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Project,
            operation: "project.create",
            entity_id: project.id,
            previous_revision: None,
            before_json: None,
            after_json: Some(json_value(&project)?),
        },
    )
    .await?;
    Ok(project)
}

async fn project_update_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: ProjectUpdate,
    now: DateTime<Utc>,
) -> Result<Project, AppError> {
    let mut project = require_live_project(store, &cmd.slug).await?;
    check_revision(&project, project.revision, cmd.revision)?;
    let before = project.clone();
    if let Some(name) = cmd.name {
        project.name = trim_project_name(&name).map_err(map_validation)?;
    }
    if let Some(repo_path) = cmd.repo_path {
        project.repo_path = if repo_path.is_empty() {
            None
        } else {
            Some(repo_path)
        };
    }
    if let Some(new_slug) = cmd.new_slug {
        let new_slug = slugify_field("slug", &new_slug)?;
        if new_slug != project.slug {
            if store.get_project_by_slug(&new_slug, false).await?.is_some() {
                return Err(AppError::DuplicateSlug { slug: new_slug });
            }
            project.slug = new_slug;
        }
    }
    project.revision += 1;
    project.updated_at = now;
    store.update_project(&project).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Project,
            operation: "project.update",
            entity_id: project.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&project)?),
        },
    )
    .await?;
    Ok(project)
}

async fn project_reorder_inner(
    store: &mut dyn Store,
    actor: &Actor,
    slugs_in_order: &[String],
    now: DateTime<Utc>,
) -> Result<Vec<Project>, AppError> {
    let live = store.list_projects(true).await?;
    if slugs_in_order.is_empty() && live.is_empty() {
        return Ok(Vec::new());
    }
    let mut remaining: std::collections::HashMap<String, Project> = live
        .into_iter()
        .map(|project| (project.slug.clone(), project))
        .collect();
    let mut seen = std::collections::HashSet::new();
    let mut ordered = Vec::with_capacity(slugs_in_order.len());
    for slug in slugs_in_order {
        if !seen.insert(slug) {
            return Err(AppError::Validation {
                field: "slugs".into(),
                message: "duplicate slug in reorder list".into(),
            });
        }
        let project = remaining.remove(slug).ok_or_else(|| AppError::NotFound {
            entity: "project".into(),
            id: slug.clone(),
        })?;
        ordered.push(project);
    }
    if !remaining.is_empty() {
        return Err(AppError::Validation {
            field: "slugs".into(),
            message: "reorder list must cover all live projects".into(),
        });
    }
    let before = ordered.clone();
    let ids: Vec<Uuid> = ordered.iter().map(|project| project.id).collect();
    store.rewrite_project_sort_orders(&ids).await?;
    for (index, project) in ordered.iter_mut().enumerate() {
        project.sort_order = index as i64;
        project.revision += 1;
        project.updated_at = now;
        store.update_project(project).await?;
    }
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Project,
            operation: "project.reorder",
            entity_id: ordered[0].id,
            previous_revision: None,
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&ordered)?),
        },
    )
    .await?;
    Ok(ordered)
}

async fn project_archive_inner(
    store: &mut dyn Store,
    actor: &Actor,
    slug: &str,
    archived: bool,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<Project, AppError> {
    let mut project = require_live_project(store, slug).await?;
    check_revision(&project, project.revision, revision)?;
    let before = project.clone();
    project.archived = archived;
    project.revision += 1;
    project.updated_at = now;
    store.update_project(&project).await?;
    let operation = if archived {
        "project.archive"
    } else {
        "project.unarchive"
    };
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Project,
            operation,
            entity_id: project.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&project)?),
        },
    )
    .await?;
    Ok(project)
}

async fn project_delete_inner(
    store: &mut dyn Store,
    actor: &Actor,
    slug: &str,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<Project, AppError> {
    let mut project = require_live_project(store, slug).await?;
    check_revision(&project, project.revision, revision)?;
    let before = project.clone();
    project.deleted_at = Some(now);
    project.revision += 1;
    project.updated_at = now;
    store.update_project(&project).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Project,
            operation: "project.delete",
            entity_id: project.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&project)?),
        },
    )
    .await?;
    Ok(project)
}

async fn project_restore_inner(
    store: &mut dyn Store,
    actor: &Actor,
    slug: &str,
    now: DateTime<Utc>,
) -> Result<Project, AppError> {
    if store.get_project_by_slug(slug, false).await?.is_some() {
        let has_deleted = store
            .list_deleted_projects()
            .await?
            .iter()
            .any(|project| project.slug == slug);
        if has_deleted {
            return Err(AppError::DuplicateSlug {
                slug: slug.to_string(),
            });
        }
        return Err(project_not_found(slug));
    }
    let mut matches: Vec<Project> = store
        .list_deleted_projects()
        .await?
        .into_iter()
        .filter(|project| project.slug == slug)
        .collect();
    matches.sort_by_key(|project| project.deleted_at);
    let mut project = matches.pop().ok_or_else(|| project_not_found(slug))?;
    let before = project.clone();
    let live = store.list_projects(true).await?;
    project.sort_order = live
        .iter()
        .map(|live_project| live_project.sort_order)
        .max()
        .map(|max| max + 1)
        .unwrap_or(0);
    project.deleted_at = None;
    project.revision += 1;
    project.updated_at = now;
    store.restore_project(project.id).await?;
    store.update_project(&project).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Project,
            operation: "project.restore",
            entity_id: project.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&project)?),
        },
    )
    .await?;
    Ok(project)
}

async fn project_note_set_inner(
    store: &mut dyn Store,
    actor: &Actor,
    slug: &str,
    markdown: String,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<Project, AppError> {
    let mut project = require_live_project(store, slug).await?;
    check_revision(&project, project.revision, revision)?;
    let before = project.clone();
    project.note_markdown = markdown;
    project.revision += 1;
    project.updated_at = now;
    store.update_project(&project).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Project,
            operation: "project.note.set",
            entity_id: project.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&project)?),
        },
    )
    .await?;
    Ok(project)
}

async fn task_create_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: TaskCreate,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    let title = trim_title(&cmd.title).map_err(map_validation)?;
    let project = require_live_project(store, &cmd.project_slug).await?;
    let column = cmd.column.unwrap_or(Column::Todo);
    let n = store.next_display_n("task").await?;
    let display_id = display_id(DisplayKind::Task, n);
    let existing = store.list_tasks(project.id).await?;
    let mut column_tasks: Vec<Task> = existing
        .into_iter()
        .filter(|task| task.column == column)
        .collect();
    let mut keys = task_keys(&column_tasks);
    insert_into_urgency_group(&mut keys, display_id.clone(), cmd.urgent);
    let position = position_of(&keys, &display_id);
    let task = Task {
        id: Uuid::now_v7(),
        display_id: display_id.clone(),
        project_id: project.id,
        title,
        column,
        urgent: cmd.urgent,
        note_markdown: String::new(),
        position: column_tasks.len() as i64,
        revision: 1,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    store.insert_task(&task).await?;
    column_tasks.push(task.clone());
    rewrite_column(store, project.id, column, &keys, &column_tasks).await?;
    let mut task = task;
    task.position = position;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Task,
            operation: "task.create",
            entity_id: task.id,
            previous_revision: None,
            before_json: None,
            after_json: Some(json_value(&task)?),
        },
    )
    .await?;
    load_task_detail(store, task).await
}

async fn task_update_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: TaskUpdate,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    let mut task = require_live_task(store, &cmd.display_id).await?;
    check_revision(&task, task.revision, cmd.revision)?;
    let before = task.clone();
    if let Some(title) = cmd.title {
        task.title = trim_title(&title).map_err(map_validation)?;
    }
    task.revision += 1;
    task.updated_at = now;
    store.update_task(&task).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Task,
            operation: "task.update",
            entity_id: task.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&task)?),
        },
    )
    .await?;
    load_task_detail(store, task).await
}

async fn task_move_inner(
    store: &mut dyn Store,
    actor: &Actor,
    display_id: &str,
    dest: Column,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    let mut task = require_live_task(store, display_id).await?;
    check_revision(&task, task.revision, revision)?;
    let before = task.clone();
    let source = task.column;
    let all = store.list_tasks(task.project_id).await?;
    if source == dest {
        let column_tasks: Vec<Task> = all
            .into_iter()
            .filter(|item| item.column == source)
            .collect();
        let mut keys = task_keys(&column_tasks);
        keys.retain(|key| key.display_id != task.display_id);
        insert_into_urgency_group(&mut keys, task.display_id.clone(), task.urgent);
        rewrite_column(store, task.project_id, source, &keys, &column_tasks).await?;
        task.position = position_of(&keys, &task.display_id);
    } else {
        let source_tasks: Vec<Task> = all
            .iter()
            .filter(|item| item.column == source && item.id != task.id)
            .cloned()
            .collect();
        let mut dest_tasks: Vec<Task> = all
            .iter()
            .filter(|item| item.column == dest)
            .cloned()
            .collect();
        let mut source_keys = task_keys(&source_tasks);
        rewrite_positions(&mut source_keys);
        let mut dest_keys = task_keys(&dest_tasks);
        insert_into_urgency_group(&mut dest_keys, task.display_id.clone(), task.urgent);
        task.column = dest;
        task.position = dest_tasks.len() as i64;
        task.revision += 1;
        task.updated_at = now;
        store.update_task(&task).await?;
        dest_tasks.push(task.clone());
        rewrite_column(store, task.project_id, source, &source_keys, &source_tasks).await?;
        rewrite_column(store, task.project_id, dest, &dest_keys, &dest_tasks).await?;
        task.position = position_of(&dest_keys, &task.display_id);
    }
    if source == dest {
        task.revision += 1;
        task.updated_at = now;
        store.update_task(&task).await?;
    }
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Task,
            operation: "task.move",
            entity_id: task.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&task)?),
        },
    )
    .await?;
    load_task_detail(store, task).await
}

async fn task_reorder_inner(
    store: &mut dyn Store,
    actor: &Actor,
    display_id: &str,
    before_display_id: Option<&str>,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    let mut task = require_live_task(store, display_id).await?;
    check_revision(&task, task.revision, revision)?;
    let before = task.clone();
    if let Some(before_id) = before_display_id {
        let other = require_live_task(store, before_id).await?;
        if other.column != task.column {
            return Err(AppError::DifferentColumn {
                left: task.column.as_str().to_string(),
                right: other.column.as_str().to_string(),
            });
        }
    }
    let column_tasks: Vec<Task> = store
        .list_tasks(task.project_id)
        .await?
        .into_iter()
        .filter(|item| item.column == task.column)
        .collect();
    let mut keys = task_keys(&column_tasks);
    place_before(&mut keys, &task.display_id, before_display_id).map_err(map_order_error)?;
    sort_column(&mut keys);
    rewrite_positions(&mut keys);
    rewrite_column(store, task.project_id, task.column, &keys, &column_tasks).await?;
    task.position = position_of(&keys, &task.display_id);
    task.revision += 1;
    task.updated_at = now;
    store.update_task(&task).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Task,
            operation: "task.reorder",
            entity_id: task.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&task)?),
        },
    )
    .await?;
    load_task_detail(store, task).await
}

async fn task_urgent_inner(
    store: &mut dyn Store,
    actor: &Actor,
    display_id: &str,
    urgent: bool,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    let mut task = require_live_task(store, display_id).await?;
    check_revision(&task, task.revision, revision)?;
    let before = task.clone();
    let column_tasks: Vec<Task> = store
        .list_tasks(task.project_id)
        .await?
        .into_iter()
        .filter(|item| item.column == task.column)
        .collect();
    let mut keys = task_keys(&column_tasks);
    place_urgent(&mut keys, &task.display_id, urgent);
    rewrite_column(store, task.project_id, task.column, &keys, &column_tasks).await?;
    task.urgent = urgent;
    task.position = position_of(&keys, &task.display_id);
    task.revision += 1;
    task.updated_at = now;
    store.update_task(&task).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Task,
            operation: "task.urgent",
            entity_id: task.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&task)?),
        },
    )
    .await?;
    load_task_detail(store, task).await
}

async fn task_note_set_inner(
    store: &mut dyn Store,
    actor: &Actor,
    display_id: &str,
    markdown: String,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    write_task_note(store, actor, display_id, markdown, revision, now).await
}

async fn task_note_add_inner(
    store: &mut dyn Store,
    actor: &Actor,
    display_id: &str,
    paragraph: &str,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    let paragraph = require_non_blank("paragraph", paragraph)?;
    let task = require_live_task(store, display_id).await?;
    let markdown = if task.note_markdown.is_empty() {
        paragraph
    } else {
        format!("{}\n\n{paragraph}", task.note_markdown)
    };
    write_task_note(store, actor, display_id, markdown, revision, now).await
}

async fn write_task_note(
    store: &mut dyn Store,
    actor: &Actor,
    display_id: &str,
    markdown: String,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    let mut task = require_live_task(store, display_id).await?;
    check_revision(&task, task.revision, revision)?;
    let before = task.clone();
    task.note_markdown = markdown;
    task.revision += 1;
    task.updated_at = now;
    store.update_task(&task).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Task,
            operation: "task.note.set",
            entity_id: task.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&task)?),
        },
    )
    .await?;
    load_task_detail(store, task).await
}

async fn task_delete_inner(
    store: &mut dyn Store,
    actor: &Actor,
    display_id: &str,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    let mut task = require_live_task(store, display_id).await?;
    check_revision(&task, task.revision, revision)?;
    let before = task.clone();
    task.deleted_at = Some(now);
    task.revision += 1;
    task.updated_at = now;
    store.update_task(&task).await?;
    rewrite_live_column(store, before.project_id, before.column).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Task,
            operation: "task.delete",
            entity_id: task.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&task)?),
        },
    )
    .await?;
    load_task_detail(store, task).await
}

async fn task_restore_inner(
    store: &mut dyn Store,
    actor: &Actor,
    display_id: &str,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    let mut task = store
        .get_task_by_display_id(display_id, true)
        .await?
        .ok_or_else(|| task_not_found(display_id))?;
    if task.deleted_at.is_none() {
        return Err(task_not_found(display_id));
    }
    let before = task.clone();
    task.deleted_at = None;
    assign_unique_task_position(store, &mut task).await?;
    task.revision += 1;
    task.updated_at = now;
    store.update_task(&task).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Task,
            operation: "task.restore",
            entity_id: task.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&task)?),
        },
    )
    .await?;
    load_task_detail(store, task).await
}

async fn link_add_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: LinkAdd,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    let mut task = require_live_task(store, &cmd.task_display_id).await?;
    check_revision(&task, task.revision, cmd.revision)?;
    let value = match cmd.kind {
        LinkKind::Url => parse_url(&cmd.value).map_err(map_validation)?,
        LinkKind::Path => parse_path_link(&cmd.value).map_err(map_validation)?,
    };
    let existing = store.list_links(task.id).await?;
    let sort_order = existing
        .iter()
        .map(|link| link.sort_order)
        .max()
        .map(|max| max + 1)
        .unwrap_or(0);
    let link = Link {
        id: Uuid::now_v7(),
        task_id: task.id,
        kind: cmd.kind,
        value,
        sort_order,
    };
    store.insert_link(&link).await?;
    task.revision += 1;
    task.updated_at = now;
    store.update_task(&task).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Link,
            operation: "link.add",
            entity_id: link.id,
            previous_revision: None,
            before_json: None,
            after_json: Some(json_value(&link)?),
        },
    )
    .await?;
    load_task_detail(store, task).await
}

async fn link_remove_inner(
    store: &mut dyn Store,
    actor: &Actor,
    link_id: Uuid,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    let link = store
        .get_link(link_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "link".into(),
            id: link_id.to_string(),
        })?;
    let mut task = store
        .get_task(link.task_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "task".into(),
            id: link.task_id.to_string(),
        })?;
    if task.deleted_at.is_some() {
        return Err(AppError::NotFound {
            entity: "task".into(),
            id: task.display_id,
        });
    }
    check_revision(&task, task.revision, revision)?;
    store.delete_link(link.id).await?;
    task.revision += 1;
    task.updated_at = now;
    store.update_task(&task).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Link,
            operation: "link.remove",
            entity_id: link.id,
            previous_revision: None,
            before_json: Some(json_value(&link)?),
            after_json: None,
        },
    )
    .await?;
    load_task_detail(store, task).await
}

async fn run_start_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: RunStart,
    now: DateTime<Utc>,
) -> Result<Run, AppError> {
    let task = require_live_task(store, &cmd.task_display_id).await?;
    let agent = parse_agent(&cmd.agent).map_err(map_validation)?;
    let session_id = parse_session_id(cmd.session_id)?;
    let n = store.next_display_n("run").await?;
    let run = Run {
        id: Uuid::now_v7(),
        display_id: display_id(DisplayKind::Run, n),
        task_id: task.id,
        agent,
        session_id,
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
    store.insert_run(&run).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Run,
            operation: "run.start",
            entity_id: run.id,
            previous_revision: None,
            before_json: None,
            after_json: Some(json_value(&run)?),
        },
    )
    .await?;
    Ok(run)
}

async fn run_update_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: RunUpdate,
    now: DateTime<Utc>,
) -> Result<Run, AppError> {
    mutate_run(
        store,
        actor,
        &cmd.run_display_id,
        cmd.revision,
        now,
        "run.update",
        |run| {
            if let Some(message) = cmd.message {
                run.message = Some(parse_run_message(&message)?);
            }
            Ok(())
        },
    )
    .await
}

async fn run_wait_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: RunWait,
    now: DateTime<Utc>,
) -> Result<Run, AppError> {
    let reason = require_non_blank("reason", &cmd.reason)?;
    mutate_run(
        store,
        actor,
        &cmd.run_display_id,
        cmd.revision,
        now,
        "run.wait",
        |run| {
            run.status = RunStatus::Waiting;
            run.waiting_reason = Some(reason);
            Ok(())
        },
    )
    .await
}

async fn run_fail_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: RunFail,
    now: DateTime<Utc>,
) -> Result<Run, AppError> {
    let summary = require_non_blank("summary", &cmd.summary)?;
    mutate_run(
        store,
        actor,
        &cmd.run_display_id,
        cmd.revision,
        now,
        "run.fail",
        |run| {
            run.status = RunStatus::Failed;
            run.summary = Some(summary);
            run.ended_at = Some(now);
            Ok(())
        },
    )
    .await
}

async fn run_finish_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: RunFinish,
    now: DateTime<Utc>,
) -> Result<Run, AppError> {
    let summary = require_non_blank("summary", &cmd.summary)?;
    mutate_run(
        store,
        actor,
        &cmd.run_display_id,
        cmd.revision,
        now,
        "run.finish",
        |run| {
            run.status = RunStatus::Completed;
            run.summary = Some(summary);
            run.ended_at = Some(now);
            Ok(())
        },
    )
    .await
}

async fn mutate_run<F>(
    store: &mut dyn Store,
    actor: &Actor,
    display_id: &str,
    revision: Option<i64>,
    now: DateTime<Utc>,
    operation: &'static str,
    mutate: F,
) -> Result<Run, AppError>
where
    F: FnOnce(&mut Run) -> Result<(), AppError>,
{
    let mut run = require_run(store, display_id).await?;
    check_revision(&run, run.revision, revision)?;
    let before = run.clone();
    mutate(&mut run)?;
    run.revision += 1;
    run.updated_at = now;
    store.update_run(&run).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Run,
            operation,
            entity_id: run.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&run)?),
        },
    )
    .await?;
    Ok(run)
}

async fn require_run(store: &mut dyn Store, display_id: &str) -> Result<Run, AppError> {
    store
        .get_run_by_display_id(display_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "run".into(),
            id: display_id.to_string(),
        })
}

fn require_non_blank(field: &str, raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation {
            field: field.to_string(),
            message: "must not be empty".into(),
        });
    }
    Ok(trimmed.to_string())
}

fn parse_session_id(session_id: Option<String>) -> Result<Option<String>, AppError> {
    match session_id {
        None => Ok(None),
        Some(raw) => {
            if raw.chars().count() > 128 {
                return Err(AppError::Validation {
                    field: "session_id".into(),
                    message: "must be at most 128 characters".into(),
                });
            }
            Ok(Some(raw))
        }
    }
}

fn parse_run_message(raw: &str) -> Result<String, AppError> {
    if raw.chars().count() > 500 {
        return Err(AppError::Validation {
            field: "message".into(),
            message: "must be at most 500 characters".into(),
        });
    }
    Ok(raw.to_string())
}

async fn require_live_task(store: &mut dyn Store, display_id: &str) -> Result<Task, AppError> {
    store
        .get_task_by_display_id(display_id, false)
        .await?
        .ok_or_else(|| task_not_found(display_id))
}

fn task_not_found(display_id: &str) -> AppError {
    AppError::NotFound {
        entity: "task".into(),
        id: display_id.to_string(),
    }
}

fn task_keys(tasks: &[Task]) -> Vec<OrderKey> {
    tasks
        .iter()
        .map(|task| OrderKey {
            display_id: task.display_id.clone(),
            urgent: task.urgent,
            position: task.position,
        })
        .collect()
}

fn insert_into_urgency_group(keys: &mut Vec<OrderKey>, display_id: String, urgent: bool) {
    let insert_at = if urgent {
        keys.iter()
            .rposition(|key| key.urgent)
            .map(|index| index + 1)
            .unwrap_or(0)
    } else {
        keys.len()
    };
    keys.insert(
        insert_at,
        OrderKey {
            display_id,
            urgent,
            position: 0,
        },
    );
    rewrite_positions(keys);
}

fn position_of(keys: &[OrderKey], display_id: &str) -> i64 {
    keys.iter()
        .find(|key| key.display_id == display_id)
        .map(|key| key.position)
        .expect("display_id must exist in column keys")
}

fn ids_from_keys(keys: &[OrderKey], tasks: &[Task]) -> Vec<Uuid> {
    let by_display: std::collections::HashMap<&str, Uuid> = tasks
        .iter()
        .map(|task| (task.display_id.as_str(), task.id))
        .collect();
    keys.iter()
        .map(|key| by_display[key.display_id.as_str()])
        .collect()
}

async fn rewrite_column(
    store: &mut dyn Store,
    project_id: Uuid,
    column: Column,
    keys: &[OrderKey],
    tasks: &[Task],
) -> Result<(), AppError> {
    store
        .rewrite_task_positions(project_id, column, &ids_from_keys(keys, tasks))
        .await
}

async fn rewrite_live_column(
    store: &mut dyn Store,
    project_id: Uuid,
    column: Column,
) -> Result<(), AppError> {
    let column_tasks: Vec<Task> = store
        .list_tasks(project_id)
        .await?
        .into_iter()
        .filter(|item| item.column == column)
        .collect();
    let mut keys = task_keys(&column_tasks);
    rewrite_positions(&mut keys);
    rewrite_column(store, project_id, column, &keys, &column_tasks).await
}

async fn load_task_detail(store: &mut dyn Store, task: Task) -> Result<TaskDetail, AppError> {
    let links = store.list_links(task.id).await?;
    let runs = store.list_runs(task.id).await?;
    let recent_activities = store.list_recent_activities(task.id, 20).await?;
    let (display_status, run_message) = display_from_runs(&runs);
    Ok(TaskDetail {
        id: task.id,
        display_id: task.display_id,
        project_id: task.project_id,
        title: task.title,
        column: task.column,
        urgent: task.urgent,
        revision: task.revision,
        display_status,
        run_message,
        note_markdown: task.note_markdown,
        links,
        runs,
        recent_activities,
    })
}

async fn to_task_summary(store: &mut dyn Store, task: Task) -> Result<TaskSummary, AppError> {
    let runs = store.list_runs(task.id).await?;
    let (display_status, run_message) = display_from_runs(&runs);
    Ok(TaskSummary {
        id: task.id,
        display_id: task.display_id,
        project_id: task.project_id,
        title: task.title,
        column: task.column,
        urgent: task.urgent,
        revision: task.revision,
        display_status,
        run_message,
    })
}

fn display_from_runs(runs: &[Run]) -> (taskboard_core::CardDisplayStatus, Option<String>) {
    let views: Vec<RunStatusView> = runs
        .iter()
        .map(|run| RunStatusView {
            status: run.status,
            started_at: run.started_at,
            display_id: run.display_id.clone(),
        })
        .collect();
    let display_status = card_display_status(&views);
    let has_active = runs
        .iter()
        .any(|run| matches!(run.status, RunStatus::Running | RunStatus::Waiting));
    let run_message = runs
        .iter()
        .filter(|run| !has_active || matches!(run.status, RunStatus::Running | RunStatus::Waiting))
        .max_by(|left, right| {
            left.started_at
                .cmp(&right.started_at)
                .then_with(|| left.display_id.cmp(&right.display_id))
        })
        .and_then(|run| run.message.clone());
    (display_status, run_message)
}

fn map_order_error(err: OrderError) -> AppError {
    match err {
        OrderError::NotInColumn { display_id } => AppError::NotFound {
            entity: "task".into(),
            id: display_id,
        },
        OrderError::DifferentColumn { left, right } => AppError::DifferentColumn { left, right },
    }
}

async fn require_live_project(store: &mut dyn Store, slug: &str) -> Result<Project, AppError> {
    store
        .get_project_by_slug(slug, false)
        .await?
        .ok_or_else(|| project_not_found(slug))
}

fn project_not_found(slug: &str) -> AppError {
    AppError::NotFound {
        entity: "project".into(),
        id: slug.to_string(),
    }
}

fn check_revision<T: serde::Serialize>(
    entity: &T,
    current: i64,
    expected: Option<i64>,
) -> Result<(), AppError> {
    if let Some(expected) = expected {
        if expected != current {
            return Err(AppError::RevisionConflict {
                current: serde_json::to_value(entity).map_err(map_json)?,
            });
        }
    }
    Ok(())
}

struct ActivityWrite {
    operation: &'static str,
    entity_type: EntityType,
    entity_id: Uuid,
    previous_revision: Option<i64>,
    before_json: Option<serde_json::Value>,
    after_json: Option<serde_json::Value>,
}

async fn record_activity(
    store: &mut dyn Store,
    actor: &Actor,
    now: DateTime<Utc>,
    write: ActivityWrite,
) -> Result<(), AppError> {
    let sequence = store.next_activity_sequence().await?;
    store
        .insert_activity(NewActivity {
            id: Uuid::now_v7(),
            sequence,
            actor_kind: actor.kind,
            actor_label: actor.label.clone(),
            operation: write.operation.to_string(),
            entity_type: write.entity_type,
            entity_id: write.entity_id,
            previous_revision: write.previous_revision,
            before_json: write.before_json,
            after_json: write.after_json,
            created_at: now,
        })
        .await
}

async fn undo_inner(
    store: &mut dyn Store,
    actor: &Actor,
    now: DateTime<Utc>,
) -> Result<UndoResult, AppError> {
    let undoable = store
        .latest_undoable()
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "activity".into(),
            id: "undo".into(),
        })?;
    let latest = store.latest_activity_for(undoable.entity_id).await?;
    if latest
        .as_ref()
        .is_some_and(|activity| activity.sequence > undoable.sequence)
    {
        return Err(AppError::UndoConflict {
            current: current_entity_json(store, undoable.entity_type, undoable.entity_id).await?,
        });
    }
    let previous_revision = revision_just_undone(store, &undoable).await?;
    let entity = apply_compensation(store, &undoable, now).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: undoable.entity_type,
            operation: "undo",
            entity_id: undoable.entity_id,
            previous_revision,
            before_json: undoable.after_json.clone(),
            after_json: Some(entity.clone()),
        },
    )
    .await?;
    Ok(UndoResult {
        entity_type: undoable.entity_type,
        entity,
    })
}

async fn apply_compensation(
    store: &mut dyn Store,
    activity: &Activity,
    now: DateTime<Utc>,
) -> Result<serde_json::Value, AppError> {
    if activity.operation == "project.reorder" {
        return compensate_project_reorder(store, activity).await;
    }
    if activity.before_json.is_none() {
        return compensate_create(store, activity, now).await;
    }
    if activity.operation == "link.remove" {
        let link: Link = serde_json::from_value(
            activity
                .before_json
                .clone()
                .ok_or_else(|| AppError::Io("link.remove missing before_json".into()))?,
        )
        .map_err(map_json)?;
        store.insert_link(&link).await?;
        return json_value(&link);
    }
    restore_snapshot(store, activity, now).await
}

async fn revision_just_undone(
    store: &mut dyn Store,
    activity: &Activity,
) -> Result<Option<i64>, AppError> {
    match activity.entity_type {
        EntityType::Task => Ok(store
            .get_task(activity.entity_id)
            .await?
            .map(|task| task.revision)),
        EntityType::Project => Ok(store
            .get_project(activity.entity_id)
            .await?
            .map(|project| project.revision)),
        EntityType::Run => Ok(store
            .get_run(activity.entity_id)
            .await?
            .map(|run| run.revision)),
        EntityType::Link => Ok(None),
    }
}

async fn compensate_create(
    store: &mut dyn Store,
    activity: &Activity,
    now: DateTime<Utc>,
) -> Result<serde_json::Value, AppError> {
    match activity.entity_type {
        EntityType::Task => {
            let mut task =
                store
                    .get_task(activity.entity_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound {
                        entity: "task".into(),
                        id: activity.entity_id.to_string(),
                    })?;
            task.deleted_at = Some(now);
            task.revision += 1;
            task.updated_at = now;
            store.update_task(&task).await?;
            rewrite_live_column(store, task.project_id, task.column).await?;
            json_value(&task)
        }
        EntityType::Project => {
            let mut project = store
                .get_project(activity.entity_id)
                .await?
                .ok_or_else(|| AppError::NotFound {
                    entity: "project".into(),
                    id: activity.entity_id.to_string(),
                })?;
            project.deleted_at = Some(now);
            project.revision += 1;
            project.updated_at = now;
            store.update_project(&project).await?;
            json_value(&project)
        }
        EntityType::Link => {
            let link = store.get_link(activity.entity_id).await?;
            store.delete_link(activity.entity_id).await?;
            match link {
                Some(link) => json_value(&link),
                None => Ok(serde_json::Value::Null),
            }
        }
        EntityType::Run => {
            let run = store.get_run(activity.entity_id).await?;
            store.delete_run(activity.entity_id).await?;
            match run {
                Some(run) => json_value(&run),
                None => Ok(serde_json::Value::Null),
            }
        }
    }
}

async fn restore_snapshot(
    store: &mut dyn Store,
    activity: &Activity,
    now: DateTime<Utc>,
) -> Result<serde_json::Value, AppError> {
    let before = activity
        .before_json
        .clone()
        .ok_or_else(|| AppError::Io("activity missing before_json".into()))?;
    match activity.entity_type {
        EntityType::Task => {
            let current =
                store
                    .get_task(activity.entity_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound {
                        entity: "task".into(),
                        id: activity.entity_id.to_string(),
                    })?;
            let mut task: Task = serde_json::from_value(before).map_err(map_json)?;
            assign_unique_task_position(store, &mut task).await?;
            task.revision = current.revision + 1;
            task.updated_at = now;
            store.update_task(&task).await?;
            json_value(&task)
        }
        EntityType::Project => {
            let current = store
                .get_project(activity.entity_id)
                .await?
                .ok_or_else(|| AppError::NotFound {
                    entity: "project".into(),
                    id: activity.entity_id.to_string(),
                })?;
            let mut project: Project = serde_json::from_value(before).map_err(map_json)?;
            assign_unique_project_sort(store, &mut project).await?;
            project.revision = current.revision + 1;
            project.updated_at = now;
            store.update_project(&project).await?;
            json_value(&project)
        }
        EntityType::Link => {
            let link: Link = serde_json::from_value(before).map_err(map_json)?;
            store.update_link(&link).await?;
            json_value(&link)
        }
        EntityType::Run => {
            let current =
                store
                    .get_run(activity.entity_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound {
                        entity: "run".into(),
                        id: activity.entity_id.to_string(),
                    })?;
            let mut run: Run = serde_json::from_value(before).map_err(map_json)?;
            run.revision = current.revision + 1;
            run.updated_at = now;
            store.update_run(&run).await?;
            json_value(&run)
        }
    }
}

async fn compensate_project_reorder(
    store: &mut dyn Store,
    activity: &Activity,
) -> Result<serde_json::Value, AppError> {
    let before: Vec<Project> = serde_json::from_value(
        activity
            .before_json
            .clone()
            .ok_or_else(|| AppError::Io("project.reorder missing before_json".into()))?,
    )
    .map_err(map_json)?;
    let ids: Vec<Uuid> = before.iter().map(|project| project.id).collect();
    store.rewrite_project_sort_orders(&ids).await?;
    for project in &before {
        store.update_project(project).await?;
    }
    json_value(&before)
}

async fn assign_unique_task_position(
    store: &mut dyn Store,
    task: &mut Task,
) -> Result<(), AppError> {
    if task.deleted_at.is_some() {
        return Ok(());
    }
    let taken: std::collections::HashSet<i64> = store
        .list_tasks(task.project_id)
        .await?
        .into_iter()
        .filter(|item| item.column == task.column && item.id != task.id)
        .map(|item| item.position)
        .collect();
    if taken.contains(&task.position) {
        task.position = taken.iter().max().copied().unwrap_or(-1) + 1;
    }
    Ok(())
}

async fn assign_unique_project_sort(
    store: &mut dyn Store,
    project: &mut Project,
) -> Result<(), AppError> {
    if project.deleted_at.is_some() {
        return Ok(());
    }
    let taken: std::collections::HashSet<i64> = store
        .list_projects(true)
        .await?
        .into_iter()
        .filter(|item| item.id != project.id)
        .map(|item| item.sort_order)
        .collect();
    if taken.contains(&project.sort_order) {
        project.sort_order = taken.iter().max().copied().unwrap_or(-1) + 1;
    }
    Ok(())
}

async fn current_entity_json(
    store: &mut dyn Store,
    entity_type: EntityType,
    entity_id: Uuid,
) -> Result<serde_json::Value, AppError> {
    match entity_type {
        EntityType::Project => match store.get_project(entity_id).await? {
            Some(project) => json_value(&project),
            None => Ok(serde_json::Value::Null),
        },
        EntityType::Task => match store.get_task(entity_id).await? {
            Some(task) => json_value(&task),
            None => Ok(serde_json::Value::Null),
        },
        EntityType::Link => match store.get_link(entity_id).await? {
            Some(link) => json_value(&link),
            None => Ok(serde_json::Value::Null),
        },
        EntityType::Run => match store.get_run(entity_id).await? {
            Some(run) => json_value(&run),
            None => Ok(serde_json::Value::Null),
        },
    }
}

async fn sync_inner(store: &mut dyn Store, after: i64) -> Result<SyncDelta, AppError> {
    let activities = store.list_activities_after(after).await?;
    let sequence = store.activity_head().await?;
    let mut project_ids = Vec::new();
    let mut task_ids = Vec::new();
    let mut run_ids = Vec::new();
    let mut seen_projects = std::collections::HashSet::new();
    let mut seen_tasks = std::collections::HashSet::new();
    let mut seen_runs = std::collections::HashSet::new();

    for activity in activities {
        match activity.entity_type {
            EntityType::Project => {
                if seen_projects.insert(activity.entity_id) {
                    project_ids.push(activity.entity_id);
                }
            }
            EntityType::Task => {
                if seen_tasks.insert(activity.entity_id) {
                    task_ids.push(activity.entity_id);
                }
            }
            EntityType::Run => {
                if seen_runs.insert(activity.entity_id) {
                    run_ids.push(activity.entity_id);
                }
            }
            EntityType::Link => {
                if let Some(task_id) = link_task_id(store, &activity).await? {
                    if seen_tasks.insert(task_id) {
                        task_ids.push(task_id);
                    }
                }
            }
        }
    }

    let mut projects = Vec::new();
    for id in project_ids {
        if let Some(project) = store.get_project(id).await? {
            projects.push(project);
        }
    }
    let mut tasks = Vec::new();
    for id in task_ids {
        if let Some(task) = store.get_task(id).await? {
            tasks.push(load_task_detail(store, task).await?);
        }
    }
    let mut runs = Vec::new();
    for id in run_ids {
        if let Some(run) = store.get_run(id).await? {
            runs.push(run);
        }
    }
    Ok(SyncDelta {
        sequence,
        projects,
        tasks,
        runs,
    })
}

async fn link_task_id(
    store: &mut dyn Store,
    activity: &Activity,
) -> Result<Option<Uuid>, AppError> {
    if let Some(link) = store.get_link(activity.entity_id).await? {
        return Ok(Some(link.task_id));
    }
    for payload in [&activity.after_json, &activity.before_json]
        .into_iter()
        .flatten()
    {
        if let Some(task_id) = uuid_from_json(payload, "task_id") {
            return Ok(Some(task_id));
        }
    }
    Ok(None)
}

fn uuid_from_json(value: &serde_json::Value, key: &str) -> Option<Uuid> {
    value
        .get(key)
        .and_then(|item| item.as_str())
        .and_then(|raw| Uuid::parse_str(raw).ok())
}

fn json_value<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, AppError> {
    serde_json::to_value(value).map_err(map_json)
}

fn slugify_field(field: &str, raw: &str) -> Result<String, AppError> {
    slugify(raw).map_err(|_| AppError::Validation {
        field: field.to_string(),
        message: "must be a valid slug".into(),
    })
}

fn map_validation(err: ValidationError) -> AppError {
    AppError::Validation {
        field: err.field.to_string(),
        message: match err.source {
            FieldError::Empty => "must not be empty".into(),
            FieldError::TooLong { max } => format!("must be at most {max} characters"),
            FieldError::Invalid { allowed } => format!("must be {allowed}"),
        },
    }
}

fn map_slug_error(err: SlugError) -> AppError {
    match err {
        SlugError::Empty => AppError::Validation {
            field: "name".into(),
            message: "cannot derive a slug".into(),
        },
    }
}

fn map_json(err: serde_json::Error) -> AppError {
    AppError::Io(err.to_string())
}
