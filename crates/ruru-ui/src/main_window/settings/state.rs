use iced::widget::Id;
use ruru_types::ProviderId;

pub struct ProviderView {
    pub(super) id: ProviderId,
    pub(super) name: String,
    pub(super) api_key_draft: String,
    pub(super) credential_present: bool,
    pub(super) discovered_models: Vec<String>,
    pub(super) selected_models: Vec<String>,
    pub(super) discovering: bool,
    pub(super) error: Option<String>,
}

pub struct Settings {
    pub(super) scroll_id: Id,
    pub(super) providers: Vec<ProviderView>,
    pub(super) active_provider: Option<ProviderId>,
    pub(super) error: Option<String>,
}
