use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayKind {
    Task,
    Run,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseDisplayIdError {
    #[error("invalid display id format")]
    InvalidFormat,
    #[error("invalid display id number")]
    InvalidNumber,
}

pub fn display_id(kind: DisplayKind, n: i64) -> String {
    let prefix = match kind {
        DisplayKind::Task => "TASK",
        DisplayKind::Run => "RUN",
    };
    format!("{prefix}-{n}")
}

pub fn parse_display_id(raw: &str) -> Result<(DisplayKind, i64), ParseDisplayIdError> {
    let (prefix, number) = raw
        .split_once('-')
        .ok_or(ParseDisplayIdError::InvalidFormat)?;

    let kind = match prefix {
        "TASK" => DisplayKind::Task,
        "RUN" => DisplayKind::Run,
        _ => return Err(ParseDisplayIdError::InvalidFormat),
    };

    let n = number
        .parse::<i64>()
        .map_err(|_| ParseDisplayIdError::InvalidNumber)?;

    Ok((kind, n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_ids_round_trip() {
        assert_eq!(display_id(DisplayKind::Task, 142), "TASK-142");
        assert_eq!(parse_display_id("RUN-37").unwrap(), (DisplayKind::Run, 37));
    }
}
