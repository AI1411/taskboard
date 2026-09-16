use std::path::Path;

use serde::Deserialize;

use taskboard_application::AppError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct UiState {
    pub last_project_slug: Option<String>,
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
    let text = match state.last_project_slug.as_deref() {
        Some(slug) if !slug.is_empty() => {
            format!("last_project_slug = \"{}\"\n", slug.replace('"', "\\\""))
        }
        _ => String::new(),
    };
    std::fs::write(data_dir.join("ui.toml"), text).map_err(|err| AppError::Io(err.to_string()))
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
}
