use iced::widget::{button, column, container, horizontal_space, row, slider, text};
use iced::{Alignment, Color, Element, Length};
use crate::app::{App, Message};
use crate::style;

const AUDIO_MODES: &[&str] = &["energy", "frequency", "beat", "drop"];

pub fn view(app: &App) -> Element<'_, Message> {
    let is_active = app.active_mode.as_deref() == Some("audio");

    let start_stop_btn = if is_active {
        button(text("■ Stop Audio Visualizer").size(13).color(Color::WHITE))
            .padding([8, 20])
            .on_press(Message::StopMode)
            .style(style::danger_action_button())
    } else {
        button(text("▶ Start Audio Visualizer").size(13).color(Color::WHITE))
            .padding([8, 20])
            .on_press(Message::StartAudio)
            .style(style::accent_action_button())
    };

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

    // Brightness card
    let brightness_card = container(
        column![
            row![
                text("Brightness").size(14).color(style::TEXT_PRIMARY),
                horizontal_space(),
                text(format!("{}%", app.config.audio.brightness)).size(14).color(style::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
            slider(1u8..=100u8, app.config.audio.brightness, Message::SetAudioBrightness)
                .width(Length::Fill),
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    // Segments card
    let segments_card = container(
        column![
            row![
                text("Segments").size(14).color(style::TEXT_PRIMARY),
                horizontal_space(),
                text(format!("{}", app.config.audio.segments)).size(14).color(style::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
            slider(1u8..=15u8, app.config.audio.segments as u8, |v| Message::SetAudioSegments(v as usize))
                .width(Length::Fill),
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    column![
        header,
        mode_card,
        brightness_card,
        segments_card,
    ]
    .spacing(14)
    .into()
}
