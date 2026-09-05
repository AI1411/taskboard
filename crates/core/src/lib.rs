pub fn workspace_name() -> &'static str {
    "taskboard"
}

mod column;
mod display_status;
mod ids;
mod run_status;
mod slug;

pub use column::{Column, ParseColumnError};
pub use display_status::{CardDisplayStatus, RunStatusView, card_display_status};
pub use ids::{DisplayKind, ParseDisplayIdError, display_id, parse_display_id};
pub use run_status::{ParseRunStatusError, RunStatus};
pub use slug::{SlugError, next_unique_slug, slugify};

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_name_is_taskboard() {
        assert_eq!(super::workspace_name(), "taskboard");
    }
}
