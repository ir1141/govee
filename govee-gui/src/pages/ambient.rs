use iced::widget::{column, container, horizontal_space, row, text, toggler};
use iced::{Alignment, Element};
use crate::app::{App, Message};
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let is_active = app.active_mode.as_deref() == Some("ambient");

    let start_stop_btn = crate::widgets::slider_card::start_stop_button(
        is_active, "Ambient Sync", Message::StartAmbient,
    );

    let header = row![
        text("Ambient Sync").size(24).color(style::TEXT_PRIMARY),
        horizontal_space(),
        start_stop_btn,
    ]
    .align_y(Alignment::Center)
    .spacing(style::SPACING);

    let brightness_card = crate::widgets::slider_card::slider_card(
        "Brightness", app.config.ambient.brightness, "%", 1..=100, Message::SetAmbientBrightness,
    );

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
