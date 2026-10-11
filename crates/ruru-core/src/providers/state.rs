use ruru_provider::Provider;
use ruru_types::ProviderId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct Providers {
    pub(super) providers: Arc<RwLock<HashMap<ProviderId, Arc<dyn Provider>>>>,
}
