use iced::Subscription;
use std::time::Duration;

use super::{Answering, Input};

impl Answering {
    pub fn subscription(&self) -> Subscription<Input> {
        if self.is_waiting() {
            iced::time::every(Duration::from_millis(80)).map(|_| Input::LoadingTick)
        } else {
            Subscription::none()
        }
    }
}
