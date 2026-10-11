use crate::ProviderRequest;
use crate::request_abort::RequestAbortController;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MockProvider {
    pub(super) chunk_delay: Duration,
    pub(super) request_abort: RequestAbortController,
    pub(super) last_request: Arc<Mutex<Option<(String, ProviderRequest)>>>,
}
