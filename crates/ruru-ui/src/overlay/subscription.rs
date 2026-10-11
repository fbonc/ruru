use iced::Subscription;

use super::{Input, Overlay, OverlayMode};

impl Overlay {
    pub fn subscription(&self) -> Subscription<Input> {
        if self.mode == OverlayMode::Answering {
            self.answering.subscription().map(Input::Answering)
        } else {
            Subscription::none()
        }
    }
}
