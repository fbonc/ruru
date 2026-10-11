use std::sync::Arc;
use std::time::Duration;

use crate::request_abort::RequestAbortController;

mod adapter;
mod state;

pub use state::MockProvider;

impl MockProvider {
    pub fn new(chunk_delay: Duration) -> Self {
        Self {
            chunk_delay,
            request_abort: RequestAbortController::default(),
            last_request: Arc::default(),
        }
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new(Duration::from_millis(55))
    }
}
