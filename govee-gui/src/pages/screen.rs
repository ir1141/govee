use iced::widget::{button, column, container, horizontal_space, row, slider, text};
use iced::{Alignment, Color, Element, Length};
use crate::app::{App, Message};
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let is_active = app.active_mode.as_deref() == Some("screen");

    let start_stop_btn = if is_active {
        button(text("■ Stop Screen Capture").size(13).color(Color::WHITE))
            .padding([8, 20])
            .on_press(Message::StopMode)
            .style(style::danger_action_button())
    } else {
        button(text("▶ Start Screen Capture").size(13).color(Color::WHITE))
            .padding([8, 20])
            .on_press(Message::StartScreen)
            .style(style::accent_action_button())
    };

    let header = row![
        text("Screen Capture").size(24).color(style::TEXT_PRIMARY),
        horizontal_space(),
        start_stop_btn,
    ]
    .align_y(Alignment::Center)
    .spacing(style::SPACING);

    // FPS card
    let fps_card = container(
        column![
            row![
                text("FPS").size(14).color(style::TEXT_PRIMARY),
                horizontal_space(),
                text(format!("{}", app.config.screen.fps)).size(14).color(style::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
            slider(1u32..=30u32, app.config.screen.fps, Message::SetScreenFps)
                .width(Length::Fill),
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
                text(format!("{}%", app.config.screen.brightness)).size(14).color(style::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
            slider(1u8..=100u8, app.config.screen.brightness, Message::SetScreenBrightness)
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
                text(format!("{}", app.config.screen.segments)).size(14).color(style::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
            slider(1u8..=15u8, app.config.screen.segments as u8, |v| Message::SetScreenSegments(v as usize))
                .width(Length::Fill),
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    column![
        header,
        fps_card,
        brightness_card,
        segments_card,
    ]
    .spacing(14)
    .into()
}
