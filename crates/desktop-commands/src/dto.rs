use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use taskboard_application::{
    BoardStatus, InboxCounts, OccupancyGroup, OccupancyRun, StatusLine, SyncDelta, Trash,
    UndoResult,
};
use taskboard_core::{
    Activity, ActorKind, CardDisplayStatus, Check, Column, Comment, EntityType, InboxItem, Link,
    LinkKind, Project, Run, RunStatus, TaskDetail, TaskSummary,
};
use taskboard_store_sqlite::UiState;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub repo_path: Option<String>,
    pub archived: bool,
    pub note_markdown: String,
    pub sort_order: i64,
    pub revision: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl From<Project> for ProjectDto {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            slug: project.slug,
            name: project.name,
            repo_path: project.repo_path,
            archived: project.archived,
            note_markdown: project.note_markdown,
            sort_order: project.sort_order,
            revision: project.revision,
            created_at: project.created_at,
            updated_at: project.updated_at,
            deleted_at: project.deleted_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummaryDto {
    pub id: Uuid,
    pub display_id: String,
    pub project_id: Uuid,
    pub title: String,
    pub column: Column,
    pub urgent: bool,
    pub revision: i64,
    pub display_status: CardDisplayStatus,
    pub run_message: Option<String>,
    pub waiting_reason: Option<String>,
    pub reply: Option<String>,
    pub blocked_by: Vec<String>,
    pub blocks: Vec<String>,
    pub stale: bool,
    pub checklist_done: i64,
    pub checklist_total: i64,
    pub worktree_path: Option<String>,
    pub branch: Option<String>,
}

impl From<TaskSummary> for TaskSummaryDto {
    fn from(task: TaskSummary) -> Self {
        Self {
            id: task.id,
            display_id: task.display_id,
            project_id: task.project_id,
            title: task.title,
            column: task.column,
            urgent: task.urgent,
            revision: task.revision,
            display_status: task.display_status,
            run_message: task.run_message,
            waiting_reason: task.waiting_reason,
            reply: task.reply,
            blocked_by: task.blocked_by,
            blocks: task.blocks,
            stale: task.stale,
            checklist_done: task.checklist_done,
            checklist_total: task.checklist_total,
            worktree_path: task.worktree_path,
            branch: task.branch,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkDto {
    pub id: Uuid,
    pub task_id: Uuid,
    pub kind: LinkKind,
    pub value: String,
    pub sort_order: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentDto {
    pub id: Uuid,
    pub task_id: Uuid,
    pub actor_kind: ActorKind,
    pub actor_label: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

impl From<Comment> for CommentDto {
    fn from(comment: Comment) -> Self {
        Self {
            id: comment.id,
            task_id: comment.task_id,
            actor_kind: comment.actor_kind,
            actor_label: comment.actor_label,
            body: comment.body,
            created_at: comment.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckDto {
    pub id: Uuid,
    pub display_id: String,
    pub task_id: Uuid,
    pub text: String,
    pub done: bool,
    pub sort_order: i64,
}

impl From<Check> for CheckDto {
    fn from(check: Check) -> Self {
        Self {
            id: check.id,
            display_id: check.display_id,
            task_id: check.task_id,
            text: check.text,
            done: check.done,
            sort_order: check.sort_order,
        }
    }
}

impl From<Link> for LinkDto {
    fn from(link: Link) -> Self {
        Self {
            id: link.id,
            task_id: link.task_id,
            kind: link.kind,
            value: link.value,
            sort_order: link.sort_order,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDto {
    pub id: Uuid,
    pub display_id: String,
    pub task_id: Uuid,
    pub agent: String,
    pub session_id: Option<String>,
    pub status: RunStatus,
    pub message: Option<String>,
    pub waiting_reason: Option<String>,
    pub summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub revision: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub worktree_path: Option<String>,
    pub branch: Option<String>,
}

impl From<Run> for RunDto {
    fn from(run: Run) -> Self {
        Self {
            id: run.id,
            display_id: run.display_id,
            task_id: run.task_id,
            agent: run.agent,
            session_id: run.session_id,
            status: run.status,
            message: run.message,
            waiting_reason: run.waiting_reason,
            summary: run.summary,
            started_at: run.started_at,
            ended_at: run.ended_at,
            revision: run.revision,
            created_at: run.created_at,
            updated_at: run.updated_at,
            worktree_path: run.worktree_path,
            branch: run.branch,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDto {
    pub id: Uuid,
    pub sequence: i64,
    pub actor_kind: ActorKind,
    pub actor_label: String,
    pub operation: String,
    pub entity_type: EntityType,
    pub entity_id: Uuid,
    pub previous_revision: Option<i64>,
    pub before_json: Option<Value>,
    pub after_json: Option<Value>,
    pub created_at: DateTime<Utc>,
}

impl From<Activity> for ActivityDto {
    fn from(activity: Activity) -> Self {
        Self {
            id: activity.id,
            sequence: activity.sequence,
            actor_kind: activity.actor_kind,
            actor_label: activity.actor_label,
            operation: activity.operation,
            entity_type: activity.entity_type,
            entity_id: activity.entity_id,
            previous_revision: activity.previous_revision,
            before_json: activity.before_json,
            after_json: activity.after_json,
            created_at: activity.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetailDto {
    pub id: Uuid,
    pub display_id: String,
    pub project_id: Uuid,
    pub title: String,
    pub column: Column,
    pub urgent: bool,
    pub revision: i64,
    pub display_status: CardDisplayStatus,
    pub run_message: Option<String>,
    pub waiting_reason: Option<String>,
    pub reply: Option<String>,
    pub note_markdown: String,
    pub links: Vec<LinkDto>,
    pub runs: Vec<RunDto>,
    pub comments: Vec<CommentDto>,
    pub checks: Vec<CheckDto>,
    pub worktree_path: Option<String>,
    pub branch: Option<String>,
    pub recent_activities: Vec<ActivityDto>,
}

impl From<TaskDetail> for TaskDetailDto {
    fn from(task: TaskDetail) -> Self {
        Self {
            id: task.id,
            display_id: task.display_id,
            project_id: task.project_id,
            title: task.title,
            column: task.column,
            urgent: task.urgent,
            revision: task.revision,
            display_status: task.display_status,
            run_message: task.run_message,
            waiting_reason: task.waiting_reason,
            reply: task.reply,
            note_markdown: task.note_markdown,
            links: task.links.into_iter().map(LinkDto::from).collect(),
            runs: task.runs.into_iter().map(RunDto::from).collect(),
            comments: task.comments.into_iter().map(CommentDto::from).collect(),
            checks: task.checks.into_iter().map(CheckDto::from).collect(),
            worktree_path: task.worktree_path,
            branch: task.branch,
            recent_activities: task
                .recent_activities
                .into_iter()
                .map(ActivityDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncDeltaDto {
    pub sequence: i64,
    pub projects: Vec<ProjectDto>,
    pub tasks: Vec<TaskDetailDto>,
    pub runs: Vec<RunDto>,
}

impl From<SyncDelta> for SyncDeltaDto {
    fn from(delta: SyncDelta) -> Self {
        Self {
            sequence: delta.sequence,
            projects: delta.projects.into_iter().map(ProjectDto::from).collect(),
            tasks: delta.tasks.into_iter().map(TaskDetailDto::from).collect(),
            runs: delta.runs.into_iter().map(RunDto::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashDto {
    pub projects: Vec<ProjectDto>,
    pub tasks: Vec<TaskSummaryDto>,
}

impl From<Trash> for TrashDto {
    fn from(trash: Trash) -> Self {
        Self {
            projects: trash.projects.into_iter().map(ProjectDto::from).collect(),
            tasks: trash.tasks.into_iter().map(TaskSummaryDto::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxItemDto {
    pub id: Uuid,
    pub display_id: String,
    pub project_id: Uuid,
    pub project_slug: String,
    pub project_name: String,
    pub title: String,
    pub column: Column,
    pub urgent: bool,
    pub revision: i64,
    pub display_status: CardDisplayStatus,
    pub run_message: Option<String>,
    pub waiting_reason: Option<String>,
    pub reason: String,
    pub stale: bool,
    pub updated_at: DateTime<Utc>,
}

impl From<InboxItem> for InboxItemDto {
    fn from(item: InboxItem) -> Self {
        Self {
            id: item.id,
            display_id: item.display_id,
            project_id: item.project_id,
            project_slug: item.project_slug,
            project_name: item.project_name,
            title: item.title,
            column: item.column,
            urgent: item.urgent,
            revision: item.revision,
            display_status: item.display_status,
            run_message: item.run_message,
            waiting_reason: item.waiting_reason,
            reason: item.reason,
            stale: item.stale,
            updated_at: item.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoResultDto {
    pub entity_type: EntityType,
    pub entity: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusLineDto {
    pub display_id: String,
    pub status: String,
    pub agent: Option<String>,
    pub detail: String,
}

impl From<StatusLine> for StatusLineDto {
    fn from(line: StatusLine) -> Self {
        Self {
            display_id: line.display_id,
            status: line.status,
            agent: line.agent,
            detail: line.detail,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxCountsDto {
    pub total: usize,
    pub waiting: usize,
    pub failed: usize,
    pub stale: usize,
    pub review: usize,
    pub urgent: usize,
}

impl From<InboxCounts> for InboxCountsDto {
    fn from(counts: InboxCounts) -> Self {
        Self {
            total: counts.total,
            waiting: counts.waiting,
            failed: counts.failed,
            stale: counts.stale,
            review: counts.review,
            urgent: counts.urgent,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardStatusDto {
    pub inbox: InboxCountsDto,
    pub inbox_head: Vec<StatusLineDto>,
    pub open_runs: usize,
    pub open_run_head: Vec<StatusLineDto>,
    pub stale: usize,
    pub stale_head: Vec<StatusLineDto>,
    pub ready: usize,
    pub ready_head: Vec<StatusLineDto>,
    pub in_review: usize,
    pub in_review_head: Vec<StatusLineDto>,
    pub blocked: usize,
    pub blocked_head: Vec<StatusLineDto>,
}

impl From<BoardStatus> for BoardStatusDto {
    fn from(snap: BoardStatus) -> Self {
        Self {
            inbox: InboxCountsDto::from(snap.inbox),
            inbox_head: snap
                .inbox_head
                .into_iter()
                .map(StatusLineDto::from)
                .collect(),
            open_runs: snap.open_runs,
            open_run_head: snap
                .open_run_head
                .into_iter()
                .map(StatusLineDto::from)
                .collect(),
            stale: snap.stale,
            stale_head: snap
                .stale_head
                .into_iter()
                .map(StatusLineDto::from)
                .collect(),
            ready: snap.ready,
            ready_head: snap
                .ready_head
                .into_iter()
                .map(StatusLineDto::from)
                .collect(),
            in_review: snap.in_review,
            in_review_head: snap
                .in_review_head
                .into_iter()
                .map(StatusLineDto::from)
                .collect(),
            blocked: snap.blocked,
            blocked_head: snap
                .blocked_head
                .into_iter()
                .map(StatusLineDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OccupancyRunDto {
    pub run_display_id: String,
    pub task_display_id: String,
    pub status: String,
    pub agent: String,
}

impl From<OccupancyRun> for OccupancyRunDto {
    fn from(run: OccupancyRun) -> Self {
        Self {
            run_display_id: run.run_display_id,
            task_display_id: run.task_display_id,
            status: run.status,
            agent: run.agent,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OccupancyGroupDto {
    pub worktree_path: String,
    pub runs: Vec<OccupancyRunDto>,
}

impl From<OccupancyGroup> for OccupancyGroupDto {
    fn from(group: OccupancyGroup) -> Self {
        Self {
            worktree_path: group.worktree_path,
            runs: group.runs.into_iter().map(OccupancyRunDto::from).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiStateDto {
    pub last_project_slug: Option<String>,
}

impl From<UiState> for UiStateDto {
    fn from(state: UiState) -> Self {
        Self {
            last_project_slug: state.last_project_slug,
        }
    }
}

impl From<UndoResult> for UndoResultDto {
    fn from(result: UndoResult) -> Self {
        Self {
            entity_type: result.entity_type,
            entity: json_keys_to_camel(result.entity),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunOp {
    Update,
    Wait,
    Fail,
    Finish,
    Cancel,
}

/// Distinguishes JSON field absent (`None`) from explicit `null` (`Some(None)`).
pub fn deserialize_present_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}

pub fn json_keys_to_camel(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, nested)| (snake_to_camel(&key), json_keys_to_camel(nested)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(json_keys_to_camel).collect()),
        other => other,
    }
}

fn snake_to_camel(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut upper = false;
    for ch in raw.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            out.extend(ch.to_uppercase());
            upper = false;
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::ProjectDto;
    use chrono::{TimeZone, Utc};
    use taskboard_core::Project;
    use uuid::Uuid;

    #[test]
    fn project_dto_serializes_camel_case_keys() {
        let dto = ProjectDto::from(Project {
            id: Uuid::nil(),
            slug: "renai-sim".into(),
            name: "Renai Sim".into(),
            repo_path: None,
            archived: false,
            note_markdown: String::new(),
            sort_order: 0,
            revision: 1,
            created_at: Utc.timestamp_opt(0, 0).unwrap(),
            updated_at: Utc.timestamp_opt(0, 0).unwrap(),
            deleted_at: None,
        });
        let value = serde_json::to_value(&dto).unwrap();
        assert!(value.get("noteMarkdown").is_some());
        assert!(value.get("note_markdown").is_none());
        assert!(value.get("repoPath").is_some());
        assert_eq!(value["slug"], "renai-sim");
    }
}
