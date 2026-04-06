use iced::widget::{button, column, horizontal_space, row, slider, text, toggler};
use iced::{Alignment, Border, Color, Element, Length};
use crate::app::{App, Message};
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let is_active = app.active_mode.as_deref() == Some("screen");

    let start_stop_btn = if is_active {
        button(text("■ Stop Screen Capture").size(13).color(Color::WHITE))
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
        button(text("▶ Start Screen Capture").size(13).color(Color::WHITE))
            .padding([6, 16])
            .on_press(Message::StartScreen)
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
        text("Screen Capture").size(22).color(style::TEXT_PRIMARY),
        horizontal_space(),
        start_stop_btn,
    ]
    .align_y(Alignment::Center)
    .spacing(style::SPACING);

    // FPS
    let fps_row = row![
        text("FPS").size(14).color(style::TEXT_PRIMARY),
        horizontal_space(),
        text(format!("{}", app.config.screen.fps)).size(14).color(style::TEXT_SECONDARY),
    ]
    .align_y(Alignment::Center);

    let fps_slider = slider(
        1u32..=30u32,
        app.config.screen.fps,
        Message::SetScreenFps,
    )
    .width(Length::Fill);

    // Brightness
    let brightness_row = row![
        text("Brightness").size(14).color(style::TEXT_PRIMARY),
        horizontal_space(),
        text(format!("{}%", app.config.screen.brightness)).size(14).color(style::TEXT_SECONDARY),
    ]
    .align_y(Alignment::Center);

    let brightness_slider = slider(
        1u8..=100u8,
        app.config.screen.brightness,
        Message::SetScreenBrightness,
    )
    .width(Length::Fill);

    // Segments
    let segments_row = row![
        text("Segments").size(14).color(style::TEXT_PRIMARY),
        horizontal_space(),
        text(format!("{}", app.config.screen.segments)).size(14).color(style::TEXT_SECONDARY),
    ]
    .align_y(Alignment::Center);

    let segments_slider = slider(
        1u8..=15u8,
        app.config.screen.segments as u8,
        |v| Message::SetScreenSegments(v as usize),
    )
    .width(Length::Fill);

    // Mirror toggle
    let mirror_row = row![
        text("Mirror").size(14).color(style::TEXT_PRIMARY),
        horizontal_space(),
        toggler(app.config.screen.mirror)
            .on_toggle(Message::ToggleScreenMirror),
    ]
    .align_y(Alignment::Center)
    .spacing(10);

    column![
        header,
        iced::widget::rule::Rule::horizontal(1),
        fps_row,
        fps_slider,
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
