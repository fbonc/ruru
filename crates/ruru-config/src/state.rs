use ruru_types::{ModelSelection, ProviderId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub selected_model: Option<ModelSelection>,
    pub providers: BTreeMap<ProviderId, ProviderSettings>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderSettings {
    pub models: Vec<String>,
}
