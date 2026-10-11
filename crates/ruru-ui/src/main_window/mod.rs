use ruru_types::ProviderId;

mod navigation;
mod settings;
mod state;
mod style;
mod update;
mod view;

pub use settings::Input as SettingsInput;
pub use settings::{ProviderView, Section};
pub use state::{MainWindow, WindowMode};

#[derive(Debug, Clone)]
pub enum Input {
    Show,
    DismissRequested,
    SettingsRequested,
    BackRequested,
    Settings(settings::Input),
}

#[derive(Clone, PartialEq, Eq)]
pub enum Output {
    Dismissed,
    JumpToSection(Section),
    DiscoverModels(ProviderId),
    SaveKey(ProviderId, String),
    RemoveKey(ProviderId),
    SetModel {
        provider: ProviderId,
        model: String,
        enabled: bool,
    },
}

impl Default for MainWindow {
    fn default() -> Self {
        Self {
            visible: true,
            mode: WindowMode::default(),
            settings: settings::Settings::default(),
        }
    }
}

impl MainWindow {
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn mode(&self) -> WindowMode {
        self.mode
    }

    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_providers(mut self, providers: Vec<ProviderView>) -> Self {
        self.settings = self.settings.with_providers(providers);
        self
    }
}
