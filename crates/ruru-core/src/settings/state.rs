use ruru_config::{AppConfig, CredentialStore};
use ruru_types::ProviderId;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use crate::providers::Providers;

pub(super) type SaveConfig = dyn Fn(&AppConfig) -> ruru_config::Result<()> + Send + Sync;

pub(super) struct SettingsState {
    pub(super) config: AppConfig,
    pub(super) load_error: Option<String>,
    pub(super) credentials_present: BTreeSet<ProviderId>,
    pub(super) latest_discovery_id: BTreeMap<ProviderId, u64>,
}

pub(crate) struct Settings {
    pub(super) state: Mutex<SettingsState>,
    pub(crate) providers: Providers,
    pub(super) save_config: Arc<SaveConfig>,
    pub(super) credentials: Arc<dyn CredentialStore>,
}
