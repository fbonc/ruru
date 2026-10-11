use futures_util::future::AbortHandle;
use ruru_capture::Capturer;
use ruru_provider::Provider;
use ruru_types::{ContextCapture, Provenance, Target};
use std::sync::{Arc, Mutex};

use crate::settings::Settings;

pub(crate) struct Core {
    pub(super) state: Mutex<State>,
    pub(super) settings: Arc<Settings>,
    pub(super) capturer: Arc<dyn Capturer>,
}

#[derive(Debug, Clone)]
pub struct Capture {
    pub target: Option<Target>,
    pub context: Option<ContextCapture>,
    pub provenance: Option<Provenance>,
    pub elapsed_ms: u32,
}

#[derive(Default)]
pub(crate) struct State {
    pub(crate) capture_id: u64,
    pub(super) current_capture: Option<Capture>,
    pub(super) provider_request_id: u64,
    pub(super) provider_abort: Option<AbortHandle>,
    pub(super) provider: Option<Arc<dyn Provider>>,
}
