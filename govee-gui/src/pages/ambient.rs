use iced::widget::{button, column, container, horizontal_space, row, slider, text, toggler};
use iced::{Alignment, Color, Element, Length};
use crate::app::{App, Message};
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let is_active = app.active_mode.as_deref() == Some("ambient");

    let start_stop_btn = if is_active {
        button(text("■ Stop Ambient Sync").size(13).color(Color::WHITE))
            .padding([8, 20])
            .on_press(Message::StopMode)
            .style(style::danger_action_button())
    } else {
        button(text("▶ Start Ambient Sync").size(13).color(Color::WHITE))
            .padding([8, 20])
            .on_press(Message::StartAmbient)
            .style(style::accent_action_button())
    };

    let header = row![
        text("Ambient Sync").size(24).color(style::TEXT_PRIMARY),
        horizontal_space(),
        start_stop_btn,
    ]
    .align_y(Alignment::Center)
    .spacing(style::SPACING);

    // Brightness card
    let brightness_card = container(
        column![
            row![
                text("Brightness").size(14).color(style::TEXT_PRIMARY),
                horizontal_space(),
                text(format!("{}%", app.config.ambient.brightness)).size(14).color(style::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
            slider(1u8..=100u8, app.config.ambient.brightness, Message::SetAmbientBrightness)
                .width(Length::Fill),
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    // Dim card
    let dim_card = container(
        row![
            text("Dim").size(14).color(style::TEXT_PRIMARY),
            horizontal_space(),
            toggler(app.config.ambient.dim)
                .on_toggle(Message::ToggleAmbientDim),
        ]
        .align_y(Alignment::Center)
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    column![
        header,
        brightness_card,
        dim_card,
    ]
    .spacing(14)
    .into()
}
