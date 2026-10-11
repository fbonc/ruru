use futures_util::future::{AbortHandle, AbortRegistration};

use super::RequestAbortController;

impl RequestAbortController {
    pub(crate) fn register(&self) -> AbortRegistration {
        let (handle, registration) = AbortHandle::new_pair();
        let previous = self
            .handle
            .lock()
            .expect("provider request abort mutex poisoned")
            .replace(handle);

        if let Some(previous) = previous {
            previous.abort();
        }

        registration
    }

    pub(crate) fn abort(&self) {
        if let Some(handle) = self
            .handle
            .lock()
            .expect("provider request abort mutex poisoned")
            .take()
        {
            handle.abort();
        }
    }
}
