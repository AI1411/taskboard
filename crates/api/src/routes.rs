use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post};
use axum::{Json, Router};
use serde::Serialize;
use serde_json::{json, Value};
use taskboard_application::{
    Actor, AppError, LinkAdd, ProjectAdd, ProjectUpdate, RunFail, RunFinish, RunStart, RunUpdate,
    RunWait, TaskCreate, TaskUpdate,
};
use taskboard_core::ActorKind;
use uuid::Uuid;

use crate::dto::{
    json_keys_to_camel, AddLinkBody, BackupBody, CreateProjectBody, CreateTaskBody,
    ListProjectsQuery, PatchProjectBody, PatchRunBody, PatchTaskBody, ProjectDto,
    ReorderProjectsBody, RunDto, RunOp, StartRunBody, SyncDeltaDto, SyncQuery, TaskDetailDto,
    TrashDto,
};
use crate::origin::origin_allowed;
use crate::server::{
    forbidden_origin, session_from_header, session_from_header_or_cookie, unauthorized, AppState,
};

pub(crate) fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/bootstrap", get(bootstrap))
        .route("/api/v1/sync", get(sync))
        .route("/api/v1/projects", get(list_projects).post(create_project))
        .route("/api/v1/projects/reorder", post(reorder_projects))
        .route(
            "/api/v1/projects/:slug/tasks",
            get(list_tasks).post(create_task),
        )
        .route("/api/v1/projects/:slug/restore", post(restore_project))
        .route(
            "/api/v1/projects/:slug",
            patch(patch_project).delete(delete_project),
        )
        .route("/api/v1/tasks/:display_id/links", post(add_link))
        .route("/api/v1/tasks/:display_id/runs", post(start_run))
        .route("/api/v1/tasks/:display_id/restore", post(restore_task))
        .route(
            "/api/v1/tasks/:display_id",
            get(show_task).patch(patch_task).delete(delete_task),
        )
        .route("/api/v1/links/:id", delete(remove_link))
        .route("/api/v1/runs/:display_id", patch(patch_run))
        .route("/api/v1/trash", get(list_trash))
        .route("/api/v1/undo", post(undo))
        .route("/api/v1/backups/export", post(backup_export))
        .route("/api/v1/backups/import", post(backup_import))
}

type ApiResult = Result<Response, Response>;

fn web_actor() -> Actor {
    Actor {
        kind: ActorKind::Web,
        label: "local-web".into(),
    }
}

fn require_read(state: &AppState, headers: &HeaderMap) -> Result<(), Response> {
    if session_from_header_or_cookie(headers, &state.session.0) {
        Ok(())
    } else {
        Err(unauthorized())
    }
}

fn require_mutation(state: &AppState, headers: &HeaderMap) -> Result<(), Response> {
    if !session_from_header(headers, &state.session.0) {
        return Err(unauthorized());
    }
    if !origin_allowed(headers.get(header::ORIGIN), state.port) {
        return Err(forbidden_origin());
    }
    Ok(())
}

fn if_match(headers: &HeaderMap) -> Result<Option<i64>, Response> {
    let Some(value) = headers.get("If-Match") else {
        return Ok(None);
    };
    let raw = value.to_str().map_err(|_| invalid_if_match())?;
    let raw = raw.trim().trim_matches('"').trim();
    if raw.is_empty() {
        return Ok(None);
    }
    raw.parse().map(Some).map_err(|_| invalid_if_match())
}

fn advance_if_match(updated_revision: i64) -> Option<i64> {
    Some(updated_revision)
}

fn invalid_if_match() -> Response {
    app_error(AppError::Validation {
        field: "If-Match".into(),
        message: "must be a revision number".into(),
    })
}

fn missing_field(field: &str) -> Response {
    app_error(AppError::Validation {
        field: field.into(),
        message: "is required".into(),
    })
}

fn entity<T: Serialize>(entity: T, revision: i64) -> Response {
    Json(json!({
        "ok": true,
        "entity": entity,
        "revision": revision,
    }))
    .into_response()
}

fn entities<T: Serialize>(entities: T) -> Response {
    Json(json!({
        "ok": true,
        "entities": entities,
    }))
    .into_response()
}

fn ok() -> Response {
    Json(json!({ "ok": true })).into_response()
}

fn app_error(err: AppError) -> Response {
    let status = match &err {
        AppError::Validation { .. } => StatusCode::BAD_REQUEST,
        AppError::NotFound { .. } => StatusCode::NOT_FOUND,
        AppError::DuplicateSlug { .. }
        | AppError::RevisionConflict { .. }
        | AppError::UndoConflict { .. }
        | AppError::DifferentColumn { .. } => StatusCode::CONFLICT,
        AppError::DatabaseBusy => StatusCode::SERVICE_UNAVAILABLE,
        AppError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let mut body = json!({
        "code": err.code(),
        "message": err.to_string(),
    });
    match err {
        AppError::Validation { field, .. } => {
            body["field"] = json!(field);
        }
        AppError::DuplicateSlug { slug } => {
            body["slug"] = json!(slug);
        }
        AppError::RevisionConflict { current } | AppError::UndoConflict { current } => {
            body["current"] = json_keys_to_camel(current);
        }
        _ => {}
    }
    (status, Json(json!({ "error": body }))).into_response()
}

async fn bootstrap(State(state): State<Arc<AppState>>, headers: HeaderMap) -> ApiResult {
    require_read(&state, &headers)?;
    let activity_sequence = state.app.activity_head().await.unwrap_or(0);
    Ok(Json(json!({
        "session": state.session.0,
        "activitySequence": activity_sequence,
    }))
    .into_response())
}

async fn sync(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<SyncQuery>,
) -> ApiResult {
    require_read(&state, &headers)?;
    let delta = state.app.sync(query.after).await.map_err(app_error)?;
    Ok(Json(SyncDeltaDto::from(delta)).into_response())
}

async fn list_projects(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ListProjectsQuery>,
) -> ApiResult {
    require_read(&state, &headers)?;
    let include_archived = query.archived.unwrap_or(false);
    let projects = state
        .app
        .project_list(include_archived)
        .await
        .map_err(app_error)?;
    Ok(entities(
        projects
            .into_iter()
            .map(ProjectDto::from)
            .collect::<Vec<_>>(),
    ))
}

async fn create_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CreateProjectBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let project = state
        .app
        .project_add(
            &web_actor(),
            ProjectAdd {
                name: body.name,
                repo_path: body.repo_path,
                slug: body.slug,
            },
        )
        .await
        .map_err(app_error)?;
    let dto = ProjectDto::from(project);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn patch_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(mut slug): Path<String>,
    Json(body): Json<PatchProjectBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let actor = web_actor();
    let mut revision = if_match(&headers)?;
    let mut project = None;
    if body.name.is_some() || body.repo_path.is_some() || body.slug.is_some() {
        let updated = state
            .app
            .project_update(
                &actor,
                ProjectUpdate {
                    slug: slug.clone(),
                    name: body.name,
                    repo_path: body.repo_path,
                    new_slug: body.slug,
                    revision,
                },
            )
            .await
            .map_err(app_error)?;
        slug = updated.slug.clone();
        revision = advance_if_match(updated.revision);
        project = Some(updated);
    }
    if let Some(archived) = body.archived {
        let updated = state
            .app
            .project_archive(&actor, &slug, archived, revision)
            .await
            .map_err(app_error)?;
        revision = advance_if_match(updated.revision);
        project = Some(updated);
    }
    if let Some(note_markdown) = body.note_markdown {
        project = Some(
            state
                .app
                .project_note_set(&actor, &slug, note_markdown, revision)
                .await
                .map_err(app_error)?,
        );
    }
    let project = match project {
        Some(project) => project,
        None => state.app.project_show(&slug).await.map_err(app_error)?,
    };
    let dto = ProjectDto::from(project);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn reorder_projects(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ReorderProjectsBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let projects = state
        .app
        .project_reorder(&web_actor(), &body.slugs)
        .await
        .map_err(app_error)?;
    Ok(entities(
        projects
            .into_iter()
            .map(ProjectDto::from)
            .collect::<Vec<_>>(),
    ))
}

async fn delete_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let project = state
        .app
        .project_delete(&web_actor(), &slug, if_match(&headers)?)
        .await
        .map_err(app_error)?;
    let dto = ProjectDto::from(project);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn restore_project(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let project = state
        .app
        .project_restore(&web_actor(), &slug)
        .await
        .map_err(app_error)?;
    let dto = ProjectDto::from(project);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn list_tasks(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> ApiResult {
    require_read(&state, &headers)?;
    let tasks = state.app.task_list(&slug).await.map_err(app_error)?;
    Ok(entities(
        tasks
            .into_iter()
            .map(crate::dto::TaskSummaryDto::from)
            .collect::<Vec<_>>(),
    ))
}

async fn create_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(slug): Path<String>,
    Json(body): Json<CreateTaskBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let task = state
        .app
        .task_create(
            &web_actor(),
            TaskCreate {
                project_slug: slug,
                title: body.title,
                column: body.column,
                urgent: body.urgent,
            },
        )
        .await
        .map_err(app_error)?;
    let dto = TaskDetailDto::from(task);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn show_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(display_id): Path<String>,
) -> ApiResult {
    require_read(&state, &headers)?;
    let task = state.app.task_show(&display_id).await.map_err(app_error)?;
    let dto = TaskDetailDto::from(task);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn patch_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(display_id): Path<String>,
    Json(body): Json<PatchTaskBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let actor = web_actor();
    let mut revision = if_match(&headers)?;
    let mut task = None;
    if body.title.is_some() {
        let updated = state
            .app
            .task_update(
                &actor,
                TaskUpdate {
                    display_id: display_id.clone(),
                    title: body.title,
                    revision,
                },
            )
            .await
            .map_err(app_error)?;
        revision = advance_if_match(updated.revision);
        task = Some(updated);
    }
    if let Some(note_markdown) = body.note_markdown {
        let updated = state
            .app
            .task_note_set(&actor, &display_id, note_markdown, revision)
            .await
            .map_err(app_error)?;
        revision = advance_if_match(updated.revision);
        task = Some(updated);
    }
    if let Some(urgent) = body.urgent {
        let updated = state
            .app
            .task_urgent(&actor, &display_id, urgent, revision)
            .await
            .map_err(app_error)?;
        revision = advance_if_match(updated.revision);
        task = Some(updated);
    }
    if let Some(column) = body.column {
        let updated = state
            .app
            .task_move(&actor, &display_id, column, revision)
            .await
            .map_err(app_error)?;
        revision = advance_if_match(updated.revision);
        task = Some(updated);
    }
    if let Some(before_display_id) = body.before_display_id {
        let updated = state
            .app
            .task_reorder(&actor, &display_id, before_display_id.as_deref(), revision)
            .await
            .map_err(app_error)?;
        #[allow(unused_assignments)]
        {
            revision = advance_if_match(updated.revision);
        }
        task = Some(updated);
    }
    let task = match task {
        Some(task) => task,
        None => state.app.task_show(&display_id).await.map_err(app_error)?,
    };
    let dto = TaskDetailDto::from(task);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn delete_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(display_id): Path<String>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let task = state
        .app
        .task_delete(&web_actor(), &display_id, if_match(&headers)?)
        .await
        .map_err(app_error)?;
    let dto = TaskDetailDto::from(task);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn restore_task(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(display_id): Path<String>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let task = state
        .app
        .task_restore(&web_actor(), &display_id)
        .await
        .map_err(app_error)?;
    let dto = TaskDetailDto::from(task);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn add_link(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(display_id): Path<String>,
    Json(body): Json<AddLinkBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let task = state
        .app
        .link_add(
            &web_actor(),
            LinkAdd {
                task_display_id: display_id,
                kind: body.kind,
                value: body.value,
                revision: if_match(&headers)?,
            },
        )
        .await
        .map_err(app_error)?;
    let dto = TaskDetailDto::from(task);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn remove_link(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let task = state
        .app
        .link_remove(&web_actor(), id, if_match(&headers)?)
        .await
        .map_err(app_error)?;
    let dto = TaskDetailDto::from(task);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn start_run(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(display_id): Path<String>,
    Json(body): Json<StartRunBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let run = state
        .app
        .run_start(
            &web_actor(),
            RunStart {
                task_display_id: display_id,
                agent: body.agent,
                session_id: body.session_id,
            },
        )
        .await
        .map_err(app_error)?;
    let dto = RunDto::from(run);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn patch_run(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(display_id): Path<String>,
    Json(body): Json<PatchRunBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    let actor = web_actor();
    let revision = if_match(&headers)?;
    let run = match body.op {
        RunOp::Update => {
            state
                .app
                .run_update(
                    &actor,
                    RunUpdate {
                        run_display_id: display_id,
                        message: body.message,
                        revision,
                    },
                )
                .await
        }
        RunOp::Wait => {
            let reason = body.reason.ok_or_else(|| missing_field("reason"))?;
            state
                .app
                .run_wait(
                    &actor,
                    RunWait {
                        run_display_id: display_id,
                        reason,
                        revision,
                    },
                )
                .await
        }
        RunOp::Fail => {
            let summary = body.summary.ok_or_else(|| missing_field("summary"))?;
            state
                .app
                .run_fail(
                    &actor,
                    RunFail {
                        run_display_id: display_id,
                        summary,
                        revision,
                    },
                )
                .await
        }
        RunOp::Finish => {
            let summary = body.summary.ok_or_else(|| missing_field("summary"))?;
            state
                .app
                .run_finish(
                    &actor,
                    RunFinish {
                        run_display_id: display_id,
                        summary,
                        revision,
                    },
                )
                .await
        }
    }
    .map_err(app_error)?;
    let dto = RunDto::from(run);
    let revision = dto.revision;
    Ok(entity(dto, revision))
}

async fn list_trash(State(state): State<Arc<AppState>>, headers: HeaderMap) -> ApiResult {
    require_read(&state, &headers)?;
    let trash = state.app.trash_list().await.map_err(app_error)?;
    Ok(Json(json!({
        "ok": true,
        "entity": TrashDto::from(trash),
    }))
    .into_response())
}

async fn undo(State(state): State<Arc<AppState>>, headers: HeaderMap) -> ApiResult {
    require_mutation(&state, &headers)?;
    let result = state.app.undo(&web_actor()).await.map_err(app_error)?;
    let revision = result
        .entity
        .get("revision")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    Ok(entity(json_keys_to_camel(result.entity), revision))
}

async fn backup_export(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<BackupBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    state
        .app
        .backup_export(&PathBuf::from(body.path))
        .await
        .map_err(app_error)?;
    Ok(ok())
}

async fn backup_import(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<BackupBody>,
) -> ApiResult {
    require_mutation(&state, &headers)?;
    state
        .app
        .backup_import(&PathBuf::from(body.path))
        .await
        .map_err(app_error)?;
    Ok(ok())
}

#[cfg(test)]
mod tests {
    use super::advance_if_match;

    fn chain_revisions<E>(
        mut revision: Option<i64>,
        first: impl FnOnce(Option<i64>) -> Result<i64, E>,
        second: impl FnOnce(Option<i64>) -> Result<i64, E>,
    ) -> Result<Option<i64>, E> {
        revision = advance_if_match(first(revision)?);
        revision = advance_if_match(second(revision)?);
        Ok(revision)
    }

    #[test]
    fn advance_if_match_keeps_revision_not_none() {
        let n = 2_i64;
        assert_eq!(advance_if_match(n), Some(n));
        assert_ne!(advance_if_match(n), None);
    }

    #[test]
    fn chain_revisions_second_op_sees_advanced_if_match() {
        let mut first_seen = None;
        let mut second_seen = None;
        let result = chain_revisions(
            Some(1),
            |rev| {
                first_seen = rev;
                Ok::<_, ()>(2)
            },
            |rev| {
                second_seen = rev;
                Ok::<_, ()>(3)
            },
        );
        assert_eq!(result, Ok(Some(3)));
        assert_eq!(first_seen, Some(1));
        assert_eq!(second_seen, Some(2));
        assert_ne!(second_seen, None);
        assert_ne!(second_seen, Some(1));
    }
}
