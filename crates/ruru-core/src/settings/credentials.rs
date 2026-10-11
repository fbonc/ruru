use ruru_types::ProviderId;

use super::{Settings, SettingsChange, SettingsUpdate};

impl Settings {
    pub(crate) fn save_key(&self, id: ProviderId, key: String) -> Result<SettingsUpdate, String> {
        let mut state = self.state.lock().expect("settings mutex poisoned");
        self.check_edit(&state, &id)?;
        let key = key.trim();
        if key.is_empty() {
            return Err("Paste an API key first".into());
        }
        let previous = self
            .credentials
            .get(&id)
            .map_err(|error| error.to_string())?;
        self.credentials
            .set(&id, key)
            .map_err(|error| error.to_string())?;
        let mut next = state.config.clone();
        next.providers.entry(id.clone()).or_default();
        let result = self.persist_provider(
            &mut state,
            id.clone(),
            next,
            Some(key.to_owned()),
            SettingsChange::KeySaved,
        );
        if result.is_err() {
            self.restore_key(&id, previous);
        } else {
            self.invalidate_discovery(&mut state, &id);
        }
        result
    }
    pub(crate) fn remove_key(&self, id: ProviderId) -> Result<SettingsUpdate, String> {
        let mut state = self.state.lock().expect("settings mutex poisoned");
        self.check_edit(&state, &id)?;
        let previous = self
            .credentials
            .get(&id)
            .map_err(|error| error.to_string())?;
        self.credentials
            .remove(&id)
            .map_err(|error| error.to_string())?;
        let next = state.config.clone();
        let result = self.persist_provider(
            &mut state,
            id.clone(),
            next,
            None,
            SettingsChange::KeyRemoved,
        );
        if result.is_err() {
            self.restore_key(&id, previous);
        } else {
            self.invalidate_discovery(&mut state, &id);
        }
        result
    }
    fn restore_key(&self, id: &ProviderId, previous: Option<String>) {
        let result = match previous {
            Some(key) => self.credentials.set(id, &key),
            None => self.credentials.remove(id),
        };
        if let Err(error) = result {
            eprintln!("failed to restore {id} credentials: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{TestConfig, TestCredentials, test_settings};
    use ruru_config::CredentialStore;
    use ruru_types::ProviderId;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    #[test]
    fn a_failed_config_save_restores_the_previous_key() {
        let config = Arc::new(TestConfig::default());
        let credentials = Arc::new(TestCredentials::default());
        let settings = test_settings(config.clone(), credentials.clone());
        let id = ProviderId::new("openai");
        config.fail_saves.store(true, Ordering::SeqCst);

        assert!(settings.save_key(id.clone(), "test-key".into()).is_err());
        assert!(credentials.get(&id).unwrap().is_none());
        assert!(!settings.snapshot().providers[0].has_key);
    }
}
