use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Waiting,
    Failed,
    Completed,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseRunStatusError {
    #[error("unknown run status: {0}")]
    Unknown(String),
}

impl RunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            RunStatus::Running => "running",
            RunStatus::Waiting => "waiting",
            RunStatus::Failed => "failed",
            RunStatus::Completed => "completed",
        }
    }
}

impl FromStr for RunStatus {
    type Err = ParseRunStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(RunStatus::Running),
            "waiting" => Ok(RunStatus::Waiting),
            "failed" => Ok(RunStatus::Failed),
            "completed" => Ok(RunStatus::Completed),
            other => Err(ParseRunStatusError::Unknown(other.to_string())),
        }
    }
}
