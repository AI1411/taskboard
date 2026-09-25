use chrono::{DateTime, Utc};
use uuid::Uuid;

use taskboard_core::{
    display_id, parse_path_link, parse_url, place_before, place_urgent, rewrite_positions,
    sort_column, trim_title, CardDisplayStatus, Column, DisplayKind, EntityType, Link, LinkKind,
    Run, Task, TaskDetail, TaskSummary,
};

use crate::actor::Actor;
use crate::commands::{
    CommentAdd, LinkAdd, NextClaim, ReviewAction, ReviewTask, RunStart, TaskCreate, TaskListQuery,
    TaskSpawn, TaskUpdate,
};
use crate::error::AppError;
use crate::store::Store;

use super::helpers::*;

pub(super) async fn task_spawn_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: TaskSpawn,
    now: DateTime<Utc>,
) -> Result<Vec<TaskDetail>, AppError> {
    if cmd.titles.is_empty() {
        return Err(AppError::Validation {
            field: "title".into(),
            message: "must not be empty".into(),
        });
    }
    let parent = require_live_task(store, &cmd.parent_display_id).await?;
    let parent_before = parent.clone();
    let project =
        store
            .get_project(parent.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: "project".into(),
                id: parent.project_id.to_string(),
            })?;
    let mut children = Vec::new();
    for title in cmd.titles {
        let child = task_create_inner(
            store,
            actor,
            TaskCreate {
                project_slug: project.slug.clone(),
                title,
                column: None,
                urgent: false,
            },
            now,
        )
        .await?;
        link_add_inner(
            store,
            actor,
            LinkAdd {
                task_display_id: parent.display_id.clone(),
                kind: LinkKind::BlockedBy,
                value: child.display_id.clone(),
                revision: None,
            },
            now,
        )
        .await?;
        children.push(child);
    }
    let parent_after = store
        .get_task(parent.id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "task".into(),
            id: parent.display_id.clone(),
        })?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Task,
            operation: "task.spawn",
            entity_id: parent.id,
            previous_revision: Some(parent_before.revision),
            before_json: Some(json_value(&parent_before)?),
            after_json: Some(json_value(&parent_after)?),
        },
    )
    .await?;
    Ok(children)
}

pub(super) async fn task_create_inner(
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
        worktree_path: None,
        branch: None,
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

pub(super) async fn task_update_inner(
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
    if let Some(worktree_path) = cmd.worktree_path {
        task.worktree_path = optional_workspace_value("worktree", worktree_path)?;
    }
    if let Some(branch) = cmd.branch {
        task.branch = optional_workspace_value("branch", branch)?;
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

pub(super) async fn review_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: ReviewTask,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    let task = require_live_task(store, &cmd.task_display_id).await?;
    if task.column != Column::InReview {
        return Err(AppError::Validation {
            field: "column".into(),
            message: "task must be in-review".into(),
        });
    }
    super::comments::comment_add_inner(
        store,
        actor,
        CommentAdd {
            task_display_id: cmd.task_display_id.clone(),
            body: cmd.text,
        },
        now,
    )
    .await?;
    let dest = match cmd.action {
        ReviewAction::Approve => Column::Done,
        ReviewAction::Changes => Column::InProgress,
    };
    let before = task.clone();
    let moved =
        task_move_inner(store, actor, &cmd.task_display_id, dest, cmd.revision, now).await?;
    let after = store
        .get_task(moved.id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "task".into(),
            id: moved.display_id.clone(),
        })?;
    record_activity(
        store,
        actor,
        now,
        ActivityWrite {
            entity_type: EntityType::Task,
            operation: "task.review",
            entity_id: moved.id,
            previous_revision: Some(before.revision),
            before_json: Some(json_value(&before)?),
            after_json: Some(json_value(&after)?),
        },
    )
    .await?;
    load_task_detail(store, after).await
}

pub(super) async fn task_move_inner(
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

pub(super) async fn task_reorder_inner(
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

pub(super) async fn task_urgent_inner(
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

pub(super) async fn task_note_set_inner(
    store: &mut dyn Store,
    actor: &Actor,
    display_id: &str,
    markdown: String,
    revision: Option<i64>,
    now: DateTime<Utc>,
) -> Result<TaskDetail, AppError> {
    write_task_note(store, actor, display_id, markdown, revision, now).await
}

pub(super) async fn task_note_add_inner(
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

pub(super) async fn write_task_note(
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

pub(super) async fn task_delete_inner(
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

pub(super) async fn task_restore_inner(
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

pub(super) async fn link_add_inner(
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
        LinkKind::BlockedBy => parse_blocked_by(store, &task, &cmd.value).await?,
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

pub(super) async fn link_remove_inner(
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

pub(super) async fn task_query_inner(
    store: &mut dyn Store,
    query: TaskListQuery,
    now: DateTime<Utc>,
) -> Result<Vec<TaskSummary>, AppError> {
    let projects = if let Some(slug) = query.project.as_deref() {
        vec![require_live_project(store, slug).await?]
    } else {
        store.list_projects(false).await?
    };
    let index = load_block_index(store).await?;
    let runs_by_task = group_by_task(store.list_all_runs().await?, |run| run.task_id);
    let comments_by_task =
        group_by_task(store.list_all_comments().await?, |comment| comment.task_id);
    let checks_by_task = group_by_task(store.list_all_checks().await?, |check| check.task_id);
    let mut summaries = Vec::new();
    for project in projects {
        let tasks = store.list_tasks(project.id).await?;
        for task in tasks {
            let runs = rows_for(&runs_by_task, task.id);
            let comments = rows_for(&comments_by_task, task.id);
            let checks = rows_for(&checks_by_task, task.id);
            let (blocked_by, blocks) = block_lists(&task, &index);
            let summary =
                to_task_summary_from_runs(task, runs, comments, checks, blocked_by, blocks, now);
            if let Some(column) = query.column {
                if summary.column != column {
                    continue;
                }
            }
            if !query.statuses.is_empty() && !query.statuses.contains(&summary.display_status) {
                continue;
            }
            if let Some(agent) = query.agent.as_deref() {
                let Some(run) = winning_run(runs) else {
                    continue;
                };
                if run.agent != agent {
                    continue;
                }
            }
            if query.blocked && !has_active_blocker(&summary.blocked_by, &index) {
                continue;
            }
            if query.ready
                && (has_active_blocker(&summary.blocked_by, &index)
                    || !(summary.display_status == CardDisplayStatus::Idle
                        || summary.column == Column::Todo))
            {
                continue;
            }
            summaries.push(summary);
        }
    }
    Ok(summaries)
}

pub(super) async fn next_inner(
    store: &mut dyn Store,
    actor: &Actor,
    cmd: NextClaim,
    now: DateTime<Utc>,
) -> Result<Run, AppError> {
    let ready = task_query_inner(
        store,
        TaskListQuery {
            project: cmd.project,
            ready: true,
            ..TaskListQuery::default()
        },
        now,
    )
    .await?;
    let mut claimed = None;
    for task in ready {
        let runs = store.list_runs(task.id).await?;
        if !has_open_run(&runs) {
            claimed = Some(task);
            break;
        }
    }
    let task = claimed.ok_or_else(|| AppError::NotFound {
        entity: "task".into(),
        id: "ready".into(),
    })?;
    let run = super::runs::run_start_inner(
        store,
        actor,
        RunStart {
            task_display_id: task.display_id.clone(),
            agent: cmd.agent,
            session_id: cmd.session_id,
        },
        now,
        true,
    )
    .await?;
    if cmd.move_to_in_progress {
        task_move_inner(
            store,
            actor,
            &task.display_id,
            Column::InProgress,
            None,
            now,
        )
        .await?;
    }
    Ok(run)
}
