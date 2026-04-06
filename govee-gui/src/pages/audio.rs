use iced::widget::{button, column, horizontal_space, row, slider, text, toggler};
use iced::{Alignment, Border, Color, Element, Length};
use crate::app::{App, Message};
use crate::style;

const AUDIO_MODES: &[&str] = &["energy", "frequency", "beat", "drop"];

pub fn view(app: &App) -> Element<'_, Message> {
    let is_active = app.active_mode.as_deref() == Some("audio");

    let start_stop_btn = if is_active {
        button(text("■ Stop Audio Visualizer").size(13).color(Color::WHITE))
            .padding([6, 16])
            .on_press(Message::StopMode)
            .style(|_theme, status| button::Style {
                background: Some(iced::Background::Color(match status {
                    button::Status::Hovered => Color::from_rgb(0.8, 0.2, 0.2),
                    _ => Color::from_rgb(0.6, 0.15, 0.15),
                })),
                text_color: Color::WHITE,
                border: Border { radius: style::RADIUS.into(), ..Default::default() },
                ..Default::default()
            })
    } else {
        button(text("▶ Start Audio Visualizer").size(13).color(Color::WHITE))
            .padding([6, 16])
            .on_press(Message::StartAudio)
            .style(|_theme, status| button::Style {
                background: Some(iced::Background::Color(match status {
                    button::Status::Hovered => Color::from_rgb(
                        style::ACCENT.r + 0.1,
                        style::ACCENT.g + 0.1,
                        style::ACCENT.b + 0.1,
                    ),
                    _ => style::ACCENT,
                })),
                text_color: Color::WHITE,
                border: Border { radius: style::RADIUS.into(), ..Default::default() },
                ..Default::default()
            })
    };

    let header = row![
        text("Audio Visualizer").size(22).color(style::TEXT_PRIMARY),
        horizontal_space(),
        start_stop_btn,
    ]
    .align_y(Alignment::Center)
    .spacing(style::SPACING);

    // Mode picker
    let mode_label = text("Mode").size(14).color(style::TEXT_PRIMARY);
    let mut mode_row = row![].spacing(4);
    for &mode in AUDIO_MODES {
        let is_mode_active = app.config.audio.mode == mode;
        let btn = button(text(mode).size(12))
            .padding([4, 10])
            .on_press(Message::SetAudioMode(mode.to_string()))
            .style(move |_theme, status| {
                let base = button::Style {
                    background: Some(iced::Background::Color(if is_mode_active {
                        style::ACCENT
                    } else {
                        style::SURFACE
                    })),
                    text_color: if is_mode_active {
                        Color::WHITE
                    } else {
                        style::TEXT_SECONDARY
                    },
                    border: Border {
                        radius: style::RADIUS.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                match status {
                    button::Status::Hovered => button::Style {
                        background: Some(iced::Background::Color(if is_mode_active {
                            style::ACCENT
                        } else {
                            Color {
                                r: style::SURFACE.r + 0.05,
                                g: style::SURFACE.g + 0.05,
                                b: style::SURFACE.b + 0.05,
                                a: 1.0,
                            }
                        })),
                        ..base
                    },
                    _ => base,
                }
            });
        mode_row = mode_row.push(btn);
    }

    // Brightness
    let brightness_row = row![
        text("Brightness").size(14).color(style::TEXT_PRIMARY),
        horizontal_space(),
        text(format!("{}%", app.config.audio.brightness)).size(14).color(style::TEXT_SECONDARY),
    ]
    .align_y(Alignment::Center);

    let brightness_slider = slider(
        1u8..=100u8,
        app.config.audio.brightness,
        Message::SetAudioBrightness,
    )
    .width(Length::Fill);

    // Segments
    let segments_row = row![
        text("Segments").size(14).color(style::TEXT_PRIMARY),
        horizontal_space(),
        text(format!("{}", app.config.audio.segments)).size(14).color(style::TEXT_SECONDARY),
    ]
    .align_y(Alignment::Center);

    let segments_slider = slider(
        1u8..=15u8,
        app.config.audio.segments as u8,
        |v| Message::SetAudioSegments(v as usize),
    )
    .width(Length::Fill);

    // Mirror toggle
    let mirror_row = row![
        text("Mirror").size(14).color(style::TEXT_PRIMARY),
        horizontal_space(),
        toggler(app.config.audio.mirror)
            .on_toggle(Message::ToggleAudioMirror),
    ]
    .align_y(Alignment::Center)
    .spacing(10);

    column![
        header,
        iced::widget::rule::Rule::horizontal(1),
        mode_label,
        mode_row,
        iced::widget::rule::Rule::horizontal(1),
        brightness_row,
        brightness_slider,
        iced::widget::rule::Rule::horizontal(1),
        segments_row,
        segments_slider,
        iced::widget::rule::Rule::horizontal(1),
        mirror_row,
    ]
    .spacing(12)
    .into()
}
