use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Config {
    pub log_level: String,
    pub poll_interval_ms: u64,
    pub note_debounce_ms: u64,
    pub backup_retention: u32,
    pub busy_timeout_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: "info".to_owned(),
            poll_interval_ms: 1000,
            note_debounce_ms: 400,
            backup_retention: 30,
            busy_timeout_ms: 5000,
        }
    }
}

/// Reads `{data_dir}/config.toml`. A missing file uses defaults; unknown keys are ignored.
pub fn load_config(data_dir: &Path) -> Config {
    let path = data_dir.join("config.toml");
    let Ok(text) = std::fs::read_to_string(path) else {
        return Config::default();
    };
    toml::from_str(&text).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_uses_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = load_config(tmp.path());
        assert_eq!(cfg.poll_interval_ms, 1000);
        assert_eq!(cfg.note_debounce_ms, 400);
        assert_eq!(cfg.backup_retention, 30);
        assert_eq!(cfg.busy_timeout_ms, 5000);
    }

    #[test]
    fn config_ignores_unknown_keys() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("config.toml"),
            "log_level = \"debug\"\nunknown = 1\n",
        )
        .unwrap();
        let cfg = load_config(tmp.path());
        assert_eq!(cfg.log_level, "debug");
    }

    #[test]
    fn missing_config_default_log_level_is_info() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = load_config(tmp.path());
        assert_eq!(cfg.log_level, "info");
    }
}
