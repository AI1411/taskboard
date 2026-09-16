use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::run_status::RunStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardDisplayStatus {
    Idle,
    Running,
    Waiting,
    Failed,
    Completed,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("unknown display status: {0}")]
pub struct ParseCardDisplayStatusError(pub String);

impl CardDisplayStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            CardDisplayStatus::Idle => "idle",
            CardDisplayStatus::Running => "running",
            CardDisplayStatus::Waiting => "waiting",
            CardDisplayStatus::Failed => "failed",
            CardDisplayStatus::Completed => "completed",
        }
    }
}

impl FromStr for CardDisplayStatus {
    type Err = ParseCardDisplayStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "idle" => Ok(CardDisplayStatus::Idle),
            "running" => Ok(CardDisplayStatus::Running),
            "waiting" => Ok(CardDisplayStatus::Waiting),
            "failed" => Ok(CardDisplayStatus::Failed),
            "completed" => Ok(CardDisplayStatus::Completed),
            other => Err(ParseCardDisplayStatusError(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStatusView {
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    pub display_id: String,
}

pub fn winning_run_view(runs: &[RunStatusView]) -> Option<&RunStatusView> {
    if runs.is_empty() {
        return None;
    }
    let has_active = runs
        .iter()
        .any(|run| matches!(run.status, RunStatus::Running | RunStatus::Waiting));
    runs.iter()
        .filter(|run| !has_active || matches!(run.status, RunStatus::Running | RunStatus::Waiting))
        .max_by(|left, right| {
            left.started_at
                .cmp(&right.started_at)
                .then_with(|| left.display_id.cmp(&right.display_id))
        })
}

pub fn card_display_status(runs: &[RunStatusView]) -> CardDisplayStatus {
    match winning_run_view(runs) {
        None => CardDisplayStatus::Idle,
        Some(best) => match best.status {
            RunStatus::Running => CardDisplayStatus::Running,
            RunStatus::Waiting => CardDisplayStatus::Waiting,
            RunStatus::Failed => CardDisplayStatus::Failed,
            RunStatus::Completed => CardDisplayStatus::Completed,
        },
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

    #[test]
    fn display_status_from_str_round_trips() {
        for status in [
            CardDisplayStatus::Idle,
            CardDisplayStatus::Running,
            CardDisplayStatus::Waiting,
            CardDisplayStatus::Failed,
            CardDisplayStatus::Completed,
        ] {
            assert_eq!(
                status.as_str().parse::<CardDisplayStatus>().unwrap(),
                status
            );
        }
        assert!("active".parse::<CardDisplayStatus>().is_err());
    }

    #[test]
    fn winning_run_view_picks_newest_active_then_terminal() {
        let started = Utc.with_ymd_and_hms(2026, 9, 5, 10, 0, 0).unwrap();
        let later = Utc.with_ymd_and_hms(2026, 9, 5, 11, 0, 0).unwrap();
        let runs = [
            RunStatusView {
                status: RunStatus::Completed,
                started_at: later,
                display_id: "RUN-1".into(),
            },
            RunStatusView {
                status: RunStatus::Waiting,
                started_at: started,
                display_id: "RUN-2".into(),
            },
        ];
        assert_eq!(
            winning_run_view(&runs).map(|run| run.display_id.as_str()),
            Some("RUN-2")
        );
        assert_eq!(winning_run_view(&[]), None);
    }
}
