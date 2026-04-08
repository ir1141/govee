//! Sunlight mode settings page: preset picker, brightness, segments, and transition.

use iced::widget::{button, column, container, horizontal_space, row, text};
use iced::{Alignment, Element};
use crate::app::{App, Message};
use crate::style;

const PRESETS: &[&str] = &["coastal", "arctic", "ember", "simple"];

pub fn view(app: &App) -> Element<'_, Message> {
    let is_active = app.active_mode.as_deref() == Some("sunlight");

    let start_stop_btn = crate::widgets::slider_card::start_stop_button(
        is_active, "Sunlight", Message::StartSunlight,
    );

    let header = row![
        text("Sunlight").size(24).color(style::TEXT_PRIMARY),
        horizontal_space(),
        start_stop_btn,
    ]
    .align_y(Alignment::Center)
    .spacing(style::SPACING);

    // Preset picker
    let mut preset_row = row![].spacing(6);
    for &preset in PRESETS {
        let is_preset_active = app.config.sunlight.preset == preset;
        let btn = button(text(preset).size(12))
            .padding([6, 14])
            .on_press(Message::SetSunlightPreset(preset.to_string()))
            .style(style::pill_button(is_preset_active));
        preset_row = preset_row.push(btn);
    }

    let preset_card = container(
        column![
            text("Preset").size(14).color(style::TEXT_PRIMARY),
            preset_row,
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    let brightness_card = crate::widgets::slider_card::slider_card(
        "Brightness", app.config.sunlight.brightness, "%", 1..=100, Message::SetSunlightBrightness,
    );

    let segments_card = crate::widgets::slider_card::segments_card(
        app.config.sunlight.segments, Message::SetSunlightSegments,
    );

    let transition_card = crate::widgets::slider_card::slider_card(
        "Transition", app.config.sunlight.transition as u8, "min", 10..=120,
        |v| Message::SetSunlightTransition(v as u32),
    );

    column![
        header,
        preset_card,
        brightness_card,
        segments_card,
        transition_card,
    ]
    .spacing(14)
    .into()
}
