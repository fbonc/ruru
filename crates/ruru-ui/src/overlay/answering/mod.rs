use iced::widget::markdown;

mod state;
mod subscription;
mod update;
mod view;

pub(crate) use state::Answering;

#[derive(Debug, Clone)]
pub enum Input {
    LinkClicked(markdown::Uri),
    LoadingTick,
    AppendToken(String),
    Finished,
    Failed(String),
    Reset,
}

impl Answering {
    pub(crate) fn answer(&self) -> &str {
        &self.answer
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub(crate) fn is_streaming(&self) -> bool {
        !self.done && self.error.is_none()
    }
    pub(crate) fn can_copy(&self) -> bool {
        self.done && self.error.is_none() && !self.answer.is_empty()
    }
    fn is_waiting(&self) -> bool {
        self.answer.is_empty() && self.is_streaming()
    }
}
