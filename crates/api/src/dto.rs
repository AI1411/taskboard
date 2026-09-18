use serde::{Deserialize, Deserializer};
use taskboard_core::{Column, LinkKind};

#[allow(unused_imports)]
pub use taskboard_wire::{
    json_keys_to_camel, ActivityDto, BoardStatusDto, CheckDto, CommentDto, InboxCountsDto,
    InboxItemDto, LinkDto, OccupancyGroupDto, OccupancyRunDto, ProjectDto, RunDto, StatusLineDto,
    SyncDeltaDto, TaskDetailDto, TaskSummaryDto, TrashDto, UiStateDto, UndoResultDto,
};

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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchTaskBody {
    pub title: Option<String>,
    pub note_markdown: Option<String>,
    pub urgent: Option<bool>,
    pub column: Option<Column>,
    /// Absent: do not reorder. `null`: move to end. String: place before that card.
    #[serde(default, deserialize_with = "deserialize_present_option")]
    pub before_display_id: Option<Option<String>>,
    pub worktree_path: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddLinkBody {
    pub kind: LinkKind,
    pub value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCommentBody {
    pub body: String,
    #[serde(default, rename = "continue")]
    pub continue_waiting: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCheckBody {
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewBody {
    pub action: ReviewActionDto,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewActionDto {
    Approve,
    Changes,
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
    Cancel,
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

#[derive(Debug, Deserialize)]
pub struct InboxQuery {
    pub project: Option<String>,
    pub archived: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    pub project: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OccupancyQueryDto {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnBody {
    pub titles: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchUiStateBody {
    pub last_project_slug: Option<String>,
}

pub(crate) fn empty_to_none(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn deserialize_present_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Some(Option::<T>::deserialize(deserializer)?))
}
