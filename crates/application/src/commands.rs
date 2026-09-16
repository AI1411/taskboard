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
pub struct TaskUpdate {
    pub display_id: String,
    pub title: Option<String>,
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
