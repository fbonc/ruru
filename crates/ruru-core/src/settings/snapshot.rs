use ruru_provider::ProviderKind;

use super::{ProviderSettingsInfo, Settings, SettingsSnapshot};

impl Settings {
    pub(crate) fn snapshot(&self) -> SettingsSnapshot {
        let state = self.state.lock().expect("settings mutex poisoned");
        let available_models = self.providers.configured_models();
        SettingsSnapshot {
            providers: ProviderKind::ALL
                .into_iter()
                .map(|kind| {
                    let id = kind.id();
                    ProviderSettingsInfo {
                        models: state
                            .config
                            .providers
                            .get(&id)
                            .map(|settings| settings.models.clone())
                            .unwrap_or_default(),
                        has_key: state.credentials_present.contains(&id),
                        id,
                        name: kind.name(),
                    }
                })
                .collect(),
            selected_model: state
                .config
                .selected_model
                .clone()
                .filter(|model| available_models.contains(model))
                .or_else(|| available_models.first().cloned()),
            available_models,
            load_error: state.load_error.clone(),
        }
    }
}
