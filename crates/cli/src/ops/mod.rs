//! Shared CLI/MCP execution ops. Edges parse args and present results; this layer calls `App`.

use serde::Serialize;
use serde_json::Value;
use taskboard_application::{
    ActivityQuery, Actor, App, AppError, BoardStatus, CheckAdd, CommentAdd, InboxScope, LinkAdd,
    NextClaim, OccupancyGroup, OccupancyQuery, ReplyContinueResult, ReviewTask, RunCancel,
    RunContinue, RunFail, RunFinish, RunListQuery, RunStart, RunWait, TaskCreate, TaskListQuery,
    TaskSpawn, TaskUpdate,
};
use taskboard_core::{
    ActivityEntry, Check, Column, Comment, InboxItem, Project, Run, TaskDetail, TaskSummary,
};

pub fn entity_ok<T: Serialize>(entity: &T, revision: i64) -> Value {
    serde_json::json!({ "ok": true, "entity": entity, "revision": revision })
}

pub fn entities_ok<T: Serialize>(entities: &[T]) -> Value {
    serde_json::json!({ "ok": true, "entities": entities })
}

pub async fn detect_project(app: &App) -> Result<Project, AppError> {
    let cwd = std::env::current_dir().map_err(|err| AppError::Io(err.to_string()))?;
    let env_slug = std::env::var("TASKBOARD_PROJECT").ok();
    app.detect_project(&cwd, env_slug.as_deref().filter(|value| !value.is_empty()))
        .await
}

pub async fn resolve_project_slug(app: &App, project: Option<String>) -> Result<String, AppError> {
    if let Some(slug) = project {
        return Ok(slug);
    }
    Ok(detect_project(app).await?.slug)
}

pub async fn project_list(app: &App, include_archived: bool) -> Result<Vec<Project>, AppError> {
    app.project_list(include_archived).await
}

pub async fn task_list(app: &App, query: TaskListQuery) -> Result<Vec<TaskSummary>, AppError> {
    app.task_query(query).await
}

pub async fn task_show(app: &App, display_id: &str) -> Result<TaskDetail, AppError> {
    app.task_show(display_id).await
}

pub async fn task_create(
    app: &App,
    actor: &Actor,
    cmd: TaskCreate,
) -> Result<TaskDetail, AppError> {
    app.task_create(actor, cmd).await
}

pub async fn task_move(
    app: &App,
    actor: &Actor,
    display_id: &str,
    column: Column,
    revision: Option<i64>,
) -> Result<TaskDetail, AppError> {
    app.task_move(actor, display_id, column, revision).await
}

pub async fn task_update(
    app: &App,
    actor: &Actor,
    cmd: TaskUpdate,
) -> Result<TaskDetail, AppError> {
    app.task_update(actor, cmd).await
}

pub async fn task_spawn(
    app: &App,
    actor: &Actor,
    cmd: TaskSpawn,
) -> Result<Vec<TaskDetail>, AppError> {
    app.task_spawn(actor, cmd).await
}

pub async fn run_list(app: &App, query: RunListQuery) -> Result<Vec<Run>, AppError> {
    app.run_list(query).await
}

pub async fn run_show(app: &App, display_id: &str) -> Result<Run, AppError> {
    app.run_show(display_id).await
}

pub async fn run_current(app: &App, task_display_id: &str) -> Result<Run, AppError> {
    app.run_current(task_display_id).await
}

pub async fn run_start(
    app: &App,
    actor: &Actor,
    cmd: RunStart,
    exclusive: bool,
) -> Result<Run, AppError> {
    if exclusive {
        app.run_start_exclusive(actor, cmd).await
    } else {
        app.run_start(actor, cmd).await
    }
}

pub async fn run_continue(app: &App, actor: &Actor, cmd: RunContinue) -> Result<Run, AppError> {
    app.run_continue(actor, cmd).await
}

pub async fn run_wait(app: &App, actor: &Actor, cmd: RunWait) -> Result<Run, AppError> {
    app.run_wait(actor, cmd).await
}

pub async fn run_finish(app: &App, actor: &Actor, cmd: RunFinish) -> Result<Run, AppError> {
    app.run_finish(actor, cmd).await
}

pub async fn run_fail(app: &App, actor: &Actor, cmd: RunFail) -> Result<Run, AppError> {
    app.run_fail(actor, cmd).await
}

pub async fn run_cancel(app: &App, actor: &Actor, cmd: RunCancel) -> Result<Run, AppError> {
    app.run_cancel(actor, cmd).await
}

pub async fn next(app: &App, actor: &Actor, cmd: NextClaim) -> Result<Run, AppError> {
    app.next(actor, cmd).await
}

pub async fn inbox(app: &App, scope: InboxScope) -> Result<Vec<InboxItem>, AppError> {
    app.inbox(scope).await
}

pub async fn status(app: &App, project: Option<String>) -> Result<BoardStatus, AppError> {
    app.status(project).await
}

pub async fn occupancy(app: &App, query: OccupancyQuery) -> Result<Vec<OccupancyGroup>, AppError> {
    app.occupancy(query).await
}

pub async fn activity(app: &App, query: ActivityQuery) -> Result<Vec<ActivityEntry>, AppError> {
    app.activity_list(query).await
}

pub async fn stale(app: &App, minutes: i64) -> Result<Vec<Run>, AppError> {
    app.stale_list(minutes).await
}

pub async fn comment_add(app: &App, actor: &Actor, cmd: CommentAdd) -> Result<Comment, AppError> {
    app.comment_add(actor, cmd).await
}

pub async fn comment_add_and_continue(
    app: &App,
    actor: &Actor,
    cmd: CommentAdd,
) -> Result<ReplyContinueResult, AppError> {
    app.comment_add_and_continue(actor, cmd).await
}

pub async fn comment_list(app: &App, task_display_id: &str) -> Result<Vec<Comment>, AppError> {
    app.comment_list(task_display_id).await
}

pub async fn review(app: &App, actor: &Actor, cmd: ReviewTask) -> Result<TaskDetail, AppError> {
    app.review(actor, cmd).await
}

pub async fn check_add(app: &App, actor: &Actor, cmd: CheckAdd) -> Result<Check, AppError> {
    app.check_add(actor, cmd).await
}

pub async fn check_toggle(app: &App, actor: &Actor, display_id: &str) -> Result<Check, AppError> {
    app.check_toggle(actor, display_id).await
}

pub async fn check_list(app: &App, task_display_id: &str) -> Result<Vec<Check>, AppError> {
    app.check_list(task_display_id).await
}

pub async fn link_add(app: &App, actor: &Actor, cmd: LinkAdd) -> Result<TaskDetail, AppError> {
    app.link_add(actor, cmd).await
}
