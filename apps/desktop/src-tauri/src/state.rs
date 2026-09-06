use taskboard_application::App;

pub struct DesktopState {
    pub app: tokio::sync::Mutex<App>,
}
