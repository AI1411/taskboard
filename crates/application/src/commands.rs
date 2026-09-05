use taskboard_core::{Column, LinkKind};

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
