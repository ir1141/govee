use iced::widget::{button, column, horizontal_space, row, slider, text, toggler};
use iced::{Alignment, Border, Color, Element, Length};
use crate::app::{App, Message};
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let is_active = app.active_mode.as_deref() == Some("ambient");

    let start_stop_btn = if is_active {
        button(text("■ Stop Ambient Sync").size(13).color(Color::WHITE))
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
        button(text("▶ Start Ambient Sync").size(13).color(Color::WHITE))
            .padding([6, 16])
            .on_press(Message::StartAmbient)
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
        text("Ambient Sync").size(22).color(style::TEXT_PRIMARY),
        horizontal_space(),
        start_stop_btn,
    ]
    .align_y(Alignment::Center)
    .spacing(style::SPACING);

    // Brightness
    let brightness_row = row![
        text("Brightness").size(14).color(style::TEXT_PRIMARY),
        horizontal_space(),
        text(format!("{}%", app.config.ambient.brightness)).size(14).color(style::TEXT_SECONDARY),
    ]
    .align_y(Alignment::Center);

    let brightness_slider = slider(
        1u8..=100u8,
        app.config.ambient.brightness,
        Message::SetAmbientBrightness,
    )
    .width(Length::Fill);

    // Dim toggle
    let dim_row = row![
        text("Dim").size(14).color(style::TEXT_PRIMARY),
        horizontal_space(),
        toggler(app.config.ambient.dim)
            .on_toggle(Message::ToggleAmbientDim),
    ]
    .align_y(Alignment::Center)
    .spacing(10);

    column![
        header,
        iced::widget::rule::Rule::horizontal(1),
        brightness_row,
        brightness_slider,
        iced::widget::rule::Rule::horizontal(1),
        dim_row,
    ]
    .spacing(12)
    .into()
}
