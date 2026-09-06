use taskboard_core::{Column, LinkKind};
use taskboard_desktop_commands::{
    deserialize_present_option, link_add_inner, link_remove_inner, project_add_inner,
    project_archive_inner, project_delete_inner, project_list_inner, project_note_set_inner,
    project_reorder_inner, project_restore_inner, project_update_inner, run_patch_inner,
    run_start_inner, sync_inner, task_create_inner, task_delete_inner, task_list_inner,
    task_move_inner, task_note_set_inner, task_reorder_inner, task_restore_inner, task_show_inner,
    task_update_inner, task_urgent_inner, trash_list_inner, undo_inner, AppErrorDto, ProjectDto,
    RunDto, RunOp, SyncDeltaDto, TaskDetailDto, TaskPatchArgs, TaskSummaryDto, TrashDto,
    UndoResultDto,
};

use crate::state::DesktopState;

// IPC keys match TauriTransport snake_case.
#[tauri::command(rename_all = "snake_case")]
pub async fn project_add(
    state: tauri::State<'_, DesktopState>,
    name: String,
    repo_path: Option<String>,
    slug: Option<String>,
) -> Result<ProjectDto, AppErrorDto> {
    let app = state.app.lock().await;
    project_add_inner(&app, name, repo_path, slug)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn project_list(
    state: tauri::State<'_, DesktopState>,
    include_archived: bool,
) -> Result<Vec<ProjectDto>, AppErrorDto> {
    let app = state.app.lock().await;
    project_list_inner(&app, include_archived)
        .await
        .map(|items| items.into_iter().map(Into::into).collect())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn project_update(
    state: tauri::State<'_, DesktopState>,
    slug: String,
    name: Option<String>,
    repo_path: Option<String>,
    new_slug: Option<String>,
    revision: Option<i64>,
) -> Result<ProjectDto, AppErrorDto> {
    let app = state.app.lock().await;
    project_update_inner(&app, slug, name, repo_path, new_slug, revision)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn project_reorder(
    state: tauri::State<'_, DesktopState>,
    slugs: Vec<String>,
) -> Result<Vec<ProjectDto>, AppErrorDto> {
    let app = state.app.lock().await;
    project_reorder_inner(&app, slugs)
        .await
        .map(|items| items.into_iter().map(Into::into).collect())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn project_archive(
    state: tauri::State<'_, DesktopState>,
    slug: String,
    archived: bool,
    revision: Option<i64>,
) -> Result<ProjectDto, AppErrorDto> {
    let app = state.app.lock().await;
    project_archive_inner(&app, slug, archived, revision)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn project_delete(
    state: tauri::State<'_, DesktopState>,
    slug: String,
    revision: Option<i64>,
) -> Result<ProjectDto, AppErrorDto> {
    let app = state.app.lock().await;
    project_delete_inner(&app, slug, revision)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn project_restore(
    state: tauri::State<'_, DesktopState>,
    slug: String,
) -> Result<ProjectDto, AppErrorDto> {
    let app = state.app.lock().await;
    project_restore_inner(&app, slug).await.map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn project_note_set(
    state: tauri::State<'_, DesktopState>,
    slug: String,
    markdown: String,
    revision: Option<i64>,
) -> Result<ProjectDto, AppErrorDto> {
    let app = state.app.lock().await;
    project_note_set_inner(&app, slug, markdown, revision)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn task_create(
    state: tauri::State<'_, DesktopState>,
    project_slug: String,
    title: String,
    column: Option<Column>,
    urgent: Option<bool>,
) -> Result<TaskDetailDto, AppErrorDto> {
    let app = state.app.lock().await;
    task_create_inner(&app, project_slug, title, column, urgent)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn task_list(
    state: tauri::State<'_, DesktopState>,
    project_slug: String,
) -> Result<Vec<TaskSummaryDto>, AppErrorDto> {
    let app = state.app.lock().await;
    task_list_inner(&app, project_slug)
        .await
        .map(|items| items.into_iter().map(Into::into).collect())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn task_show(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
) -> Result<TaskDetailDto, AppErrorDto> {
    let app = state.app.lock().await;
    task_show_inner(&app, display_id).await.map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn task_update(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
    title: Option<String>,
    note_markdown: Option<String>,
    urgent: Option<bool>,
    column: Option<Column>,
    #[serde(default, deserialize_with = "deserialize_present_option")]
    before_display_id: Option<Option<String>>,
    revision: Option<i64>,
) -> Result<TaskDetailDto, AppErrorDto> {
    let app = state.app.lock().await;
    task_update_inner(
        &app,
        TaskPatchArgs {
            display_id,
            title,
            note_markdown,
            urgent,
            column,
            before_display_id,
            revision,
        },
    )
    .await
    .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn task_move(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
    column: Column,
    revision: Option<i64>,
) -> Result<TaskDetailDto, AppErrorDto> {
    let app = state.app.lock().await;
    task_move_inner(&app, display_id, column, revision)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn task_reorder(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
    before_display_id: Option<String>,
    revision: Option<i64>,
) -> Result<TaskDetailDto, AppErrorDto> {
    let app = state.app.lock().await;
    task_reorder_inner(&app, display_id, before_display_id, revision)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn task_urgent(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
    urgent: bool,
    revision: Option<i64>,
) -> Result<TaskDetailDto, AppErrorDto> {
    let app = state.app.lock().await;
    task_urgent_inner(&app, display_id, urgent, revision)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn task_delete(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
    revision: Option<i64>,
) -> Result<TaskDetailDto, AppErrorDto> {
    let app = state.app.lock().await;
    task_delete_inner(&app, display_id, revision)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn task_restore(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
) -> Result<TaskDetailDto, AppErrorDto> {
    let app = state.app.lock().await;
    task_restore_inner(&app, display_id).await.map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn task_note_set(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
    markdown: String,
    revision: Option<i64>,
) -> Result<TaskDetailDto, AppErrorDto> {
    let app = state.app.lock().await;
    task_note_set_inner(&app, display_id, markdown, revision)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn link_add(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
    kind: LinkKind,
    value: String,
    revision: Option<i64>,
) -> Result<TaskDetailDto, AppErrorDto> {
    let app = state.app.lock().await;
    link_add_inner(&app, display_id, kind, value, revision)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn link_remove(
    state: tauri::State<'_, DesktopState>,
    link_id: String,
    revision: Option<i64>,
) -> Result<TaskDetailDto, AppErrorDto> {
    let app = state.app.lock().await;
    link_remove_inner(&app, link_id, revision)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn run_start(
    state: tauri::State<'_, DesktopState>,
    display_id: String,
    agent: String,
    session_id: Option<String>,
) -> Result<RunDto, AppErrorDto> {
    let app = state.app.lock().await;
    run_start_inner(&app, display_id, agent, session_id)
        .await
        .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn run_patch(
    state: tauri::State<'_, DesktopState>,
    run_display_id: String,
    op: RunOp,
    message: Option<String>,
    reason: Option<String>,
    summary: Option<String>,
    revision: Option<i64>,
) -> Result<RunDto, AppErrorDto> {
    let app = state.app.lock().await;
    run_patch_inner(
        &app,
        run_display_id,
        op,
        message,
        reason,
        summary,
        revision,
    )
    .await
    .map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn trash_list(state: tauri::State<'_, DesktopState>) -> Result<TrashDto, AppErrorDto> {
    let app = state.app.lock().await;
    trash_list_inner(&app).await.map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn undo(state: tauri::State<'_, DesktopState>) -> Result<UndoResultDto, AppErrorDto> {
    let app = state.app.lock().await;
    undo_inner(&app).await.map(Into::into)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn sync(
    state: tauri::State<'_, DesktopState>,
    after: i64,
) -> Result<SyncDeltaDto, AppErrorDto> {
    let app = state.app.lock().await;
    sync_inner(&app, after).await.map(Into::into)
}
