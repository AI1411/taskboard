use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use taskboard_application::{SyncDelta, Trash};
use taskboard_core::{
    Activity, ActorKind, CardDisplayStatus, Column, EntityType, Link, LinkKind, Project, Run,
    RunStatus, TaskDetail, TaskSummary,
};
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
    pub note_markdown: String,
    pub links: Vec<LinkDto>,
    pub runs: Vec<RunDto>,
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
            note_markdown: task.note_markdown,
            links: task.links.into_iter().map(LinkDto::from).collect(),
            runs: task.runs.into_iter().map(RunDto::from).collect(),
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectBody {
    pub name: String,
    pub repo_path: Option<String>,
    pub slug: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchProjectBody {
    pub name: Option<String>,
    pub repo_path: Option<String>,
    pub slug: Option<String>,
    pub archived: Option<bool>,
    pub note_markdown: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderProjectsBody {
    pub slugs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskBody {
    pub title: String,
    pub column: Option<Column>,
    #[serde(default)]
    pub urgent: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchTaskBody {
    pub title: Option<String>,
    pub note_markdown: Option<String>,
    pub urgent: Option<bool>,
    pub column: Option<Column>,
    pub before_display_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddLinkBody {
    pub kind: LinkKind,
    pub value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRunBody {
    pub agent: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunOp {
    Update,
    Wait,
    Fail,
    Finish,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchRunBody {
    pub op: RunOp,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupBody {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct SyncQuery {
    pub after: i64,
}

#[derive(Debug, Deserialize)]
pub struct ListProjectsQuery {
    pub archived: Option<bool>,
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
