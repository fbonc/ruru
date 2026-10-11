use crate::ProviderRequest;
use crate::request_abort::RequestAbortController;
use std::sync::{Arc, Mutex};

pub struct OpenAiConfig {
    pub api_key: String,
    pub models: Vec<String>,
    pub base_url: String,
}

#[derive(Clone)]
pub struct OpenAiProvider {
    pub(super) client: reqwest::Client,
    pub(super) api_key: Arc<str>,
    pub(super) models: Vec<String>,
    pub(super) endpoint: Arc<str>,
    pub(super) models_endpoint: Arc<str>,
    pub(super) request_abort: RequestAbortController,
    pub(super) last_request: Arc<Mutex<Option<(String, ProviderRequest)>>>,
}
