use chrono::{DateTime, SecondsFormat, Utc};

use crate::column::Column;
use crate::display_status::CardDisplayStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InboxGroup {
    Waiting = 0,
    Failed = 1,
    Stale = 2,
    Review = 3,
    Urgent = 4,
}

pub fn inbox_membership(
    column: Column,
    urgent: bool,
    status: CardDisplayStatus,
    stale: bool,
) -> Option<InboxGroup> {
    match status {
        CardDisplayStatus::Waiting => Some(InboxGroup::Waiting),
        CardDisplayStatus::Failed => Some(InboxGroup::Failed),
        _ if stale => Some(InboxGroup::Stale),
        _ if column == Column::InReview && status != CardDisplayStatus::Running => {
            Some(InboxGroup::Review)
        }
        _ if urgent && column != Column::Done => Some(InboxGroup::Urgent),
        _ => None,
    }
}

pub fn inbox_reason(waiting_reason: Option<&str>, run_message: Option<&str>) -> String {
    let waiting = waiting_reason.map(str::trim).filter(|s| !s.is_empty());
    let message = run_message.map(str::trim).filter(|s| !s.is_empty());
    waiting.or(message).unwrap_or("").to_string()
}

pub fn inbox_stale_reason(updated_at: DateTime<Utc>) -> String {
    format!(
        "stale · last update {}",
        updated_at.to_rfc3339_opts(SecondsFormat::Secs, true)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_is_inbox_even_in_done() {
        assert_eq!(
            inbox_membership(Column::Done, false, CardDisplayStatus::Waiting, false),
            Some(InboxGroup::Waiting)
        );
    }

    #[test]
    fn failed_is_inbox() {
        assert_eq!(
            inbox_membership(Column::InProgress, false, CardDisplayStatus::Failed, false),
            Some(InboxGroup::Failed)
        );
    }

    #[test]
    fn urgent_todo_is_inbox() {
        assert_eq!(
            inbox_membership(Column::Todo, true, CardDisplayStatus::Idle, false),
            Some(InboxGroup::Urgent)
        );
    }

    #[test]
    fn urgent_done_without_waiting_or_failed_is_out() {
        assert_eq!(
            inbox_membership(Column::Done, true, CardDisplayStatus::Completed, false),
            None
        );
        assert_eq!(
            inbox_membership(Column::Done, true, CardDisplayStatus::Idle, false),
            None
        );
    }

    #[test]
    fn idle_non_urgent_is_out() {
        assert_eq!(
            inbox_membership(Column::Todo, false, CardDisplayStatus::Idle, false),
            None
        );
    }

    #[test]
    fn completed_in_review_is_review_even_if_urgent() {
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Completed, false),
            Some(InboxGroup::Review)
        );
        assert_eq!(
            inbox_membership(Column::InReview, true, CardDisplayStatus::Completed, false),
            Some(InboxGroup::Review)
        );
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Idle, false),
            Some(InboxGroup::Review)
        );
    }

    #[test]
    fn running_waiting_failed_stale_in_review_are_not_review() {
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Running, false),
            None
        );
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Waiting, false),
            Some(InboxGroup::Waiting)
        );
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Failed, false),
            Some(InboxGroup::Failed)
        );
        assert_eq!(
            inbox_membership(Column::InReview, false, CardDisplayStatus::Running, true),
            Some(InboxGroup::Stale)
        );
        assert!(InboxGroup::Stale < InboxGroup::Review);
        assert!(InboxGroup::Review < InboxGroup::Urgent);
    }

    #[test]
    fn done_completed_is_still_out() {
        assert_eq!(
            inbox_membership(Column::Done, false, CardDisplayStatus::Completed, false),
            None
        );
    }

    #[test]
    fn running_is_out_unless_urgent() {
        assert_eq!(
            inbox_membership(Column::InProgress, false, CardDisplayStatus::Running, false),
            None
        );
    }

    #[test]
    fn stale_running_is_inbox_between_failed_and_urgent() {
        assert_eq!(
            inbox_membership(Column::InProgress, false, CardDisplayStatus::Running, true),
            Some(InboxGroup::Stale)
        );
        assert_eq!(
            inbox_membership(Column::InProgress, true, CardDisplayStatus::Running, true),
            Some(InboxGroup::Stale)
        );
        assert!(InboxGroup::Failed < InboxGroup::Stale);
        assert!(InboxGroup::Stale < InboxGroup::Urgent);
    }

    #[test]
    fn stale_reason_includes_last_update() {
        let at = chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 9, 16, 12, 0, 0).unwrap();
        assert_eq!(
            inbox_stale_reason(at),
            "stale · last update 2026-09-16T12:00:00Z"
        );
    }

    #[test]
    fn waiting_outranks_urgent_flag() {
        assert_eq!(
            inbox_membership(Column::Todo, true, CardDisplayStatus::Waiting, false),
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
