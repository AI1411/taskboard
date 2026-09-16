use std::path::PathBuf;

use taskboard_application::App;

pub struct DesktopState {
    pub app: tokio::sync::Mutex<App>,
    pub data_dir: PathBuf,
}
