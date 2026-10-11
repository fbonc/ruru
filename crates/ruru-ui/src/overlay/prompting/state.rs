use iced::widget::Id;

#[derive(Debug)]
pub struct Prompting {
    pub(super) prompt_value: String,
    pub(super) input_id: Id,
    pub(super) input_focused: bool,
    pub(super) capture_failed: bool,
    pub(super) placeholder: &'static str,
}
