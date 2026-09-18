use serde::Serialize;
use serde_json::Value;
use taskboard_application::AppError;
use taskboard_wire::WireError;

/// Invoke error payload. Keys match `{ "code", "message", "field", "current", "slug" }`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppErrorDto {
    pub code: String,
    pub message: String,
    pub field: Option<String>,
    pub current: Option<Value>,
    pub slug: Option<String>,
}

impl std::fmt::Display for AppErrorDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppErrorDto {}

impl From<AppError> for AppErrorDto {
    fn from(err: AppError) -> Self {
        let wire = WireError::from(&err);
        Self {
            code: wire.code,
            message: wire.message,
            field: wire.field,
            current: wire.current,
            slug: wire.slug,
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
        assert_eq!(value["slug"], serde_json::Value::Null);
    }
}
