use ruru_types::ProviderId;

use crate::Result;

mod adapter;
mod state;

pub use state::KeychainCredentialStore;

pub trait CredentialStore: Send + Sync {
    fn get(&self, provider: &ProviderId) -> Result<Option<String>>;

    fn set(&self, provider: &ProviderId, api_key: &str) -> Result<()>;

    fn remove(&self, provider: &ProviderId) -> Result<()>;
}

const DEFAULT_KEYCHAIN_SERVICE: &str = "ruru";
const LEGACY_KEYCHAIN_SERVICES: [&str; 2] = ["minim", "little-owl"];

impl KeychainCredentialStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            legacy_services: Vec::new(),
        }
    }
}

impl Default for KeychainCredentialStore {
    fn default() -> Self {
        Self {
            service: DEFAULT_KEYCHAIN_SERVICE.into(),
            legacy_services: LEGACY_KEYCHAIN_SERVICES.map(str::to_owned).to_vec(),
        }
    }
}
