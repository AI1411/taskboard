use std::path::Path;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use taskboard_application::{App, AppError};

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct UiState {
    pub last_project_slug: Option<String>,
    pub last_purge_at: Option<String>,
}

/// Reads `{data_dir}/ui.toml`. A missing file uses defaults; unknown keys are ignored.
pub fn load_ui_state(data_dir: &Path) -> UiState {
    let path = data_dir.join("ui.toml");
    let Ok(text) = std::fs::read_to_string(path) else {
        return UiState::default();
    };
    let mut state: UiState = toml::from_str(&text).unwrap_or_default();
    if state
        .last_project_slug
        .as_deref()
        .is_some_and(str::is_empty)
    {
        state.last_project_slug = None;
    }
    state
}

/// Writes `{data_dir}/ui.toml`. Omits `last_project_slug` when `None`.
pub fn save_ui_state(data_dir: &Path, state: &UiState) -> Result<(), AppError> {
    std::fs::create_dir_all(data_dir).map_err(|err| AppError::Io(err.to_string()))?;
    let mut text = String::new();
    if let Some(slug) = state
        .last_project_slug
        .as_deref()
        .filter(|slug| !slug.is_empty())
    {
        text.push_str(&format!(
            "last_project_slug = \"{}\"\n",
            slug.replace('"', "\\\"")
        ));
    }
    if let Some(at) = state.last_purge_at.as_deref().filter(|at| !at.is_empty()) {
        text.push_str(&format!(
            "last_purge_at = \"{}\"\n",
            at.replace('"', "\\\"")
        ));
    }
    std::fs::write(data_dir.join("ui.toml"), text).map_err(|err| AppError::Io(err.to_string()))
}

pub fn purge_is_due(data_dir: &Path, now: DateTime<Utc>) -> bool {
    let Some(raw) = load_ui_state(data_dir).last_purge_at else {
        return true;
    };
    let Ok(last) = DateTime::parse_from_rfc3339(&raw) else {
        return true;
    };
    now.signed_duration_since(last.with_timezone(&Utc)) >= chrono::Duration::hours(1)
}

pub fn record_purge(data_dir: &Path, now: DateTime<Utc>) -> Result<(), AppError> {
    let mut state = load_ui_state(data_dir);
    state.last_purge_at = Some(now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    save_ui_state(data_dir, &state)
}

pub async fn purge_expired_if_due(
    app: &App,
    data_dir: &Path,
    now: DateTime<Utc>,
) -> Result<(), AppError> {
    if !purge_is_due(data_dir, now) {
        return Ok(());
    }
    app.purge_expired_trash(now).await?;
    record_purge(data_dir, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_ui_toml_is_empty_slug() {
        let tmp = tempfile::tempdir().unwrap();
        let state = load_ui_state(tmp.path());
        assert_eq!(state.last_project_slug, None);
    }

    #[test]
    fn save_and_load_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        save_ui_state(
            tmp.path(),
            &UiState {
                last_project_slug: Some("taskboard".into()),
                last_purge_at: None,
            },
        )
        .unwrap();
        let state = load_ui_state(tmp.path());
        assert_eq!(state.last_project_slug.as_deref(), Some("taskboard"));
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("ui.toml"),
            "last_project_slug = \"alpha\"\nunknown = 1\n",
        )
        .unwrap();
        let state = load_ui_state(tmp.path());
        assert_eq!(state.last_project_slug.as_deref(), Some("alpha"));
    }

    #[test]
    fn purge_is_due_until_an_hour_after_the_stamp() {
        let tmp = tempfile::tempdir().unwrap();
        let now = Utc::now();
        assert!(purge_is_due(tmp.path(), now));
        record_purge(tmp.path(), now).unwrap();
        assert!(!purge_is_due(tmp.path(), now));
        assert!(purge_is_due(tmp.path(), now + chrono::Duration::hours(1)));
        let state = load_ui_state(tmp.path());
        assert_eq!(state.last_project_slug, None);
    }

    #[test]
    fn record_purge_keeps_the_project_slug() {
        let tmp = tempfile::tempdir().unwrap();
        save_ui_state(
            tmp.path(),
            &UiState {
                last_project_slug: Some("taskboard".into()),
                last_purge_at: None,
            },
        )
        .unwrap();
        record_purge(tmp.path(), Utc::now()).unwrap();
        let state = load_ui_state(tmp.path());
        assert_eq!(state.last_project_slug.as_deref(), Some("taskboard"));
        assert!(state.last_purge_at.is_some());
    }
}
