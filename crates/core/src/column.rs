use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Column {
    Todo,
    InProgress,
    InReview,
    Done,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseColumnError {
    #[error("unknown column: {0}")]
    Unknown(String),
}

impl Column {
    pub fn as_str(self) -> &'static str {
        match self {
            Column::Todo => "todo",
            Column::InProgress => "in-progress",
            Column::InReview => "in-review",
            Column::Done => "done",
        }
    }
}

impl FromStr for Column {
    type Err = ParseColumnError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "todo" => Ok(Column::Todo),
            "in-progress" => Ok(Column::InProgress),
            "in-review" => Ok(Column::InReview),
            "done" => Ok(Column::Done),
            other => Err(ParseColumnError::Unknown(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_from_str_rejects_unknown() {
        assert!("doing".parse::<Column>().is_err());
        assert_eq!("in-progress".parse::<Column>().unwrap(), Column::InProgress);
    }
}
