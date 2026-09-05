use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub fn default_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        return home_dir().join("Library/Application Support/Taskboard");
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("taskboard");
            }
        }
        home_dir().join(".local/share/taskboard")
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// CLI override wins, then `TASKBOARD_DATA_DIR`, then the platform default.
pub fn resolve_data_dir(cli_override: Option<&Path>, env: Option<&OsStr>) -> PathBuf {
    if let Some(cli) = cli_override {
        return cli.to_path_buf();
    }
    if let Some(env_dir) = env {
        return PathBuf::from(env_dir);
    }
    default_data_dir()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::path::Path;

    use super::*;

    #[test]
    fn cli_override_wins() {
        let dir = resolve_data_dir(
            Some(Path::new("/tmp/tb-cli")),
            Some(OsStr::new("/tmp/tb-env")),
        );
        assert_eq!(dir, Path::new("/tmp/tb-cli"));
    }

    #[test]
    fn env_override_used_when_cli_absent() {
        let dir = resolve_data_dir(None, Some(OsStr::new("/tmp/tb-env")));
        assert_eq!(dir, Path::new("/tmp/tb-env"));
    }

    #[test]
    fn default_used_when_cli_and_env_absent() {
        let dir = resolve_data_dir(None, None);
        assert_eq!(dir, default_data_dir());
    }

    #[test]
    fn default_data_dir_linux_uses_xdg_or_local_share() {
        let dir = default_data_dir();
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
            assert_eq!(dir, PathBuf::from(xdg).join("taskboard"));
        } else {
            assert_eq!(dir, home_dir().join(".local/share/taskboard"));
        }
    }
}
