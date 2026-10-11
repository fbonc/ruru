use super::settings;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WindowMode {
    #[default]
    Home,
    Settings,
}

pub struct MainWindow {
    pub(super) visible: bool,
    pub(super) mode: WindowMode,
    pub(super) settings: settings::Settings,
}
