use iced::Theme;

use super::App;

pub(super) fn application(_state: &App, _theme: &Theme) -> iced::theme::Style {
    iced::theme::Style {
        background_color: iced::Color::TRANSPARENT,
        text_color: iced::Color::from_rgb8(0xED, 0xEB, 0xE6),
    }
}

pub(super) const WINDOW_WIDTH: f32 = 500.0;
pub(super) const MIN_WINDOW_WIDTH: f32 = 225.0;
pub(super) const PROMPTING_HEIGHT: f32 = 180.0;
pub(super) const MIN_WINDOW_HEIGHT: f32 = PROMPTING_HEIGHT;
pub(super) const ANSWERING_HEIGHT: f32 = 360.0;
