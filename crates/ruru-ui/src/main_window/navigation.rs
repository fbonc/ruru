use super::{MainWindow, Section};

impl MainWindow {
    pub fn jump_to_section<T>(&self, section: Section) -> iced::Task<T> {
        self.settings.jump_to_section(section)
    }
}
