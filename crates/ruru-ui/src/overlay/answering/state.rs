use iced::widget::markdown;

#[derive(Debug, Default)]
pub(crate) struct Answering {
    pub(super) answer: String,
    pub(super) markdown: markdown::Content,
    pub(super) done: bool,
    pub(super) error: Option<String>,
    pub(super) loading_frame: u8,
}
