use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::column::Column;
use crate::display_status::CardDisplayStatus;
use crate::run_status::RunStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    Url,
    Path,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    Cli,
    Desktop,
    Web,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Project,
    Task,
    Run,
    Link,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Project {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Task {
    pub id: Uuid,
    pub display_id: String,
    pub project_id: Uuid,
    pub title: String,
    pub column: Column,
    pub urgent: bool,
    pub note_markdown: String,
    pub position: i64,
    pub revision: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TaskSummary {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TaskDetail {
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
    pub links: Vec<Link>,
    pub runs: Vec<Run>,
    pub recent_activities: Vec<Activity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Link {
    pub id: Uuid,
    pub task_id: Uuid,
    pub kind: LinkKind,
    pub value: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Run {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Activity {
    pub id: Uuid,
    pub sequence: i64,
    pub actor_kind: ActorKind,
    pub actor_label: String,
    pub operation: String,
    pub entity_type: EntityType,
    pub entity_id: Uuid,
    pub previous_revision: Option<i64>,
    pub before_json: Option<serde_json::Value>,
    pub after_json: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}
