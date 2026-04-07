use iced::widget::{button, column, container, horizontal_space, row, text};
use iced::{Alignment, Element};
use crate::app::{App, Message};
use crate::style;

const AUDIO_MODES: &[&str] = &["energy", "frequency", "beat", "drop"];

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

    let brightness_card = crate::widgets::slider_card::slider_card(
        "Brightness", app.config.audio.brightness, "%", 1..=100, Message::SetAudioBrightness,
    );

    let segments_card = crate::widgets::slider_card::segments_card(
        app.config.audio.segments, Message::SetAudioSegments,
    );

    column![
        header,
        mode_card,
        brightness_card,
        segments_card,
    ]
    .spacing(14)
    .into()
}
