use iced::widget::Id;
use ruru_types::ModelSelection;

use super::style;

mod focus;
mod state;
mod update;
mod view;

pub use state::Prompting;

#[derive(Debug, Clone)]
pub enum Input {
    InputChanged(String),
    InputFocusChanged(bool),
    SubmitRequested,
    ModelSelected(ModelSelection),
    CaptureRegionRequested,
    CaptureFailed,
    Reset,
}

fn random_placeholder() -> &'static str {
    use std::hash::{BuildHasher, Hasher};

    let n = std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish() as usize;
    style::PLACEHOLDERS[n % style::PLACEHOLDERS.len()]
}

impl Default for Prompting {
    fn default() -> Self {
        Self {
            prompt_value: String::new(),
            input_id: Id::unique(),
            input_focused: false,
            capture_failed: false,
            placeholder: random_placeholder(),
        }
    }
}

impl Prompting {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn prompt_value(&self) -> &str {
        &self.prompt_value
    }
}
