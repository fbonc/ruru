use iced::widget::Id;
use ruru_types::ProviderId;

mod navigation;
mod state;
mod update;
mod view;

pub use state::ProviderView;
pub(super) use state::Settings;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Section {
    #[default]
    Providers,
}

#[derive(Clone)]
pub enum Input {
    SectionSelected(Section),
    ProviderSelected(ProviderId),
    ProviderRowToggled(ProviderId),
    ApiKeyChanged(ProviderId, String),
    SaveKeyRequested(ProviderId),
    RemoveKeyRequested(ProviderId),
    RefreshModelsRequested(ProviderId),
    ModelToggled(ProviderId, String, bool),
    KeySaved(ProviderId),
    KeyRemoved(ProviderId),
    SelectedModelsChanged(ProviderId, Vec<String>),
    DiscoveryStarted(ProviderId),
    DiscoveryFinished(ProviderId, Vec<String>),
    ProviderFailed(ProviderId, String),
    Failed(String),
    ClearError,
    DiscardDraft,
}

impl std::fmt::Debug for Input {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeySaved(id) => formatter.debug_tuple("KeySaved").field(id).finish(),
            Self::KeyRemoved(id) => formatter.debug_tuple("KeyRemoved").field(id).finish(),
            Self::SelectedModelsChanged(id, models) => formatter
                .debug_tuple("SelectedModelsChanged")
                .field(id)
                .field(models)
                .finish(),
            Self::DiscoveryStarted(id) => {
                formatter.debug_tuple("DiscoveryStarted").field(id).finish()
            }
            Self::DiscoveryFinished(id, models) => formatter
                .debug_tuple("DiscoveryFinished")
                .field(id)
                .field(models)
                .finish(),
            Self::ProviderFailed(id, error) => formatter
                .debug_tuple("ProviderFailed")
                .field(id)
                .field(error)
                .finish(),
            Self::Failed(error) => formatter.debug_tuple("Failed").field(error).finish(),
            Self::ClearError => formatter.write_str("ClearError"),
            Self::DiscardDraft => formatter.write_str("DiscardDraft"),
            Self::SectionSelected(section) => formatter
                .debug_tuple("SectionSelected")
                .field(section)
                .finish(),
            Self::ProviderSelected(id) => {
                formatter.debug_tuple("ProviderSelected").field(id).finish()
            }
            Self::ProviderRowToggled(id) => formatter
                .debug_tuple("ProviderRowToggled")
                .field(id)
                .finish(),
            Self::ApiKeyChanged(id, _) => formatter
                .debug_tuple("ApiKeyChanged")
                .field(id)
                .field(&"[redacted]")
                .finish(),
            Self::SaveKeyRequested(id) => {
                formatter.debug_tuple("SaveKeyRequested").field(id).finish()
            }
            Self::RemoveKeyRequested(id) => formatter
                .debug_tuple("RemoveKeyRequested")
                .field(id)
                .finish(),
            Self::RefreshModelsRequested(id) => formatter
                .debug_tuple("RefreshModelsRequested")
                .field(id)
                .finish(),
            Self::ModelToggled(id, model, enabled) => formatter
                .debug_tuple("ModelToggled")
                .field(id)
                .field(model)
                .field(enabled)
                .finish(),
        }
    }
}

impl ProviderView {
    pub fn new(
        id: ProviderId,
        name: impl Into<String>,
        has_key: bool,
        models: Vec<String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            api_key_draft: String::new(),
            credential_present: has_key,
            discovered_models: models.clone(),
            selected_models: models,
            discovering: false,
            error: None,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            scroll_id: Id::unique(),
            providers: Vec::new(),
            active_provider: None,
            error: None,
        }
    }
}

impl Settings {
    pub fn with_providers(mut self, providers: Vec<ProviderView>) -> Self {
        self.providers = providers;
        self
    }
}
