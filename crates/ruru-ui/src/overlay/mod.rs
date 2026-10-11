use ruru_types::{ModelSelection, Target};

pub mod answering;
pub mod prompting;

mod focus;
mod latex;
mod state;
mod style;
mod subscription;
mod update;
mod view;

pub use state::{Overlay, OverlayMode};

#[derive(Debug, Clone)]
pub enum Input {
    Prompting(prompting::Input),
    Answering(answering::Input),
    Show,
    CaptureCompleted,
    SetTarget(Target),
    CaptureFailed,
    AppendAnswer(String),
    FinishAnswer,
    FailAnswer(String),
    ModelsChanged(Vec<ModelSelection>),
    ProviderRequestAbortRequested,
    ProviderRequestRetryRequested,
    CopyAnswerRequested,
    RemoveTargetRequested,
    BackRequested,
    DismissRequested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    Submitted {
        prompt: Option<String>,
        model: Option<ModelSelection>,
    },
    ModelSelected(ModelSelection),
    TargetRemoved,
    CaptureRegionRequested,
    ProviderRequestAbortRequested,
    ProviderRequestRetryRequested,
    CopyAnswerRequested(String),
    LinkClicked(String),
    ModeChanged(OverlayMode),
    Dismissed,
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            visible: false,
            target: None,
            selected_model: None,
            available_models: Vec::new(),
            prompting: prompting::Prompting::new(),
            answering: answering::Answering::default(),
            capture_ready: true,
            mode: OverlayMode::default(),
        }
    }
}

impl Overlay {
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn mode(&self) -> OverlayMode {
        self.mode
    }

    pub fn target(&self) -> Option<&Target> {
        self.target.as_ref()
    }

    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_selected_model(mut self, selected_model: Option<ModelSelection>) -> Self {
        self.selected_model = selected_model;
        self
    }
    pub fn with_models(mut self, models: Vec<ModelSelection>) -> Self {
        self.update(Input::ModelsChanged(models));
        self
    }
    pub fn selected_model(&self) -> Option<&ModelSelection> {
        self.selected_model.as_ref()
    }
}
