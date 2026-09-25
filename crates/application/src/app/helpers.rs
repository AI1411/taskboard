use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use taskboard_core::{
    card_display_status, rewrite_positions, slugify, winning_run_view, Activity, CardDisplayStatus,
    Check, Column, Comment, EntityType, FieldError, Link, LinkKind, OrderError, OrderKey, Project,
    Run, RunStatus, RunStatusView, SlugError, Task, TaskDetail, TaskSummary, ValidationError,
};

use crate::actor::Actor;
use crate::error::AppError;
use crate::store::{NewActivity, Store, SyncDelta};

pub(super) async fn commit_or_rollback<T>(
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

pub(super) fn has_open_run(runs: &[Run]) -> bool {
    runs.iter()
        .any(|run| matches!(run.status, RunStatus::Running | RunStatus::Waiting))
}

pub(super) fn require_non_blank(field: &str, raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation {
            field: field.to_string(),
            message: "must not be empty".into(),
        });
    }
    Ok(trimmed.to_string())
}

pub(super) fn parse_session_id(session_id: Option<String>) -> Result<Option<String>, AppError> {
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

pub(super) fn parse_run_message(raw: &str) -> Result<String, AppError> {
    if raw.chars().count() > 500 {
        return Err(AppError::Validation {
            field: "message".into(),
            message: "must be at most 500 characters".into(),
        });
    }
    Ok(raw.to_string())
}

pub(super) async fn require_live_task(
    store: &mut dyn Store,
    display_id: &str,
) -> Result<Task, AppError> {
    store
        .get_task_by_display_id(display_id, false)
        .await?
        .ok_or_else(|| task_not_found(display_id))
}

pub(super) fn task_not_found(display_id: &str) -> AppError {
    AppError::NotFound {
        entity: "task".into(),
        id: display_id.to_string(),
    }
}

pub(super) fn task_keys(tasks: &[Task]) -> Vec<OrderKey> {
    tasks
        .iter()
        .map(|task| OrderKey {
            display_id: task.display_id.clone(),
            urgent: task.urgent,
            position: task.position,
        })
        .collect()
}

pub(super) fn insert_into_urgency_group(
    keys: &mut Vec<OrderKey>,
    display_id: String,
    urgent: bool,
) {
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

pub(super) fn position_of(keys: &[OrderKey], display_id: &str) -> i64 {
    keys.iter()
        .find(|key| key.display_id == display_id)
        .map(|key| key.position)
        .expect("display_id must exist in column keys")
}

pub(super) fn ids_from_keys(keys: &[OrderKey], tasks: &[Task]) -> Vec<Uuid> {
    let by_display: std::collections::HashMap<&str, Uuid> = tasks
        .iter()
        .map(|task| (task.display_id.as_str(), task.id))
        .collect();
    keys.iter()
        .map(|key| by_display[key.display_id.as_str()])
        .collect()
}

pub(super) async fn rewrite_column(
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

pub(super) async fn rewrite_live_column(
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

pub(super) async fn load_task_detail(
    store: &mut dyn Store,
    task: Task,
) -> Result<TaskDetail, AppError> {
    let links = store.list_links(task.id).await?;
    let runs = store
        .list_runs(task.id)
        .await?
        .into_iter()
        .map(|run| with_task_workspace(run, &task))
        .collect::<Vec<_>>();
    let comments = store.list_comments(task.id).await?;
    let checks = store.list_checks(task.id).await?;
    let recent_activities = store.list_recent_task_activities(task.id, 20).await?;
    let (display_status, run_message, waiting_reason) = display_from_runs(&runs);
    let reply = reply_for(display_status, &comments);
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
        waiting_reason,
        reply,
        note_markdown: task.note_markdown,
        links,
        runs,
        comments,
        checks,
        worktree_path: task.worktree_path,
        branch: task.branch,
        recent_activities,
    })
}

pub(super) fn is_stale(run: &Run, now: DateTime<Utc>, minutes: i64) -> bool {
    run.status == RunStatus::Running && now - run.updated_at >= chrono::Duration::minutes(minutes)
}

pub(super) fn winning_run(runs: &[Run]) -> Option<&Run> {
    let views: Vec<RunStatusView> = runs
        .iter()
        .map(|run| RunStatusView {
            status: run.status,
            started_at: run.started_at,
            display_id: run.display_id.clone(),
        })
        .collect();
    let view = winning_run_view(&views)?;
    runs.iter().find(|run| run.display_id == view.display_id)
}

pub(super) fn to_task_summary_from_runs(
    task: Task,
    runs: &[Run],
    comments: &[Comment],
    checks: &[Check],
    blocked_by: Vec<String>,
    blocks: Vec<String>,
    now: DateTime<Utc>,
) -> TaskSummary {
    let (display_status, run_message, waiting_reason) = display_from_runs(runs);
    let stale = winning_run(runs)
        .map(|run| is_stale(run, now, 30))
        .unwrap_or(false);
    let checklist_total = checks.len() as i64;
    let checklist_done = checks.iter().filter(|check| check.done).count() as i64;
    TaskSummary {
        id: task.id,
        display_id: task.display_id,
        project_id: task.project_id,
        title: task.title,
        column: task.column,
        urgent: task.urgent,
        revision: task.revision,
        display_status,
        run_message,
        waiting_reason,
        reply: reply_for(display_status, comments),
        blocked_by,
        blocks,
        stale,
        checklist_done,
        checklist_total,
        worktree_path: task.worktree_path,
        branch: task.branch,
    }
}

pub(super) fn with_task_workspace(mut run: Run, task: &Task) -> Run {
    run.worktree_path = task.worktree_path.clone();
    run.branch = task.branch.clone();
    run
}

pub(super) fn optional_workspace_value(
    field: &str,
    value: Option<String>,
) -> Result<Option<String>, AppError> {
    match value {
        None => Ok(None),
        Some(raw) => Ok(Some(require_non_blank(field, &raw)?)),
    }
}

pub(super) fn reply_for(status: CardDisplayStatus, comments: &[Comment]) -> Option<String> {
    if status != CardDisplayStatus::Waiting {
        return None;
    }
    comments.last().map(|comment| comment.body.clone())
}

pub(super) struct BlockIndex {
    pub(super) links: Vec<Link>,
    pub(super) tasks_by_id: HashMap<Uuid, Task>,
    pub(super) display_by_id: HashMap<Uuid, String>,
}

pub(super) async fn load_block_index(store: &mut dyn Store) -> Result<BlockIndex, AppError> {
    let links = store.list_all_links().await?;
    let mut tasks_by_id = HashMap::new();
    let mut display_by_id = HashMap::new();
    for project in store.list_projects(false).await? {
        for task in store.list_tasks(project.id).await? {
            display_by_id.insert(task.id, task.display_id.clone());
            tasks_by_id.insert(task.id, task);
        }
    }
    Ok(BlockIndex {
        links,
        tasks_by_id,
        display_by_id,
    })
}

pub(super) fn block_lists(task: &Task, index: &BlockIndex) -> (Vec<String>, Vec<String>) {
    let blocked_by = index
        .links
        .iter()
        .filter(|link| link.task_id == task.id && link.kind == LinkKind::BlockedBy)
        .map(|link| link.value.clone())
        .collect();
    let blocks = index
        .links
        .iter()
        .filter(|link| link.kind == LinkKind::BlockedBy && link.value == task.display_id)
        .filter_map(|link| index.display_by_id.get(&link.task_id).cloned())
        .collect();
    (blocked_by, blocks)
}

pub(super) fn has_active_blocker(blocked_by: &[String], index: &BlockIndex) -> bool {
    blocked_by.iter().any(|display_id| {
        index.tasks_by_id.values().any(|task| {
            task.display_id == *display_id
                && task.deleted_at.is_none()
                && task.column != Column::Done
        })
    })
}

pub(super) fn would_cycle(
    edges: &HashMap<String, Vec<String>>,
    blocked: &str,
    blocker: &str,
) -> bool {
    let mut stack = vec![blocker.to_string()];
    let mut seen = HashSet::new();
    while let Some(current) = stack.pop() {
        if current == blocked {
            return true;
        }
        if !seen.insert(current.clone()) {
            continue;
        }
        if let Some(next) = edges.get(&current) {
            stack.extend(next.iter().cloned());
        }
    }
    false
}

pub(super) async fn parse_blocked_by(
    store: &mut dyn Store,
    task: &Task,
    value: &str,
) -> Result<String, AppError> {
    let blocker = require_live_task(store, value).await?;
    if blocker.display_id == task.display_id {
        return Err(AppError::Validation {
            field: "blocked_by".into(),
            message: "a task cannot block itself".into(),
        });
    }
    let index = load_block_index(store).await?;
    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    for link in &index.links {
        if link.kind != LinkKind::BlockedBy {
            continue;
        }
        if let Some(from) = index.display_by_id.get(&link.task_id) {
            edges
                .entry(from.clone())
                .or_default()
                .push(link.value.clone());
        }
    }
    if would_cycle(&edges, &task.display_id, &blocker.display_id) {
        return Err(AppError::Validation {
            field: "blocked_by".into(),
            message: "blocked-by cycle".into(),
        });
    }
    Ok(blocker.display_id)
}

pub(super) async fn to_task_summary(
    store: &mut dyn Store,
    task: Task,
    now: DateTime<Utc>,
) -> Result<TaskSummary, AppError> {
    let runs = store.list_runs(task.id).await?;
    let comments = store.list_comments(task.id).await?;
    let checks = store.list_checks(task.id).await?;
    let index = load_block_index(store).await?;
    let (blocked_by, blocks) = block_lists(&task, &index);
    Ok(to_task_summary_from_runs(
        task, &runs, &comments, &checks, blocked_by, blocks, now,
    ))
}

pub(super) fn group_by_task<T>(
    rows: Vec<T>,
    task_id: impl Fn(&T) -> Uuid,
) -> HashMap<Uuid, Vec<T>> {
    let mut grouped: HashMap<Uuid, Vec<T>> = HashMap::new();
    for row in rows {
        grouped.entry(task_id(&row)).or_default().push(row);
    }
    grouped
}

pub(super) fn rows_for<T>(grouped: &HashMap<Uuid, Vec<T>>, task_id: Uuid) -> &[T] {
    grouped.get(&task_id).map(Vec::as_slice).unwrap_or(&[])
}

pub(super) fn display_from_runs(
    runs: &[Run],
) -> (
    taskboard_core::CardDisplayStatus,
    Option<String>,
    Option<String>,
) {
    let views: Vec<RunStatusView> = runs
        .iter()
        .map(|run| RunStatusView {
            status: run.status,
            started_at: run.started_at,
            display_id: run.display_id.clone(),
        })
        .collect();
    let display_status = card_display_status(&views);
    let winning = winning_run(runs);
    let run_message = winning.and_then(|run| run.message.clone());
    let waiting_reason = winning.and_then(|run| run.waiting_reason.clone());
    (display_status, run_message, waiting_reason)
}

pub(super) fn map_order_error(err: OrderError) -> AppError {
    match err {
        OrderError::NotInColumn { display_id } => AppError::NotFound {
            entity: "task".into(),
            id: display_id,
        },
        OrderError::DifferentColumn { left, right } => AppError::DifferentColumn { left, right },
    }
}

pub(super) async fn require_live_project(
    store: &mut dyn Store,
    slug: &str,
) -> Result<Project, AppError> {
    store
        .get_project_by_slug(slug, false)
        .await?
        .ok_or_else(|| project_not_found(slug))
}

pub(super) fn project_not_found(slug: &str) -> AppError {
    AppError::NotFound {
        entity: "project".into(),
        id: slug.to_string(),
    }
}

pub(super) fn check_revision<T: serde::Serialize>(
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

pub(super) struct ActivityWrite {
    pub(super) operation: &'static str,
    pub(super) entity_type: EntityType,
    pub(super) entity_id: Uuid,
    pub(super) previous_revision: Option<i64>,
    pub(super) before_json: Option<serde_json::Value>,
    pub(super) after_json: Option<serde_json::Value>,
}

pub(super) async fn record_activity(
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

pub(super) async fn assign_unique_task_position(
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

pub(super) async fn assign_unique_project_sort(
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

pub(super) struct ResolvedActivity {
    pub(super) target: String,
    pub(super) project_slug: Option<String>,
    pub(super) task_display_id: Option<String>,
}

pub(super) async fn resolve_activity_target(
    store: &mut dyn Store,
    activity: &Activity,
) -> Result<Option<ResolvedActivity>, AppError> {
    match activity.entity_type {
        EntityType::Project => {
            let Some(project) = store.get_project(activity.entity_id).await? else {
                return Ok(None);
            };
            Ok(Some(ResolvedActivity {
                target: project.slug.clone(),
                project_slug: Some(project.slug),
                task_display_id: None,
            }))
        }
        EntityType::Task => {
            let Some(task) = store.get_task(activity.entity_id).await? else {
                return Ok(None);
            };
            let project_slug = store
                .get_project(task.project_id)
                .await?
                .map(|project| project.slug);
            Ok(Some(ResolvedActivity {
                target: task.display_id.clone(),
                project_slug,
                task_display_id: Some(task.display_id),
            }))
        }
        EntityType::Run => {
            let Some(run) = store.get_run(activity.entity_id).await? else {
                return Ok(None);
            };
            let Some(task) = store.get_task(run.task_id).await? else {
                return Ok(Some(ResolvedActivity {
                    target: run.display_id,
                    project_slug: None,
                    task_display_id: None,
                }));
            };
            let project_slug = store
                .get_project(task.project_id)
                .await?
                .map(|project| project.slug);
            Ok(Some(ResolvedActivity {
                target: run.display_id,
                project_slug,
                task_display_id: Some(task.display_id),
            }))
        }
        EntityType::Link | EntityType::Comment | EntityType::Check => {
            let Some(task_id) = child_task_id(store, activity).await? else {
                return Ok(None);
            };
            let Some(task) = store.get_task(task_id).await? else {
                return Ok(Some(ResolvedActivity {
                    target: activity.entity_id.to_string(),
                    project_slug: None,
                    task_display_id: None,
                }));
            };
            let project_slug = store
                .get_project(task.project_id)
                .await?
                .map(|project| project.slug);
            Ok(Some(ResolvedActivity {
                target: task.display_id.clone(),
                project_slug,
                task_display_id: Some(task.display_id),
            }))
        }
    }
}

pub(super) async fn current_entity_json(
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
        EntityType::Comment => match store.get_comment(entity_id).await? {
            Some(comment) => json_value(&comment),
            None => Ok(serde_json::Value::Null),
        },
        EntityType::Check => match store.get_check(entity_id).await? {
            Some(check) => json_value(&check),
            None => Ok(serde_json::Value::Null),
        },
    }
}

pub(super) async fn sync_inner(store: &mut dyn Store, after: i64) -> Result<SyncDelta, AppError> {
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
            EntityType::Link | EntityType::Comment | EntityType::Check => {
                if let Some(task_id) = child_task_id(store, &activity).await? {
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

pub(super) async fn child_task_id(
    store: &mut dyn Store,
    activity: &Activity,
) -> Result<Option<Uuid>, AppError> {
    if let Some(link) = store.get_link(activity.entity_id).await? {
        return Ok(Some(link.task_id));
    }
    if let Some(comment) = store.get_comment(activity.entity_id).await? {
        return Ok(Some(comment.task_id));
    }
    if let Some(check) = store.get_check(activity.entity_id).await? {
        return Ok(Some(check.task_id));
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

pub(super) fn uuid_from_json(value: &serde_json::Value, key: &str) -> Option<Uuid> {
    value
        .get(key)
        .and_then(|item| item.as_str())
        .and_then(|raw| Uuid::parse_str(raw).ok())
}

pub(super) fn json_value<T: serde::Serialize>(value: &T) -> Result<serde_json::Value, AppError> {
    serde_json::to_value(value).map_err(map_json)
}

pub(super) fn slugify_field(field: &str, raw: &str) -> Result<String, AppError> {
    slugify(raw).map_err(|_| AppError::Validation {
        field: field.to_string(),
        message: "must be a valid slug".into(),
    })
}

pub(super) fn map_validation(err: ValidationError) -> AppError {
    AppError::Validation {
        field: err.field.to_string(),
        message: match err.source {
            FieldError::Empty => "must not be empty".into(),
            FieldError::TooLong { max } => format!("must be at most {max} characters"),
            FieldError::Invalid { allowed } => format!("must be {allowed}"),
        },
    }
}

pub(super) fn map_slug_error(err: SlugError) -> AppError {
    match err {
        SlugError::Empty => AppError::Validation {
            field: "name".into(),
            message: "cannot derive a slug".into(),
        },
    }
}

pub(super) fn map_json(err: serde_json::Error) -> AppError {
    AppError::Io(err.to_string())
}
