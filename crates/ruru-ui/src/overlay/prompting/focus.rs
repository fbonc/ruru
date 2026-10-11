use iced::Task;

use super::{Input, Prompting};

impl Prompting {
    pub fn check_focus(&self) -> Task<Input> {
        iced::widget::operation::is_focused(self.input_id.clone()).map(Input::InputFocusChanged)
    }
}
