use std::sync::Arc;

use crate::request_abort::RequestAbortController;

mod adapter;
mod discovery;
mod protocol;
mod state;

pub use state::{OpenAiConfig, OpenAiProvider};

const DEFAULT_BASE_URL: &str = "https://api.openai.com";

impl OpenAiConfig {
    pub fn new(api_key: impl Into<String>, models: Vec<String>) -> Self {
        Self {
            api_key: api_key.into(),
            models,
            base_url: DEFAULT_BASE_URL.into(),
        }
    }
}

impl OpenAiProvider {
    pub fn new(config: OpenAiConfig) -> Self {
        let base_url = config.base_url.trim_end_matches('/');
        let endpoint = format!("{base_url}/v1/responses");
        let models_endpoint = format!("{base_url}/v1/models");

        Self {
            client: reqwest::Client::new(),
            api_key: config.api_key.into(),
            models: config.models,
            endpoint: endpoint.into(),
            models_endpoint: models_endpoint.into(),
            request_abort: RequestAbortController::default(),
            last_request: Arc::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ProviderRequest;
    use ruru_types::{ContextCapture, ImageCapture, Target};

    pub(super) fn request(target: Option<Target>) -> ProviderRequest {
        ProviderRequest {
            prompt: Some("What does this mean?".into()),
            target,
            context: Some(ContextCapture {
                image: ImageCapture {
                    png: b"context".to_vec(),
                    region: None,
                },
                accessibility_text: Some("surrounding text".into()),
            }),
        }
    }
}
