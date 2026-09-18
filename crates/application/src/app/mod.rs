use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;

use taskboard_core::{
    ActivityEntry, Check, Column, Comment, InboxGroup, InboxItem, Project, Run, RunStatus,
    TaskDetail, TaskSummary,
};

use crate::actor::Actor;
use crate::commands::{
    ActivityQuery, BoardStatus, CheckAdd, CommentAdd, InboxCounts, InboxScope, LinkAdd, NextClaim,
    OccupancyGroup, OccupancyQuery, ProjectAdd, ProjectUpdate, ReplyContinueResult, ReviewTask,
    RunCancel, RunContinue, RunFail, RunFinish, RunListQuery, RunStart, RunUpdate, RunWait,
    TaskCreate, TaskListQuery, TaskSpawn, TaskUpdate, STATUS_HEAD,
};
use crate::error::AppError;
use crate::store::{Store, SyncDelta, Trash, UndoResult};

mod board;
mod checks;
mod comments;
mod helpers;
mod projects;
mod runs;
mod tasks;
mod undo;

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
        let result = projects::project_add_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn project_list(&self, include_archived: bool) -> Result<Vec<Project>, AppError> {
        let mut store = self.store.lock().await;
        store.list_projects(include_archived).await
    }

    pub async fn project_show(&self, slug: &str) -> Result<Project, AppError> {
        let mut store = self.store.lock().await;
        helpers::require_live_project(&mut **store, slug).await
    }

    pub async fn detect_project(
        &self,
        cwd: &Path,
        env_slug: Option<&str>,
    ) -> Result<Project, AppError> {
        let live = self.project_list(false).await?;
        if let Some(slug) = env_slug.filter(|value| !value.is_empty()) {
            return live
                .into_iter()
                .find(|project| project.slug == slug)
                .ok_or_else(|| AppError::NotFound {
                    entity: "project".into(),
                    id: slug.to_string(),
                });
        }
        crate::detect::pick_by_repo_path(&live, cwd).ok_or(AppError::ProjectRequired)
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
        let result = projects::project_update_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
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
        let result = projects::project_reorder_inner(store, actor, slugs_in_order, now).await;
        helpers::commit_or_rollback(store, result).await
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
        let result =
            projects::project_archive_inner(store, actor, slug, archived, revision, now).await;
        helpers::commit_or_rollback(store, result).await
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
        let result = projects::project_delete_inner(store, actor, slug, revision, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn project_restore(&self, actor: &Actor, slug: &str) -> Result<Project, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = projects::project_restore_inner(store, actor, slug, now).await;
        helpers::commit_or_rollback(store, result).await
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
        let result =
            projects::project_note_set_inner(store, actor, slug, markdown, revision, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn task_create(
        &self,
        actor: &Actor,
        cmd: TaskCreate,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = tasks::task_create_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn task_spawn(
        &self,
        actor: &Actor,
        cmd: TaskSpawn,
    ) -> Result<Vec<TaskDetail>, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = tasks::task_spawn_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn task_list(&self, project_slug: &str) -> Result<Vec<TaskSummary>, AppError> {
        self.task_query(TaskListQuery {
            project: Some(project_slug.to_string()),
            statuses: Vec::new(),
            column: None,
            agent: None,
            blocked: false,
            ready: false,
        })
        .await
    }

    pub async fn task_query(&self, query: TaskListQuery) -> Result<Vec<TaskSummary>, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        tasks::task_query_inner(store, query, now).await
    }

    pub async fn inbox(&self, scope: InboxScope) -> Result<Vec<InboxItem>, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        board::inbox_inner(store, scope, now).await
    }

    pub async fn status(&self, project: Option<String>) -> Result<BoardStatus, AppError> {
        let inbox = self
            .inbox(InboxScope {
                project: project.clone(),
                include_archived: false,
            })
            .await?;
        let ready = self
            .task_query(TaskListQuery {
                project: project.clone(),
                ready: true,
                ..TaskListQuery::default()
            })
            .await?;
        let blocked = self
            .task_query(TaskListQuery {
                project: project.clone(),
                blocked: true,
                ..TaskListQuery::default()
            })
            .await?;
        let in_review = self
            .task_query(TaskListQuery {
                project: project.clone(),
                column: Some(Column::InReview),
                ..TaskListQuery::default()
            })
            .await?;
        let tasks = self
            .task_query(TaskListQuery {
                project: project.clone(),
                ..TaskListQuery::default()
            })
            .await?;
        let task_ids: HashSet<_> = tasks.iter().map(|task| task.id).collect();
        let task_by_id: HashMap<_, _> = tasks.iter().map(|task| (task.id, task)).collect();
        let mut open = self
            .run_list(RunListQuery {
                open: true,
                ..RunListQuery::default()
            })
            .await?;
        let mut stale_runs = self.stale_list(30).await?;
        if project.is_some() {
            open.retain(|run| task_ids.contains(&run.task_id));
            stale_runs.retain(|run| task_ids.contains(&run.task_id));
        }
        let inbox_counts = InboxCounts {
            total: inbox.len(),
            waiting: board::count_group(&inbox, InboxGroup::Waiting),
            failed: board::count_group(&inbox, InboxGroup::Failed),
            stale: board::count_group(&inbox, InboxGroup::Stale),
            review: board::count_group(&inbox, InboxGroup::Review),
            urgent: board::count_group(&inbox, InboxGroup::Urgent),
        };
        Ok(BoardStatus {
            inbox: inbox_counts,
            inbox_head: inbox
                .iter()
                .take(STATUS_HEAD)
                .map(board::inbox_line)
                .collect(),
            open_runs: open.len(),
            open_run_head: open
                .iter()
                .take(STATUS_HEAD)
                .map(|run| board::run_line(run, &task_by_id))
                .collect(),
            stale: stale_runs.len(),
            stale_head: stale_runs
                .iter()
                .take(STATUS_HEAD)
                .map(|run| board::run_line(run, &task_by_id))
                .collect(),
            ready: ready.len(),
            ready_head: ready
                .iter()
                .take(STATUS_HEAD)
                .map(board::task_line)
                .collect(),
            in_review: in_review.len(),
            in_review_head: in_review
                .iter()
                .take(STATUS_HEAD)
                .map(board::task_line)
                .collect(),
            blocked: blocked.len(),
            blocked_head: blocked
                .iter()
                .take(STATUS_HEAD)
                .map(board::task_line)
                .collect(),
        })
    }

    pub async fn task_show(&self, display_id: &str) -> Result<TaskDetail, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let task = helpers::require_live_task(store, display_id).await?;
        helpers::load_task_detail(store, task).await
    }

    pub async fn task_update(
        &self,
        actor: &Actor,
        cmd: TaskUpdate,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = tasks::task_update_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn review(&self, actor: &Actor, cmd: ReviewTask) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = tasks::review_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn task_move(
        &self,
        actor: &Actor,
        display_id: &str,
        column: Column,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = tasks::task_move_inner(store, actor, display_id, column, revision, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn task_reorder(
        &self,
        actor: &Actor,
        display_id: &str,
        before_display_id: Option<&str>,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result =
            tasks::task_reorder_inner(store, actor, display_id, before_display_id, revision, now)
                .await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn task_urgent(
        &self,
        actor: &Actor,
        display_id: &str,
        urgent: bool,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result =
            tasks::task_urgent_inner(store, actor, display_id, urgent, revision, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn task_note_set(
        &self,
        actor: &Actor,
        display_id: &str,
        markdown: String,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result =
            tasks::task_note_set_inner(store, actor, display_id, markdown, revision, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn task_note_add(
        &self,
        actor: &Actor,
        display_id: &str,
        paragraph: &str,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result =
            tasks::task_note_add_inner(store, actor, display_id, paragraph, revision, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn link_add(&self, actor: &Actor, cmd: LinkAdd) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = tasks::link_add_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn link_remove(
        &self,
        actor: &Actor,
        link_id: Uuid,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = tasks::link_remove_inner(store, actor, link_id, revision, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn run_start(&self, actor: &Actor, cmd: RunStart) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = runs::run_start_inner(store, actor, cmd, now, false).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn run_start_exclusive(&self, actor: &Actor, cmd: RunStart) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = runs::run_start_inner(store, actor, cmd, now, true).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn next(&self, actor: &Actor, cmd: NextClaim) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = tasks::next_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn run_update(&self, actor: &Actor, cmd: RunUpdate) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = runs::run_update_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn run_wait(&self, actor: &Actor, cmd: RunWait) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = runs::run_wait_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn run_continue(&self, actor: &Actor, cmd: RunContinue) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = runs::run_continue_with_reply_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn run_fail(&self, actor: &Actor, cmd: RunFail) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = runs::run_fail_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn run_cancel(&self, actor: &Actor, cmd: RunCancel) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = runs::run_cancel_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn run_finish(&self, actor: &Actor, cmd: RunFinish) -> Result<Run, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = runs::run_finish_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn occupancy(&self, query: OccupancyQuery) -> Result<Vec<OccupancyGroup>, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        board::occupancy_inner(store, query).await
    }

    pub async fn run_list(&self, query: RunListQuery) -> Result<Vec<Run>, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let mut runs = if let Some(session_id) = query.session_id.as_deref() {
            let session_id = helpers::require_non_blank("session_id", session_id)?;
            store.list_runs_by_session_id(&session_id).await?
        } else {
            store.list_all_runs().await?
        };
        if query.open {
            runs.retain(|run| matches!(run.status, RunStatus::Running | RunStatus::Waiting));
        }
        if let Some(agent) = query.agent.as_deref() {
            runs.retain(|run| run.agent == agent);
        }
        Ok(runs)
    }

    pub async fn run_show(&self, display_id: &str) -> Result<Run, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let run = runs::require_run(store, display_id).await?;
        let task = store
            .get_task(run.task_id)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: "task".into(),
                id: run.task_id.to_string(),
            })?;
        Ok(helpers::with_task_workspace(run, &task))
    }

    pub async fn run_current(&self, task_display_id: &str) -> Result<Run, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let task = helpers::require_live_task(store, task_display_id).await?;
        let runs = store.list_runs(task.id).await?;
        helpers::winning_run(&runs)
            .cloned()
            .map(|run| helpers::with_task_workspace(run, &task))
            .ok_or_else(|| AppError::NotFound {
                entity: "run".into(),
                id: task_display_id.to_string(),
            })
    }

    pub async fn check_add(&self, actor: &Actor, cmd: CheckAdd) -> Result<Check, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = checks::check_add_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn check_toggle(&self, actor: &Actor, display_id: &str) -> Result<Check, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = checks::check_toggle_inner(store, actor, display_id, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn check_remove(&self, actor: &Actor, display_id: &str) -> Result<Check, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = checks::check_remove_inner(store, actor, display_id, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn check_list(&self, task_display_id: &str) -> Result<Vec<Check>, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let task = helpers::require_live_task(store, task_display_id).await?;
        store.list_checks(task.id).await
    }

    pub async fn comment_add(&self, actor: &Actor, cmd: CommentAdd) -> Result<Comment, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = comments::comment_add_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn comment_add_and_continue(
        &self,
        actor: &Actor,
        cmd: CommentAdd,
    ) -> Result<ReplyContinueResult, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = comments::comment_add_and_continue_inner(store, actor, cmd, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn comment_list(&self, task_display_id: &str) -> Result<Vec<Comment>, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let task = helpers::require_live_task(store, task_display_id).await?;
        store.list_comments(task.id).await
    }

    pub async fn comment_remove_latest(
        &self,
        actor: &Actor,
        task_display_id: &str,
    ) -> Result<Comment, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result =
            comments::comment_remove_latest_inner(store, actor, task_display_id, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn run_by_session(&self, session_id: &str) -> Result<Run, AppError> {
        let session_id = helpers::require_non_blank("session_id", session_id)?;
        let mut store = self.store.lock().await;
        store
            .list_runs_by_session_id(&session_id)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::NotFound {
                entity: "run".into(),
                id: session_id,
            })
    }

    pub async fn task_delete(
        &self,
        actor: &Actor,
        display_id: &str,
        revision: Option<i64>,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = tasks::task_delete_inner(store, actor, display_id, revision, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn task_restore(
        &self,
        actor: &Actor,
        display_id: &str,
    ) -> Result<TaskDetail, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = tasks::task_restore_inner(store, actor, display_id, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn trash_list(&self) -> Result<Trash, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        let projects = store.list_deleted_projects().await?;
        let deleted_tasks = store.list_deleted_tasks().await?;
        let mut tasks = Vec::with_capacity(deleted_tasks.len());
        for task in deleted_tasks {
            tasks.push(helpers::to_task_summary(store, task, now).await?);
        }
        Ok(Trash { projects, tasks })
    }

    pub async fn purge_expired_trash(&self, now: DateTime<Utc>) -> Result<u64, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = store.purge_expired(now, 30).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn undo(&self, actor: &Actor) -> Result<UndoResult, AppError> {
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.begin().await?;
        let result = undo::undo_inner(store, actor, now).await;
        helpers::commit_or_rollback(store, result).await
    }

    pub async fn stale_list(&self, minutes: i64) -> Result<Vec<Run>, AppError> {
        if minutes < 1 {
            return Err(AppError::Validation {
                field: "minutes".into(),
                message: "must be at least 1".into(),
            });
        }
        let now = self.clock.now();
        let mut store = self.store.lock().await;
        let runs = store.list_all_runs().await?;
        Ok(runs
            .into_iter()
            .filter(|run| helpers::is_stale(run, now, minutes))
            .collect())
    }

    pub async fn activity_head(&self) -> Result<i64, AppError> {
        let mut store = self.store.lock().await;
        store.activity_head().await
    }

    pub async fn activity_list(
        &self,
        query: ActivityQuery,
    ) -> Result<Vec<ActivityEntry>, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        if let Some(slug) = query.project.as_deref() {
            let _ = helpers::require_live_project(store, slug).await?;
        }
        if let Some(display_id) = query.task_display_id.as_deref() {
            let _ = helpers::require_live_task(store, display_id).await?;
        }
        let activities = store.list_activities_after(query.after).await?;
        let mut rows = Vec::new();
        for activity in activities {
            let Some(resolved) = helpers::resolve_activity_target(store, &activity).await? else {
                continue;
            };
            if let Some(slug) = query.project.as_deref() {
                if resolved.project_slug.as_deref() != Some(slug) {
                    continue;
                }
            }
            if let Some(display_id) = query.task_display_id.as_deref() {
                if resolved.task_display_id.as_deref() != Some(display_id) {
                    continue;
                }
            }
            rows.push(ActivityEntry {
                sequence: activity.sequence,
                created_at: activity.created_at,
                actor: activity.actor_label,
                operation: activity.operation,
                target: resolved.target,
            });
        }
        Ok(rows)
    }

    pub async fn sync(&self, after: i64) -> Result<SyncDelta, AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        helpers::sync_inner(store, after).await
    }

    pub async fn backup_export(&self, dest: &Path) -> Result<(), AppError> {
        let mut store = self.store.lock().await;
        store.backup_to(dest).await
    }

    pub async fn backup_import(&self, src: &Path) -> Result<(), AppError> {
        let mut store = self.store.lock().await;
        let store = &mut **store;
        store.validate_import(src).await?;
        let pre = store.pre_import_path()?;
        store.backup_to(&pre).await?;
        store.close_pool().await?;
        let copied = store.replace_from(src).await;
        let reopened = store.reopen_pool().await;
        match (copied, reopened) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(err), Ok(())) => Err(err),
            (Ok(()), Err(err)) => Err(err),
            (Err(err), Err(_)) => Err(err),
        }
    }
}
