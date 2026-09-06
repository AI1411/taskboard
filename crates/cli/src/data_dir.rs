use std::path::{Path, PathBuf};

use taskboard_store_sqlite::resolve_data_dir;

/// `--data-dir`, then `TASKBOARD_DATA_DIR`, then the platform default.
pub fn resolve(cli_override: Option<&Path>) -> PathBuf {
    resolve_data_dir(
        cli_override,
        std::env::var_os("TASKBOARD_DATA_DIR").as_deref(),
    )
}
