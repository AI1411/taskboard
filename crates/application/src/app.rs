use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;

use taskboard_core::{
    next_unique_slug, slugify, trim_project_name, EntityType, FieldError, Project, SlugError,
    ValidationError,
};

use crate::actor::Actor;
use crate::commands::{ProjectAdd, ProjectUpdate};
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
    check_revision(&project, cmd.revision)?;
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
    check_revision(&project, revision)?;
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
    check_revision(&project, revision)?;
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
    check_revision(&project, revision)?;
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

fn check_revision(project: &Project, expected: Option<i64>) -> Result<(), AppError> {
    if let Some(expected) = expected {
        if expected != project.revision {
            return Err(AppError::RevisionConflict {
                current: serde_json::to_value(project).map_err(map_json)?,
            });
        }
    }
    Ok(())
}

struct ActivityWrite {
    operation: &'static str,
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
            entity_type: EntityType::Project,
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
