use iced::widget::{container, row, text, toggler};
use iced::{Color, Element, Length};
use crate::app::Message;
use crate::style;

pub fn view(connected: bool, mode_label: &str, mirror: bool) -> Element<'static, Message> {
    let mode_label: String = mode_label.to_string();

    let status_text = if connected {
        text("● Connected").size(13).color(style::SUCCESS)
    } else {
        text("● Disconnected").size(13).color(Color::from_rgb(1.0, 0.3, 0.3))
    };

    let mode_text = text(format!("Mode: {}", mode_label)).size(13).color(style::TEXT_SECONDARY);

    let mirror_toggle = row![
        text("Mirror").size(13).color(style::TEXT_MUTED),
        toggler(mirror).on_toggle(Message::ToggleMirror),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);

    let bar = row![
        status_text,
        mode_text,
        iced::widget::horizontal_space(),
        mirror_toggle,
    ]
    .spacing(16.0)
    .padding([10, 16])
    .align_y(iced::Alignment::Center);

    container(bar)
        .width(Length::Fill)
        .style(style::status_bar_style)
        .into()
}
