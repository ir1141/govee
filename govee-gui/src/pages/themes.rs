use iced::widget::{column, text};
use iced::Element;
use crate::app::Message;

pub fn view() -> Element<'static, Message> {
    column![text("Themes").size(20)]
        .spacing(8.0)
        .into()
}
