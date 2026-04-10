use iced::widget::{button, column, container, horizontal_space, row, slider, text, toggler};
use iced::{Alignment, Element, Length};
use crate::app::{App, Message};
use crate::style;

const AUDIO_MODES: &[&str] = &["energy", "frequency", "beat", "drop"];
const AUDIO_PALETTES: &[&str] = &["fire", "ocean", "forest", "neon", "ice", "sunset", "rainbow"];

pub fn view(app: &App) -> Element<'_, Message> {
    let is_active = app.active_mode.as_deref() == Some("audio");

    let start_stop_btn = crate::widgets::slider_card::start_stop_button(
        is_active, "Audio Visualizer", Message::StartAudio,
    );

    let header = row![
        text("Audio Visualizer").size(24).color(style::TEXT_PRIMARY),
        horizontal_space(),
        start_stop_btn,
    ]
    .align_y(Alignment::Center)
    .spacing(style::SPACING);

    // Mode card
    let mut mode_row = row![].spacing(6);
    for &mode in AUDIO_MODES {
        let is_mode_active = app.config.audio.mode == mode;
        let btn = button(text(mode).size(12))
            .padding([6, 14])
            .on_press(Message::SetAudioMode(mode.to_string()))
            .style(style::pill_button(is_mode_active));
        mode_row = mode_row.push(btn);
    }

    let mode_card = container(
        column![
            text("Mode").size(14).color(style::TEXT_PRIMARY),
            mode_row,
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    // Palette card
    let mut palette_row = row![].spacing(6);
    for &palette in AUDIO_PALETTES {
        let is_palette_active = app.config.audio.palette == palette;
        let btn = button(text(palette).size(12))
            .padding([6, 14])
            .on_press(Message::SetAudioPalette(palette.to_string()))
            .style(style::pill_button(is_palette_active));
        palette_row = palette_row.push(btn);
    }

    let palette_card = container(
        column![
            text("Palette").size(14).color(style::TEXT_PRIMARY),
            palette_row,
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    let brightness_card = crate::widgets::slider_card::slider_card(
        "Brightness", app.config.audio.brightness, "%", 1..=100, Message::SetAudioBrightness, Message::ApplyAudioSettings,
    );

    // Sensitivity slider (0.1–3.0, stored as u8 1–30 mapped to f64)
    let sens_val = (app.config.audio.sensitivity.clamp(0.1, 3.0) * 10.0).round() as u8;
    let sensitivity_card = container(
        column![
            row![
                text("Sensitivity").size(14).color(style::TEXT_PRIMARY),
                horizontal_space(),
                text(format!("{:.1}x", app.config.audio.sensitivity)).size(14).color(style::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
            slider(1u8..=30u8, sens_val, Message::SetAudioSensitivity)
                .on_release(Message::ApplyAudioSettings)
                .width(Length::Fill),
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    let segments_card = crate::widgets::slider_card::segments_card(
        app.config.audio.segments, Message::SetAudioSegments, Message::ApplyAudioSettings,
    );

    // Gradient toggle
    let gradient_card = container(
        row![
            text("Gradient").size(14).color(style::TEXT_PRIMARY),
            horizontal_space(),
            toggler(app.config.audio.gradient)
                .on_toggle(Message::ToggleAudioGradient),
        ]
        .align_y(Alignment::Center)
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    column![
        header,
        mode_card,
        palette_card,
        brightness_card,
        sensitivity_card,
        segments_card,
        gradient_card,
    ]
    .spacing(14)
    .into()
}
