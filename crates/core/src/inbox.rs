use crate::column::Column;
use crate::display_status::CardDisplayStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InboxGroup {
    Waiting = 0,
    Failed = 1,
    Urgent = 2,
}

pub fn inbox_membership(
    column: Column,
    urgent: bool,
    status: CardDisplayStatus,
) -> Option<InboxGroup> {
    match status {
        CardDisplayStatus::Waiting => Some(InboxGroup::Waiting),
        CardDisplayStatus::Failed => Some(InboxGroup::Failed),
        _ if urgent && column != Column::Done => Some(InboxGroup::Urgent),
        _ => None,
    }
}

pub fn inbox_reason(waiting_reason: Option<&str>, run_message: Option<&str>) -> String {
    let waiting = waiting_reason.map(str::trim).filter(|s| !s.is_empty());
    let message = run_message.map(str::trim).filter(|s| !s.is_empty());
    waiting.or(message).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_is_inbox_even_in_done() {
        assert_eq!(
            inbox_membership(Column::Done, false, CardDisplayStatus::Waiting),
            Some(InboxGroup::Waiting)
        );
    }

    #[test]
    fn failed_is_inbox() {
        assert_eq!(
            inbox_membership(Column::InProgress, false, CardDisplayStatus::Failed),
            Some(InboxGroup::Failed)
        );
    }

    #[test]
    fn urgent_todo_is_inbox() {
        assert_eq!(
            inbox_membership(Column::Todo, true, CardDisplayStatus::Idle),
            Some(InboxGroup::Urgent)
        );
    }

    #[test]
    fn urgent_done_without_waiting_or_failed_is_out() {
        assert_eq!(
            inbox_membership(Column::Done, true, CardDisplayStatus::Completed),
            None
        );
        assert_eq!(
            inbox_membership(Column::Done, true, CardDisplayStatus::Idle),
            None
        );
    }

    #[test]
    fn idle_non_urgent_is_out() {
        assert_eq!(
            inbox_membership(Column::Todo, false, CardDisplayStatus::Idle),
            None
        );
    }

    #[test]
    fn completed_in_review_is_out_unless_urgent() {
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Completed),
            None
        );
        assert_eq!(
            inbox_membership(Column::InReview, true, CardDisplayStatus::Completed),
            Some(InboxGroup::Urgent)
        );
    }

    #[test]
    fn running_is_out_unless_urgent() {
        assert_eq!(
            inbox_membership(Column::InProgress, false, CardDisplayStatus::Running),
            None
        );
    }

    #[test]
    fn waiting_outranks_urgent_flag() {
        assert_eq!(
            inbox_membership(Column::Todo, true, CardDisplayStatus::Waiting),
            Some(InboxGroup::Waiting)
        );
    }

    #[test]
    fn reason_prefers_waiting_reason_then_run_message() {
        assert_eq!(
            inbox_reason(Some("Need spec"), Some("still going")),
            "Need spec"
        );
        assert_eq!(inbox_reason(None, Some("still going")), "still going");
        assert_eq!(inbox_reason(None, None), "");
        assert_eq!(inbox_reason(Some(""), Some("msg")), "msg");
    }
}
