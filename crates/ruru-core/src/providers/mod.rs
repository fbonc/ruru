mod registry;
mod state;

pub(crate) use state::Providers;

impl Providers {
    pub fn new() -> Self {
        Self::default()
    }
}
