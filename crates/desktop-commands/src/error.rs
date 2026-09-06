use serde::Serialize;
use serde_json::Value;
use taskboard_application::AppError;

use crate::dto::json_keys_to_camel;

/// Invoke error payload. Keys match `{ "code", "message", "field", "current" }`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppErrorDto {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub current: Option<Value>,
}

impl std::fmt::Display for AppErrorDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppErrorDto {}

impl From<AppError> for AppErrorDto {
    fn from(err: AppError) -> Self {
        let code = err.code().to_string();
        let message = err.to_string();
        let (field, current) = match err {
            AppError::Validation { field, .. } => (Some(field), None),
            AppError::RevisionConflict { current } | AppError::UndoConflict { current } => {
                (None, Some(json_keys_to_camel(current)))
            }
            _ => (None, None),
        };
        Self {
            code,
            message,
            field,
            current,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppErrorDto;
    use taskboard_application::AppError;

    #[test]
    fn app_error_dto_serializes_brief_shape() {
        let err = AppErrorDto::from(AppError::Validation {
            field: "title".into(),
            message: "must not be empty".into(),
        });
        let value = serde_json::to_value(&err).unwrap();
        assert_eq!(value["code"], "validation_error");
        assert_eq!(value["message"], "must not be empty");
        assert_eq!(value["field"], "title");
        assert_eq!(value["current"], serde_json::Value::Null);
    }
}
