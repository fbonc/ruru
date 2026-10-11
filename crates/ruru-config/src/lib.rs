use std::collections::BTreeMap;

mod adapter;
mod credentials;
mod error;
mod state;

pub use credentials::{CredentialStore, KeychainCredentialStore};
pub use error::{Error, Result};
pub use state::{AppConfig, ProviderSettings};

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            selected_model: None,
            providers: BTreeMap::new(),
        }
    }
}

impl AppConfig {
    pub const CURRENT_VERSION: u32 = 1;
}
