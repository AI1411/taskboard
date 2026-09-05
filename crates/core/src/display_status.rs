use chrono::{DateTime, Utc};

use crate::run_status::RunStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardDisplayStatus {
    Idle,
    Running,
    Waiting,
    Failed,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStatusView {
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    pub display_id: String,
}

pub fn card_display_status(runs: &[RunStatusView]) -> CardDisplayStatus {
    if runs.is_empty() {
        return CardDisplayStatus::Idle;
    }

    let has_active = runs
        .iter()
        .any(|r| matches!(r.status, RunStatus::Running | RunStatus::Waiting));

    let best = runs
        .iter()
        .filter(|r| {
            !has_active || matches!(r.status, RunStatus::Running | RunStatus::Waiting)
        })
        .max_by(|a, b| {
            a.started_at
                .cmp(&b.started_at)
                .then_with(|| a.display_id.cmp(&b.display_id))
        })
        .expect("non-empty iterator");

    match best.status {
        RunStatus::Running => CardDisplayStatus::Running,
        RunStatus::Waiting => CardDisplayStatus::Waiting,
        RunStatus::Failed => CardDisplayStatus::Failed,
        RunStatus::Completed => CardDisplayStatus::Completed,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::run_status::RunStatus;

    #[test]
    fn idle_when_no_runs() {
        assert_eq!(card_display_status(&[]), CardDisplayStatus::Idle);
    }

    #[test]
    fn newest_running_wins_over_completed() {
        let runs = [
            RunStatusView {
                status: RunStatus::Completed,
                started_at: Utc.with_ymd_and_hms(2026, 9, 5, 10, 0, 0).unwrap(),
                display_id: "RUN-1".into(),
            },
            RunStatusView {
                status: RunStatus::Running,
                started_at: Utc.with_ymd_and_hms(2026, 9, 5, 11, 0, 0).unwrap(),
                display_id: "RUN-2".into(),
            },
        ];
        assert_eq!(card_display_status(&runs), CardDisplayStatus::Running);
    }

    #[test]
    fn waiting_in_active_set_wins_over_completed_and_failed() {
        let runs = [
            RunStatusView {
                status: RunStatus::Completed,
                started_at: Utc.with_ymd_and_hms(2026, 9, 5, 12, 0, 0).unwrap(),
                display_id: "RUN-1".into(),
            },
            RunStatusView {
                status: RunStatus::Failed,
                started_at: Utc.with_ymd_and_hms(2026, 9, 5, 13, 0, 0).unwrap(),
                display_id: "RUN-2".into(),
            },
            RunStatusView {
                status: RunStatus::Waiting,
                started_at: Utc.with_ymd_and_hms(2026, 9, 5, 11, 0, 0).unwrap(),
                display_id: "RUN-3".into(),
            },
        ];
        assert_eq!(card_display_status(&runs), CardDisplayStatus::Waiting);
    }

    #[test]
    fn newest_terminal_run_shown_when_no_active_runs() {
        let runs = [
            RunStatusView {
                status: RunStatus::Completed,
                started_at: Utc.with_ymd_and_hms(2026, 9, 5, 10, 0, 0).unwrap(),
                display_id: "RUN-1".into(),
            },
            RunStatusView {
                status: RunStatus::Failed,
                started_at: Utc.with_ymd_and_hms(2026, 9, 5, 11, 0, 0).unwrap(),
                display_id: "RUN-2".into(),
            },
        ];
        assert_eq!(card_display_status(&runs), CardDisplayStatus::Failed);
    }

    #[test]
    fn equal_started_at_tie_breaks_on_lexicographic_max_display_id() {
        let started_at = Utc.with_ymd_and_hms(2026, 9, 5, 10, 0, 0).unwrap();
        let runs = [
            RunStatusView {
                status: RunStatus::Completed,
                started_at,
                display_id: "RUN-1".into(),
            },
            RunStatusView {
                status: RunStatus::Failed,
                started_at,
                display_id: "RUN-9".into(),
            },
        ];
        assert_eq!(card_display_status(&runs), CardDisplayStatus::Failed);
    }
}
