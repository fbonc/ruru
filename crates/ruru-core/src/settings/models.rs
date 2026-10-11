use ruru_types::{ModelSelection, ProviderId};

use super::{Settings, SettingsChange, SettingsUpdate};

impl Settings {
    pub(crate) fn set_model(
        &self,
        id: ProviderId,
        model: String,
        enabled: bool,
    ) -> Result<SettingsUpdate, String> {
        let mut state = self.state.lock().expect("settings mutex poisoned");
        self.check_edit(&state, &id)?;
        let key = self
            .credentials
            .get(&id)
            .map_err(|error| error.to_string())?
            .ok_or("Save an API key first")?;
        let mut next = state.config.clone();
        let settings = next.providers.entry(id.clone()).or_default();
        if enabled {
            settings.models.push(model);
            settings.models.sort_unstable();
            settings.models.dedup();
        } else {
            settings.models.retain(|saved| saved != &model);
        }
        self.persist_provider(
            &mut state,
            id,
            next,
            Some(key),
            SettingsChange::ModelsChanged,
        )
    }
    pub(crate) fn select_model(&self, model: ModelSelection) -> Result<(), String> {
        let mut state = self.state.lock().expect("settings mutex poisoned");
        self.check_edit(&state, &model.provider)?;
        if !self.providers.configured_models().contains(&model) {
            return Err("model is not available".into());
        }
        let mut next = state.config.clone();
        next.selected_model = Some(model);
        (self.save_config)(&next).map_err(|error| error.to_string())?;
        state.config = next;
        Ok(())
    }
}
