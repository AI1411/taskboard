use serde::{Deserialize, Serialize};
use taskboard_core::{CardDisplayStatus, Column, LinkKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectAdd {
    pub name: String,
    pub repo_path: Option<String>,
    pub slug: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectUpdate {
    pub slug: String,
    pub name: Option<String>,
    pub repo_path: Option<String>,
    pub new_slug: Option<String>,
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCreate {
    pub project_slug: String,
    pub title: String,
    pub column: Option<Column>,
    pub urgent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSpawn {
    pub parent_display_id: String,
    pub titles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskUpdate {
    pub display_id: String,
    pub title: Option<String>,
    pub worktree_path: Option<Option<String>>,
    pub branch: Option<Option<String>>,
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkAdd {
    pub task_display_id: String,
    pub kind: LinkKind,
    pub value: String,
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextClaim {
    pub project: Option<String>,
    pub agent: String,
    pub session_id: Option<String>,
    pub move_to_in_progress: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStart {
    pub task_display_id: String,
    pub agent: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunUpdate {
    pub run_display_id: String,
    pub message: Option<String>,
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunWait {
    pub run_display_id: String,
    pub reason: String,
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunContinue {
    pub run_display_id: String,
    pub message: Option<String>,
    pub reply: Option<String>,
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReplyContinueResult {
    pub comment: taskboard_core::Comment,
    pub run: taskboard_core::Run,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCancel {
    pub run_display_id: String,
    pub summary: Option<String>,
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFail {
    pub run_display_id: String,
    pub summary: String,
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFinish {
    pub run_display_id: String,
    pub summary: String,
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxScope {
    pub project: Option<String>,
    pub include_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskListQuery {
    pub project: Option<String>,
    pub statuses: Vec<CardDisplayStatus>,
    pub column: Option<Column>,
    pub agent: Option<String>,
    pub blocked: bool,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OccupancyQuery {
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OccupancyRun {
    pub run_display_id: String,
    pub task_display_id: String,
    pub status: String,
    pub agent: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OccupancyGroup {
    pub worktree_path: String,
    pub runs: Vec<OccupancyRun>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunListQuery {
    pub open: bool,
    pub session_id: Option<String>,
    pub agent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActivityQuery {
    pub after: i64,
    pub project: Option<String>,
    pub task_display_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckAdd {
    pub task_display_id: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentAdd {
    pub task_display_id: String,
    pub body: String,
}

pub const STATUS_HEAD: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAction {
    Approve,
    Changes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewTask {
    pub task_display_id: String,
    pub action: ReviewAction,
    pub text: String,
    pub revision: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct StatusLine {
    pub display_id: String,
    pub status: String,
    pub agent: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct InboxCounts {
    pub total: usize,
    pub waiting: usize,
    pub failed: usize,
    pub stale: usize,
    pub review: usize,
    pub urgent: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BoardStatus {
    pub inbox: InboxCounts,
    pub inbox_head: Vec<StatusLine>,
    pub open_runs: usize,
    pub open_run_head: Vec<StatusLine>,
    pub stale: usize,
    pub stale_head: Vec<StatusLine>,
    pub ready: usize,
    pub ready_head: Vec<StatusLine>,
    pub in_review: usize,
    pub in_review_head: Vec<StatusLine>,
    pub blocked: usize,
    pub blocked_head: Vec<StatusLine>,
}
