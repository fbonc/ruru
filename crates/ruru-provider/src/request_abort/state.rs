use futures_util::future::AbortHandle;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default)]
pub(crate) struct RequestAbortController {
    pub(super) handle: Arc<Mutex<Option<AbortHandle>>>,
}
