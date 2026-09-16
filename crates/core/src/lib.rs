pub fn workspace_name() -> &'static str {
    "taskboard"
}

mod column;
mod display_status;
mod error;
mod ids;
mod inbox;
mod models;
mod order;
mod run_status;
mod slug;
mod validation;

pub use column::{Column, ParseColumnError};
pub use display_status::{
    card_display_status, winning_run_view, CardDisplayStatus, ParseCardDisplayStatusError,
    RunStatusView,
};
pub use error::{FieldError, ValidationError};
pub use ids::{display_id, parse_display_id, DisplayKind, ParseDisplayIdError};
pub use inbox::{inbox_membership, inbox_reason, inbox_stale_reason, InboxGroup};
pub use models::{
    Activity, ActivityEntry, ActorKind, Check, Comment, EntityType, InboxItem, Link, LinkKind,
    Project, Run, Task, TaskDetail, TaskSummary,
};
pub use order::{place_before, place_urgent, rewrite_positions, sort_column, OrderError, OrderKey};
pub use run_status::{ParseRunStatusError, RunStatus};
pub use slug::{next_unique_slug, slugify, SlugError};
pub use validation::{parse_agent, parse_path_link, parse_url, trim_project_name, trim_title};

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_name_is_taskboard() {
        assert_eq!(super::workspace_name(), "taskboard");
    }
}
