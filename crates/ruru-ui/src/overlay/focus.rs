use iced::Task;

use super::{Input, Overlay};

impl Overlay {
    pub fn check_prompt_input_focus(&self) -> Task<Input> {
        self.prompting.check_focus().map(Input::Prompting)
    }
}
