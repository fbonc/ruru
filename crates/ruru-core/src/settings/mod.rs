use ruru_types::{ModelSelection, ProviderId};

mod credentials;
mod discovery;
mod load;
mod models;
mod persistence;
mod snapshot;
mod state;

pub(crate) use state::Settings;

#[derive(Debug, Clone)]
pub struct ProviderSettingsInfo {
    pub id: ProviderId,
    pub name: &'static str,
    pub has_key: bool,
    pub models: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SettingsSnapshot {
    pub providers: Vec<ProviderSettingsInfo>,
    pub available_models: Vec<ModelSelection>,
    pub selected_model: Option<ModelSelection>,
    pub load_error: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum SettingsChange {
    KeySaved,
    KeyRemoved,
    ModelsChanged,
}

#[derive(Debug, Clone)]
pub struct SettingsUpdate {
    pub provider: ProviderId,
    pub change: SettingsChange,
    pub models: Vec<String>,
    pub available_models: Vec<ModelSelection>,
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use ruru_config::{CredentialStore, Error, Result};
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use super::Settings;
    use ruru_config::AppConfig;
    use ruru_types::ProviderId;
    use std::sync::Arc;

    #[derive(Default)]
    pub(super) struct TestConfig {
        pub(super) value: Mutex<AppConfig>,
        pub(super) fail_loads: AtomicBool,
        pub(super) fail_saves: AtomicBool,
    }

    impl TestConfig {
        fn load(&self) -> Result<AppConfig> {
            if self.fail_loads.load(Ordering::SeqCst) {
                return Err(Error::InvalidConfig("load failed".into()));
            }
            Ok(self.value.lock().expect("config mutex poisoned").clone())
        }

        fn save(&self, config: &AppConfig) -> Result<()> {
            if self.fail_saves.load(Ordering::SeqCst) {
                return Err(Error::InvalidConfig("save failed".into()));
            }
            *self.value.lock().expect("config mutex poisoned") = config.clone();
            Ok(())
        }
    }

    #[derive(Default)]
    pub(super) struct TestCredentials(Mutex<BTreeMap<ProviderId, String>>);

    impl CredentialStore for TestCredentials {
        fn get(&self, provider: &ProviderId) -> Result<Option<String>> {
            Ok(self
                .0
                .lock()
                .expect("credentials mutex poisoned")
                .get(provider)
                .cloned())
        }

        fn set(&self, provider: &ProviderId, key: &str) -> Result<()> {
            self.0
                .lock()
                .expect("credentials mutex poisoned")
                .insert(provider.clone(), key.into());
            Ok(())
        }

        fn remove(&self, provider: &ProviderId) -> Result<()> {
            self.0
                .lock()
                .expect("credentials mutex poisoned")
                .remove(provider);
            Ok(())
        }
    }

    pub(super) fn test_settings(
        config: Arc<TestConfig>,
        credentials: Arc<TestCredentials>,
    ) -> Settings {
        let loaded = config.load();
        Settings::new(loaded, credentials, Arc::new(move |next| config.save(next)))
    }
}
