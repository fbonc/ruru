use super::Settings;
use super::state::{SaveConfig, SettingsState};
use crate::providers::Providers;
use ruru_config::{AppConfig, CredentialStore};
use ruru_provider::ProviderKind;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

impl Settings {
    pub(crate) fn load(credentials: Arc<dyn CredentialStore>) -> Self {
        Self::new(AppConfig::load(), credentials, Arc::new(AppConfig::save))
    }
    pub(crate) fn new(
        config_result: ruru_config::Result<AppConfig>,
        credentials: Arc<dyn CredentialStore>,
        save_config: Arc<SaveConfig>,
    ) -> Self {
        let (config, load_error) = match config_result {
            Ok(config) => (config, None),
            Err(error) => {
                eprintln!("failed to load application configuration: {error}");
                (AppConfig::default(), Some(error.to_string()))
            }
        };
        let providers = Providers::new();
        let mut credentials_present = BTreeSet::new();

        for kind in ProviderKind::ALL {
            let id = kind.id();
            match credentials.get(&id) {
                Ok(Some(key)) => {
                    let models = config
                        .providers
                        .get(&id)
                        .map(|settings| settings.models.clone())
                        .unwrap_or_default();
                    if let Err(error) = providers.configure(id.clone(), key, models) {
                        eprintln!("failed to configure {}: {error}", kind.name());
                    } else {
                        credentials_present.insert(id);
                    }
                }
                Ok(None) => {}
                Err(error) => eprintln!("failed to load {} credentials: {error}", kind.name()),
            }
        }

        Self {
            state: Mutex::new(SettingsState {
                config,
                load_error,
                credentials_present,
                latest_discovery_id: BTreeMap::new(),
            }),
            providers,
            save_config,
            credentials,
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
    fn a_failed_config_load_blocks_settings_writes() {
        let config = Arc::new(TestConfig::default());
        config.fail_loads.store(true, Ordering::SeqCst);
        let credentials = Arc::new(TestCredentials::default());
        let settings = test_settings(config, credentials.clone());
        let id = ProviderId::new("openai");

        assert!(settings.snapshot().load_error.is_some());
        assert!(settings.save_key(id.clone(), "test-key".into()).is_err());
        assert!(credentials.get(&id).unwrap().is_none());
    }
}
