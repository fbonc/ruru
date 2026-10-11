#[derive(Debug, Clone)]
pub struct KeychainCredentialStore {
    pub(super) service: String,
    pub(super) legacy_services: Vec<String>,
}
