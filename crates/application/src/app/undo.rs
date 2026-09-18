use chrono::{DateTime, Utc};
use uuid::Uuid;

use taskboard_core::{Activity, Check, Comment, EntityType, Link, Project, Run, Task};

use crate::actor::Actor;
use crate::error::AppError;
use crate::store::{Store, UndoResult};

use super::helpers::*;

pub(super) async fn undo_inner(
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

pub(super) async fn apply_compensation(
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
    if activity.operation == "check.remove" {
        let check: Check = serde_json::from_value(
            activity
                .before_json
                .clone()
                .ok_or_else(|| AppError::Io("check.remove missing before_json".into()))?,
        )
        .map_err(map_json)?;
        store.insert_check(&check).await?;
        return json_value(&check);
    }
    if activity.operation == "comment.remove" {
        let comment: Comment = serde_json::from_value(
            activity
                .before_json
                .clone()
                .ok_or_else(|| AppError::Io("comment.remove missing before_json".into()))?,
        )
        .map_err(map_json)?;
        store.insert_comment(&comment).await?;
        return json_value(&comment);
    }
    restore_snapshot(store, activity, now).await
}

pub(super) async fn revision_just_undone(
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
        EntityType::Link | EntityType::Comment | EntityType::Check => Ok(None),
    }
}

pub(super) async fn compensate_create(
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
        EntityType::Comment => {
            let comment = store.get_comment(activity.entity_id).await?;
            store.delete_comment(activity.entity_id).await?;
            match comment {
                Some(comment) => json_value(&comment),
                None => Ok(serde_json::Value::Null),
            }
        }
        EntityType::Check => {
            let check = store.get_check(activity.entity_id).await?;
            store.delete_check(activity.entity_id).await?;
            match check {
                Some(check) => json_value(&check),
                None => Ok(serde_json::Value::Null),
            }
        }
    }
}

pub(super) async fn restore_snapshot(
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
        EntityType::Check => {
            let check: Check = serde_json::from_value(before).map_err(map_json)?;
            store.update_check(&check).await?;
            json_value(&check)
        }
        EntityType::Comment => {
            let comment: Comment = serde_json::from_value(before).map_err(map_json)?;
            if store.get_comment(comment.id).await?.is_none() {
                store.insert_comment(&comment).await?;
            }
            json_value(&comment)
        }
    }
}

pub(super) async fn compensate_project_reorder(
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
