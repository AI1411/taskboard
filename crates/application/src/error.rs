use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum AppError {
    #[error("{message}")]
    Validation { field: String, message: String },
    #[error("{entity} {id} not found")]
    NotFound { entity: String, id: String },
    #[error("slug `{slug}` already exists")]
    DuplicateSlug { slug: String },
    #[error("revision conflict")]
    RevisionConflict { current: serde_json::Value },
    #[error("cannot reorder across columns {left} and {right}")]
    DifferentColumn { left: String, right: String },
    #[error("undo conflict")]
    UndoConflict { current: serde_json::Value },
    #[error("database busy")]
    DatabaseBusy,
    #[error("{0}")]
    Io(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Validation { .. } => "validation_error",
            AppError::NotFound { .. } => "not_found",
            AppError::DuplicateSlug { .. } => "duplicate_slug",
            AppError::RevisionConflict { .. } => "revision_conflict",
            AppError::DifferentColumn { .. } => "different_column",
            AppError::UndoConflict { .. } => "undo_conflict",
            AppError::DatabaseBusy => "database_busy",
            AppError::Io(_) => "io_error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn revision_conflict_code() {
        let err = AppError::RevisionConflict {
            current: serde_json::json!({}),
        };
        assert_eq!(err.code(), "revision_conflict");
    }

    #[test]
    fn validation_code() {
        let err = AppError::Validation {
            field: "title".into(),
            message: "must not be empty".into(),
        };
        assert_eq!(err.code(), "validation_error");
    }

    #[test]
    fn not_found_code() {
        let err = AppError::NotFound {
            entity: "task".into(),
            id: "TASK-1".into(),
        };
        assert_eq!(err.code(), "not_found");
    }

    #[test]
    fn duplicate_slug_code() {
        let err = AppError::DuplicateSlug {
            slug: "renai-sim".into(),
        };
        assert_eq!(err.code(), "duplicate_slug");
    }

    #[test]
    fn different_column_code() {
        let err = AppError::DifferentColumn {
            left: "todo".into(),
            right: "done".into(),
        };
        assert_eq!(err.code(), "different_column");
    }

    #[test]
    fn undo_conflict_code() {
        let err = AppError::UndoConflict {
            current: serde_json::json!({}),
        };
        assert_eq!(err.code(), "undo_conflict");
    }

    #[test]
    fn database_busy_code() {
        assert_eq!(AppError::DatabaseBusy.code(), "database_busy");
    }

    #[test]
    fn io_code() {
        let err = AppError::Io("disk full".into());
        assert_eq!(err.code(), "io_error");
    }
}
