use chrono::{DateTime, Utc};
use uuid::Uuid;

use taskboard_core::{Comment, EntityType, RunStatus};

use crate::actor::Actor;
use crate::commands::{CommentAdd, ReplyContinueResult, RunContinue};
use crate::error::AppError;
use crate::store::Store;

use super::helpers::*;

pub(super) async fn comment_add_and_continue_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: CommentAdd,
    now: DateTime<Utc>,
) -> Result<ReplyContinueResult, AppError> {
    let task_display_id = cmd.task_display_id.clone();
    let comment = comment_add_inner(store, actor, cmd, now).await?;
    let task = require_live_task(store, &task_display_id).await?;
    let runs = store.list_runs(task.id).await?;
    let winning = winning_run(&runs).ok_or_else(|| AppError::Validation {
        field: "status".into(),
        message: "run must be waiting".into(),
    })?;
    if winning.status != RunStatus::Waiting {
        return Err(AppError::Validation {
            field: "status".into(),
            message: "run must be waiting".into(),
        });
    }
    let run = super::runs::run_continue_inner(
        store,
        actor,
        RunContinue {
            run_display_id: winning.display_id.clone(),
            message: None,
            reply: None,
            revision: None,
        },
        now,
    )
    .await?;
    Ok(ReplyContinueResult { comment, run })
}

pub(super) async fn comment_add_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: CommentAdd,
    now: DateTime<Utc>,
) -> Result<Comment, AppError> {
    let body = require_non_blank("text", &cmd.body)?;
    if body.chars().count() > 4000 {
        return Err(AppError::Validation {
            field: "text".into(),
            message: "must be at most 4000 characters".into(),
        });
    }
    let task = require_live_task(store, &cmd.task_display_id).await?;
    let comment = Comment {
        id: Uuid::now_v7(),
        task_id: task.id,
        actor_kind: actor.kind,
        actor_label: actor.label.clone(),
        body,
        created_at: now,
    };
    store.insert_comment(&comment).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Comment,
            operation: "comment.add",
            entity_id: comment.id,
            previous_revision: None,
            before_json: None,
            after_json: Some(json_value(&comment)?),
        },
    )
    .await?;
    Ok(comment)
}

pub(super) async fn comment_remove_latest_inner(
    store: &mut dyn Store,
    actor: &Actor,
    task_display_id: &str,
    now: DateTime<Utc>,
) -> Result<Comment, AppError> {
    let task = require_live_task(store, task_display_id).await?;
    let comments = store.list_comments(task.id).await?;
    let comment = comments
        .last()
        .cloned()
        .ok_or_else(|| AppError::Validation {
            field: "comment".into(),
            message: "thread is empty".into(),
        })?;
    store.delete_comment(comment.id).await?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Comment,
            operation: "comment.remove",
            entity_id: comment.id,
            previous_revision: None,
            before_json: Some(json_value(&comment)?),
            after_json: None,
        },
    )
    .await?;
    Ok(comment)
}
