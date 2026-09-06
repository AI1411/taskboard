use taskboard_application::{
    Actor, App, AppError, LinkAdd, ProjectAdd, ProjectUpdate, RunFail, RunFinish, RunStart,
    RunUpdate, RunWait, SyncDelta, TaskCreate, TaskUpdate, Trash, UndoResult,
};
use taskboard_core::{ActorKind, Column, LinkKind, Project, Run, TaskDetail, TaskSummary};
use uuid::Uuid;

use crate::dto::RunOp;
use crate::error::AppErrorDto;

pub fn desktop_actor() -> Actor {
    Actor {
        kind: ActorKind::Desktop,
        label: "local-ui".into(),
    }
}

fn actor() -> Actor {
    desktop_actor()
}

fn missing(field: &str) -> AppErrorDto {
    AppError::Validation {
        field: field.into(),
        message: format!("{field} is required"),
    }
    .into()
}

pub async fn project_add_inner(
    app: &App,
    name: String,
    repo_path: Option<String>,
    slug: Option<String>,
) -> Result<Project, AppErrorDto> {
    app.project_add(
        &actor(),
        ProjectAdd {
            name,
            repo_path,
            slug,
        },
    )
    .await
    .map_err(Into::into)
}

pub async fn project_list_inner(
    app: &App,
    include_archived: bool,
) -> Result<Vec<Project>, AppErrorDto> {
    app.project_list(include_archived).await.map_err(Into::into)
}

pub async fn project_update_inner(
    app: &App,
    slug: String,
    name: Option<String>,
    repo_path: Option<String>,
    new_slug: Option<String>,
    revision: Option<i64>,
) -> Result<Project, AppErrorDto> {
    app.project_update(
        &actor(),
        ProjectUpdate {
            slug,
            name,
            repo_path,
            new_slug,
            revision,
        },
    )
    .await
    .map_err(Into::into)
}

pub async fn project_reorder_inner(
    app: &App,
    slugs: Vec<String>,
) -> Result<Vec<Project>, AppErrorDto> {
    app.project_reorder(&actor(), &slugs)
        .await
        .map_err(Into::into)
}

pub async fn project_archive_inner(
    app: &App,
    slug: String,
    archived: bool,
    revision: Option<i64>,
) -> Result<Project, AppErrorDto> {
    app.project_archive(&actor(), &slug, archived, revision)
        .await
        .map_err(Into::into)
}

pub async fn project_delete_inner(
    app: &App,
    slug: String,
    revision: Option<i64>,
) -> Result<Project, AppErrorDto> {
    app.project_delete(&actor(), &slug, revision)
        .await
        .map_err(Into::into)
}

pub async fn project_restore_inner(app: &App, slug: String) -> Result<Project, AppErrorDto> {
    app.project_restore(&actor(), &slug)
        .await
        .map_err(Into::into)
}

pub async fn project_note_set_inner(
    app: &App,
    slug: String,
    markdown: String,
    revision: Option<i64>,
) -> Result<Project, AppErrorDto> {
    app.project_note_set(&actor(), &slug, markdown, revision)
        .await
        .map_err(Into::into)
}

pub async fn task_create_inner(
    app: &App,
    project_slug: String,
    title: String,
    column: Option<Column>,
    urgent: Option<bool>,
) -> Result<TaskDetail, AppErrorDto> {
    app.task_create(
        &actor(),
        TaskCreate {
            project_slug,
            title,
            column,
            urgent: urgent.unwrap_or(false),
        },
    )
    .await
    .map_err(Into::into)
}

pub async fn task_list_inner(
    app: &App,
    project_slug: String,
) -> Result<Vec<TaskSummary>, AppErrorDto> {
    app.task_list(&project_slug).await.map_err(Into::into)
}

pub async fn task_show_inner(app: &App, display_id: String) -> Result<TaskDetail, AppErrorDto> {
    app.task_show(&display_id).await.map_err(Into::into)
}

pub struct TaskPatchArgs {
    pub display_id: String,
    pub title: Option<String>,
    pub note_markdown: Option<String>,
    pub urgent: Option<bool>,
    pub column: Option<Column>,
    pub before_display_id: Option<Option<String>>,
    pub revision: Option<i64>,
}

pub async fn task_update_inner(app: &App, patch: TaskPatchArgs) -> Result<TaskDetail, AppErrorDto> {
    let actor = actor();
    let TaskPatchArgs {
        display_id,
        title,
        note_markdown,
        urgent,
        column,
        before_display_id,
        mut revision,
    } = patch;
    let mut task = None;
    if title.is_some() {
        let updated = app
            .task_update(
                &actor,
                TaskUpdate {
                    display_id: display_id.clone(),
                    title,
                    revision,
                },
            )
            .await?;
        revision = Some(updated.revision);
        task = Some(updated);
    }
    if let Some(markdown) = note_markdown {
        let updated = app
            .task_note_set(&actor, &display_id, markdown, revision)
            .await?;
        revision = Some(updated.revision);
        task = Some(updated);
    }
    if let Some(urgent) = urgent {
        let updated = app
            .task_urgent(&actor, &display_id, urgent, revision)
            .await?;
        revision = Some(updated.revision);
        task = Some(updated);
    }
    if let Some(column) = column {
        let updated = app.task_move(&actor, &display_id, column, revision).await?;
        revision = Some(updated.revision);
        task = Some(updated);
    }
    if let Some(before) = before_display_id {
        let updated = app
            .task_reorder(&actor, &display_id, before.as_deref(), revision)
            .await?;
        task = Some(updated);
    }
    match task {
        Some(task) => Ok(task),
        None => app.task_show(&display_id).await.map_err(Into::into),
    }
}

pub async fn task_move_inner(
    app: &App,
    display_id: String,
    column: Column,
    revision: Option<i64>,
) -> Result<TaskDetail, AppErrorDto> {
    app.task_move(&actor(), &display_id, column, revision)
        .await
        .map_err(Into::into)
}

pub async fn task_reorder_inner(
    app: &App,
    display_id: String,
    before_display_id: Option<String>,
    revision: Option<i64>,
) -> Result<TaskDetail, AppErrorDto> {
    app.task_reorder(
        &actor(),
        &display_id,
        before_display_id.as_deref(),
        revision,
    )
    .await
    .map_err(Into::into)
}

pub async fn task_urgent_inner(
    app: &App,
    display_id: String,
    urgent: bool,
    revision: Option<i64>,
) -> Result<TaskDetail, AppErrorDto> {
    app.task_urgent(&actor(), &display_id, urgent, revision)
        .await
        .map_err(Into::into)
}

pub async fn task_delete_inner(
    app: &App,
    display_id: String,
    revision: Option<i64>,
) -> Result<TaskDetail, AppErrorDto> {
    app.task_delete(&actor(), &display_id, revision)
        .await
        .map_err(Into::into)
}

pub async fn task_restore_inner(app: &App, display_id: String) -> Result<TaskDetail, AppErrorDto> {
    app.task_restore(&actor(), &display_id)
        .await
        .map_err(Into::into)
}

pub async fn task_note_set_inner(
    app: &App,
    display_id: String,
    markdown: String,
    revision: Option<i64>,
) -> Result<TaskDetail, AppErrorDto> {
    app.task_note_set(&actor(), &display_id, markdown, revision)
        .await
        .map_err(Into::into)
}

pub async fn link_add_inner(
    app: &App,
    display_id: String,
    kind: LinkKind,
    value: String,
    revision: Option<i64>,
) -> Result<TaskDetail, AppErrorDto> {
    app.link_add(
        &actor(),
        LinkAdd {
            task_display_id: display_id,
            kind,
            value,
            revision,
        },
    )
    .await
    .map_err(Into::into)
}

pub async fn link_remove_inner(
    app: &App,
    link_id: String,
    revision: Option<i64>,
) -> Result<TaskDetail, AppErrorDto> {
    let id = Uuid::parse_str(&link_id).map_err(|_| {
        AppErrorDto::from(AppError::Validation {
            field: "link_id".into(),
            message: "invalid uuid".into(),
        })
    })?;
    app.link_remove(&actor(), id, revision)
        .await
        .map_err(Into::into)
}

pub async fn run_start_inner(
    app: &App,
    display_id: String,
    agent: String,
    session_id: Option<String>,
) -> Result<Run, AppErrorDto> {
    app.run_start(
        &actor(),
        RunStart {
            task_display_id: display_id,
            agent,
            session_id,
        },
    )
    .await
    .map_err(Into::into)
}

pub async fn run_patch_inner(
    app: &App,
    run_display_id: String,
    op: RunOp,
    message: Option<String>,
    reason: Option<String>,
    summary: Option<String>,
    revision: Option<i64>,
) -> Result<Run, AppErrorDto> {
    let actor = actor();
    match op {
        RunOp::Update => app
            .run_update(
                &actor,
                RunUpdate {
                    run_display_id,
                    message,
                    revision,
                },
            )
            .await
            .map_err(Into::into),
        RunOp::Wait => {
            let reason = reason.ok_or_else(|| missing("reason"))?;
            app.run_wait(
                &actor,
                RunWait {
                    run_display_id,
                    reason,
                    revision,
                },
            )
            .await
            .map_err(Into::into)
        }
        RunOp::Fail => {
            let summary = summary.ok_or_else(|| missing("summary"))?;
            app.run_fail(
                &actor,
                RunFail {
                    run_display_id,
                    summary,
                    revision,
                },
            )
            .await
            .map_err(Into::into)
        }
        RunOp::Finish => {
            let summary = summary.ok_or_else(|| missing("summary"))?;
            app.run_finish(
                &actor,
                RunFinish {
                    run_display_id,
                    summary,
                    revision,
                },
            )
            .await
            .map_err(Into::into)
        }
    }
}

pub async fn trash_list_inner(app: &App) -> Result<Trash, AppErrorDto> {
    app.trash_list().await.map_err(Into::into)
}

pub async fn undo_inner(app: &App) -> Result<UndoResult, AppErrorDto> {
    app.undo(&actor()).await.map_err(Into::into)
}

pub async fn sync_inner(app: &App, after: i64) -> Result<SyncDelta, AppErrorDto> {
    app.sync(after).await.map_err(Into::into)
}
