use crate::error::{FieldError, ValidationError};

const TITLE_FIELD: &str = "title";
const NAME_FIELD: &str = "name";
const AGENT_FIELD: &str = "agent";
const VALUE_FIELD: &str = "value";

const AGENT_ALLOWED: &str = "[a-z0-9][a-z0-9._-]{0,63}";
const URL_SCHEMES_ALLOWED: &str = "http, https, or file";

pub fn trim_title(raw: &str) -> Result<String, ValidationError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ValidationError::new(TITLE_FIELD, FieldError::Empty));
    }
    if trimmed.len() > 200 {
        return Err(ValidationError::new(
            TITLE_FIELD,
            FieldError::TooLong { max: 200 },
        ));
    }
    Ok(trimmed.to_string())
}

pub fn trim_project_name(raw: &str) -> Result<String, ValidationError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ValidationError::new(NAME_FIELD, FieldError::Empty));
    }
    if trimmed.len() > 120 {
        return Err(ValidationError::new(
            NAME_FIELD,
            FieldError::TooLong { max: 120 },
        ));
    }
    Ok(trimmed.to_string())
}

pub fn parse_agent(raw: &str) -> Result<String, ValidationError> {
    if raw.is_empty() || raw.len() > 64 {
        return Err(ValidationError::new(
            AGENT_FIELD,
            FieldError::Invalid { allowed: AGENT_ALLOWED },
        ));
    }

    let mut chars = raw.chars();
    let first = chars.next().expect("non-empty");
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(ValidationError::new(
            AGENT_FIELD,
            FieldError::Invalid { allowed: AGENT_ALLOWED },
        ));
    }

    for ch in chars {
        if !ch.is_ascii_lowercase()
            && !ch.is_ascii_digit()
            && ch != '.'
            && ch != '_'
            && ch != '-'
        {
            return Err(ValidationError::new(
                AGENT_FIELD,
                FieldError::Invalid { allowed: AGENT_ALLOWED },
            ));
        }
    }

    Ok(raw.to_string())
}

pub fn parse_url(raw: &str) -> Result<String, ValidationError> {
    let scheme = raw
        .split("://")
        .next()
        .filter(|_| raw.contains("://"))
        .ok_or_else(|| {
            ValidationError::new(
                VALUE_FIELD,
                FieldError::Invalid {
                    allowed: URL_SCHEMES_ALLOWED,
                },
            )
        })?;

    match scheme {
        "http" | "https" | "file" => Ok(raw.to_string()),
        _ => Err(ValidationError::new(
            VALUE_FIELD,
            FieldError::Invalid {
                allowed: URL_SCHEMES_ALLOWED,
            },
        )),
    }
}

pub fn parse_path_link(raw: &str) -> Result<String, ValidationError> {
    if raw.is_empty() {
        return Err(ValidationError::new(VALUE_FIELD, FieldError::Empty));
    }
    Ok(raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::{parse_agent, parse_url, trim_title};

    #[test]
    fn title_must_not_be_blank() {
        let err = trim_title("   ").unwrap_err();
        assert_eq!(err.field, "title");
    }

    #[test]
    fn title_max_200() {
        assert!(trim_title(&"a".repeat(200)).is_ok());
        assert!(trim_title(&"a".repeat(201)).is_err());
    }

    #[test]
    fn agent_pattern() {
        assert!(parse_agent("codex").is_ok());
        assert!(parse_agent("Claude").is_err());
        assert!(parse_agent("-bad").is_err());
    }

    #[test]
    fn url_schemes() {
        assert!(parse_url("https://example.com").is_ok());
        assert!(parse_url("ftp://x").is_err());
    }
}
