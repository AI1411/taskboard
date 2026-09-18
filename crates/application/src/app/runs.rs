use chrono::{DateTime, Utc};
use uuid::Uuid;

use taskboard_core::{display_id, parse_agent, DisplayKind, EntityType, Run, RunStatus};

use crate::actor::Actor;
use crate::commands::{
    CommentAdd, RunCancel, RunContinue, RunFail, RunFinish, RunStart, RunUpdate, RunWait,
};
use crate::error::AppError;
use crate::store::Store;

use super::helpers::*;

pub(super) async fn run_start_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: RunStart,
    now: DateTime<Utc>,
    exclusive: bool,
) -> Result<Run, AppError> {
    let task = require_live_task(store, &cmd.task_display_id).await?;
    if exclusive {
        let runs = store.list_runs(task.id).await?;
        if has_open_run(&runs) {
            return Err(AppError::Conflict {
                entity: "task".into(),
                id: task.display_id,
            });
        }
    }
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
        worktree_path: task.worktree_path.clone(),
        branch: task.branch.clone(),
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

pub(super) async fn run_update_inner(
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

pub(super) async fn run_wait_inner(
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

pub(super) async fn run_continue_with_reply_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: RunContinue,
    now: DateTime<Utc>,
) -> Result<Run, AppError> {
    if let Some(reply) = cmd.reply.clone() {
        let run = require_run(store, &cmd.run_display_id).await?;
        let task = store
            .get_task(run.task_id)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: "task".into(),
                id: run.task_id.to_string(),
            })?;
        super::comments::comment_add_inner(
            store,
            actor,
            CommentAdd {
                task_display_id: task.display_id,
                body: reply,
            },
            now,
        )
        .await?;
    }
    run_continue_inner(store, actor, RunContinue { reply: None, ..cmd }, now).await
}

pub(super) async fn run_continue_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: RunContinue,
    now: DateTime<Utc>,
) -> Result<Run, AppError> {
    mutate_run(
        store,
        actor,
        &cmd.run_display_id,
        cmd.revision,
        now,
        "run.continue",
        |run| {
            if run.status != RunStatus::Waiting {
                return Err(AppError::Validation {
                    field: "status".into(),
                    message: "run must be waiting".into(),
                });
            }
            if let Some(message) = cmd.message {
                run.message = Some(parse_run_message(&message)?);
            }
            run.status = RunStatus::Running;
            run.waiting_reason = None;
            Ok(())
        },
    )
    .await
}

pub(super) async fn run_cancel_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: RunCancel,
    now: DateTime<Utc>,
) -> Result<Run, AppError> {
    let run = require_run(store, &cmd.run_display_id).await?;
    if !matches!(run.status, RunStatus::Running | RunStatus::Waiting) {
        return Err(AppError::Validation {
            field: "status".into(),
            message: "run must be running or waiting".into(),
        });
    }
    let summary = match cmd.summary {
        Some(raw) => require_non_blank("summary", &raw)?,
        None => "canceled".into(),
    };
    run_fail_inner(
        store,
        actor,
        RunFail {
            run_display_id: cmd.run_display_id,
            summary,
            revision: cmd.revision,
        },
        now,
    )
    .await
}

pub(super) async fn run_fail_inner(
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

pub(super) async fn run_finish_inner(
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

pub(super) async fn mutate_run<F>(
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

pub(super) async fn require_run(store: &mut dyn Store, display_id: &str) -> Result<Run, AppError> {
    store
        .get_run_by_display_id(display_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "run".into(),
            id: display_id.to_string(),
        })
}
