use iced::widget::{container, row, text};
use iced::{Background, Border, Color, Element, Length};
use crate::app::Message;
use crate::style;

pub fn view(connected: bool, mode_label: &str) -> Element<'static, Message> {
    let mode_label: String = mode_label.to_string();

    let status_text = if connected {
        text("● Connected").color(style::SUCCESS)
    } else {
        text("● Disconnected").color(Color::from_rgb(1.0, 0.3, 0.3))
    };

    let mode_text = text(format!("Mode: {}", mode_label)).color(style::TEXT_SECONDARY);

    let bar = row![status_text, mode_text]
        .spacing(16.0)
        .padding(8.0);

    container(bar)
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(style::SIDEBAR_BG)),
            border: Border {
                radius: 0.0.into(),
                color: style::SURFACE,
                width: 1.0,
            },
            ..Default::default()
        })
        .into()
}
