use iced::widget::{container, markdown, scrollable, svg, text};
use iced::{Element, Fill, Radians};

use super::super::{latex, style};
use super::{Answering, Input};

impl Answering {
    pub fn view(&self) -> Element<'_, Input> {
        if let Some(error) = self.error() {
            return text(error)
                .size(style::ERROR_SIZE)
                .color(style::DANGER_COLOR)
                .into();
        }

        if self.is_waiting() {
            let angle = Radians(f32::from(self.loading_frame) * std::f32::consts::TAU / 24.0);
            let loader = svg(svg::Handle::from_memory(
                include_bytes!("../../../../../assets/loading.svg").as_slice(),
            ))
            .width(style::LOADING_ICON_SIZE)
            .height(style::LOADING_ICON_SIZE)
            .rotation(angle);

            return container(loader).center(Fill).into();
        }

        let answer = container(markdown::view_with(
            self.markdown.items(),
            markdown::Settings::with_text_size(style::ANSWER_SIZE, style::answer_markdown()),
            &latex::Viewer,
        ))
        .width(Fill)
        .padding(iced::Padding {
            top: 0.0,
            right: style::SCROLL_GUTTER,
            bottom: 0.0,
            left: 0.0,
        });

        scrollable(answer)
            .width(Fill)
            .height(Fill)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(style::SCROLLBAR_WIDTH)
                    .scroller_width(style::SCROLLBAR_WIDTH)
                    .margin(2.0),
            ))
            .style(style::scroll)
            .into()
    }
}
