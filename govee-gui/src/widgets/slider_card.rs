use iced::widget::{column, container, horizontal_space, row, slider, text};
use iced::{Alignment, Element, Length};
use crate::app::Message;
use crate::style;

/// A labeled slider card with value display. Saves config on release, not on every tick.
pub fn slider_card<'a>(
    label: &'a str,
    value: u8,
    suffix: &'a str,
    range: std::ops::RangeInclusive<u8>,
    on_change: impl Fn(u8) -> Message + 'a,
) -> Element<'a, Message> {
    let display = if suffix.is_empty() {
        format!("{value}")
    } else {
        format!("{value}{suffix}")
    };
    container(
        column![
            row![
                text(label).size(14).color(style::TEXT_PRIMARY),
                horizontal_space(),
                text(display).size(14).color(style::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
            slider(range, value, on_change)
                .on_release(Message::SaveConfig)
                .width(Length::Fill),
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style)
    .into()
}

/// A labeled slider card for usize values (mapped through u8 slider).
pub fn segments_card<'a>(
    value: usize,
    on_change: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message> {
    container(
        column![
            row![
                text("Segments").size(14).color(style::TEXT_PRIMARY),
                horizontal_space(),
                text(format!("{value}")).size(14).color(style::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
            slider(1u8..=15u8, value as u8, move |v| on_change(v as usize))
                .on_release(Message::SaveConfig)
                .width(Length::Fill),
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style)
    .into()
}

/// Start/stop button for continuous modes.
pub fn start_stop_button<'a>(
    is_active: bool,
    mode_label: &'a str,
    start_msg: Message,
) -> Element<'a, Message> {
    if is_active {
        iced::widget::button(
            iced::widget::text(format!("■ Stop {mode_label}")).size(13).color(iced::Color::WHITE)
        )
            .padding([8, 20])
            .on_press(Message::StopMode)
            .style(style::danger_action_button())
            .into()
    } else {
        iced::widget::button(
            iced::widget::text(format!("▶ Start {mode_label}")).size(13).color(iced::Color::WHITE)
        )
            .padding([8, 20])
            .on_press(start_msg)
            .style(style::accent_action_button())
            .into()
    }
}
