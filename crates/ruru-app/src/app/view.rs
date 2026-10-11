use iced::widget::mouse_area;
use iced::{Element, window as iced_window};

use super::{App, Input};

impl App {
    pub(super) fn view(&self, id: iced_window::Id) -> Element<'_, Input> {
        if id == self.overlay.id {
            mouse_area(self.overlay.ui.view().map(Input::Overlay))
                .on_press(Input::DragWindow)
                .into()
        } else if id == self.main_window.id {
            self.main_window.ui.view().map(Input::MainWindow)
        } else {
            iced::widget::space().into()
        }
    }
}
