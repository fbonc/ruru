use iced::Task;
use iced::widget::operation;

use super::{Section, Settings};

impl Settings {
    pub fn jump_to_section<T>(&self, section: Section) -> Task<T> {
        match section {
            Section::Providers => {
                operation::snap_to(self.scroll_id.clone(), operation::RelativeOffset::START)
            }
        }
    }
}
