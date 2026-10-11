use ruru_types::{ModelSelection, Target};

use super::{answering, prompting};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OverlayMode {
    #[default]
    Prompting,
    Answering,
}

#[derive(Debug)]
pub struct Overlay {
    pub(super) visible: bool,
    pub(super) target: Option<Target>,
    pub(super) selected_model: Option<ModelSelection>,
    pub(super) available_models: Vec<ModelSelection>,
    pub(super) prompting: prompting::Prompting,
    pub(super) answering: answering::Answering,
    pub(super) capture_ready: bool,
    pub(super) mode: OverlayMode,
}
