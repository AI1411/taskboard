use serde::{Deserialize, Deserializer};

#[allow(unused_imports)]
pub use taskboard_wire::{
    json_keys_to_camel, ActivityDto, BoardStatusDto, CheckDto, CommentDto, InboxCountsDto,
    InboxItemDto, LinkDto, OccupancyGroupDto, OccupancyRunDto, ProjectDto, RunDto, StatusLineDto,
    SyncDeltaDto, TaskDetailDto, TaskSummaryDto, TrashDto, UiStateDto, UndoResultDto,
};

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
