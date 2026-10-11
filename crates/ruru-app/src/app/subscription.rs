use iced::{Subscription, window as iced_window};

use super::{App, Input};

impl App {
    pub(super) fn subscription(&self) -> Subscription<Input> {
        Subscription::batch([
            iced::event::listen_with(|event, _status, window| match event {
                iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_))
                | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { .. }) => {
                    Some(Input::CheckPromptInputFocus(window))
                }
                iced::Event::Window(iced_window::Event::CloseRequested) => {
                    Some(Input::WindowCloseRequested(window))
                }
                _ => None,
            }),
            self.overlay.ui.subscription().map(Input::Overlay),
        ])
    }
}
