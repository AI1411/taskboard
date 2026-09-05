pub fn workspace_name() -> &'static str {
    "taskboard"
}

mod column;
mod display_status;
mod error;
mod ids;
mod models;
mod run_status;
mod slug;
mod validation;

pub use column::{Column, ParseColumnError};
pub use display_status::{CardDisplayStatus, RunStatusView, card_display_status};
pub use error::{FieldError, ValidationError};
pub use ids::{DisplayKind, ParseDisplayIdError, display_id, parse_display_id};
pub use models::{
    Activity, ActorKind, EntityType, Link, LinkKind, Project, Run, Task, TaskDetail, TaskSummary,
};
pub use run_status::{ParseRunStatusError, RunStatus};
pub use slug::{SlugError, next_unique_slug, slugify};
pub use validation::{parse_agent, parse_path_link, parse_url, trim_project_name, trim_title};

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_name_is_taskboard() {
        assert_eq!(super::workspace_name(), "taskboard");
    }
}
