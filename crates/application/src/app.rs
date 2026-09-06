use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;

use taskboard_core::{
    card_display_status, display_id, next_unique_slug, place_before, place_urgent,
    rewrite_positions, slugify, sort_column, trim_project_name, trim_title, Column, DisplayKind,
    EntityType, FieldError, OrderError, OrderKey, Project, RunStatusView, SlugError, Task,
    TaskDetail, TaskSummary, ValidationError,
};

use crate::actor::Actor;
use crate::commands::{ProjectAdd, ProjectUpdate, TaskCreate, TaskUpdate};
use crate::error::AppError;
use crate::store::{NewActivity, Store};

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

fn display_from_runs(
    runs: &[taskboard_core::Run],
) -> (taskboard_core::CardDisplayStatus, Option<String>) {
    let views: Vec<RunStatusView> = runs
        .iter()
        .map(|run| RunStatusView {
            status: run.status,
            started_at: run.started_at,
            display_id: run.display_id.clone(),
        })
        .collect();
    let display_status = card_display_status(&views);
    let run_message = runs
        .iter()
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
