use ruru_config::AppConfig;
use ruru_provider::ProviderKind;
use ruru_types::{ModelSelection, ProviderId};

use super::state::SettingsState;
use super::{Settings, SettingsChange, SettingsUpdate};

impl Settings {
    pub(super) fn persist_provider(
        &self,
        state: &mut SettingsState,
        id: ProviderId,
        mut next: AppConfig,
        key: Option<String>,
        change: SettingsChange,
    ) -> Result<SettingsUpdate, String> {
        let models = next
            .providers
            .get(&id)
            .map(|settings| settings.models.clone())
            .unwrap_or_default();
        let mut available_models = self.providers.configured_models();
        available_models.retain(|selection| selection.provider != id);
        if key.is_some() {
            available_models.extend(
                models
                    .iter()
                    .map(|model| ModelSelection::new(id.clone(), model.clone())),
            );
        }
        available_models.sort();
        next.selected_model = state
            .config
            .selected_model
            .clone()
            .filter(|selected| available_models.contains(selected))
            .or_else(|| available_models.first().cloned());
        (self.save_config)(&next).map_err(|error| error.to_string())?;
        state.config = next;
        if let Some(key) = key {
            self.providers.configure(id.clone(), key, models.clone())?;
            state.credentials_present.insert(id.clone());
        } else {
            self.providers.remove(&id);
            state.credentials_present.remove(&id);
        }
        Ok(SettingsUpdate {
            provider: id,
            change,
            models,
            available_models,
        })
    }
    pub(super) fn check_edit(&self, state: &SettingsState, id: &ProviderId) -> Result<(), String> {
        if let Some(error) = &state.load_error {
            return Err(format!("Cannot edit settings: {error}"));
        }
        ProviderKind::from_id(id)
            .map(|_| ())
            .ok_or_else(|| format!("unknown provider: {id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{TestConfig, TestCredentials, test_settings};
    use ruru_config::CredentialStore;
    use ruru_types::ModelSelection;
    use ruru_types::ProviderId;
    use std::sync::Arc;

    #[test]
    fn settings_changes_update_storage_and_live_models() {
        let config = Arc::new(TestConfig::default());
        let credentials = Arc::new(TestCredentials::default());
        let settings = test_settings(config.clone(), credentials.clone());
        let id = ProviderId::new("openai");
        let model = ModelSelection::new(id.clone(), "gpt-test");

        settings.save_key(id.clone(), "test-key".into()).unwrap();
        settings
            .set_model(id.clone(), model.model.clone(), true)
            .unwrap();
        settings.select_model(model.clone()).unwrap();

        assert_eq!(settings.snapshot().available_models, vec![model.clone()]);
        assert_eq!(settings.snapshot().selected_model, Some(model));
        assert!(settings.snapshot().providers[0].has_key);

        settings.remove_key(id.clone()).unwrap();

        assert!(settings.snapshot().available_models.is_empty());
        assert!(settings.snapshot().selected_model.is_none());
        assert!(!settings.snapshot().providers[0].has_key);
        assert!(credentials.get(&id).unwrap().is_none());
        assert!(config.value.lock().unwrap().selected_model.is_none());
    }
}
