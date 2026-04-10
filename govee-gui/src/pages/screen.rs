//! Screen capture settings page: FPS, brightness, and segment count controls.

use iced::widget::{column, container, horizontal_space, row, slider, text};
use iced::{Alignment, Element, Length};
use crate::app::{App, Message};
use crate::style;

pub fn view(app: &App) -> Element<'_, Message> {
    let is_active = app.active_mode.as_deref() == Some("screen");

    let start_stop_btn = crate::widgets::slider_card::start_stop_button(
        is_active, "Screen Capture", Message::StartScreen,
    );

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
                .on_release(Message::ApplyScreenSettings)
                .width(Length::Fill),
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    let brightness_card = crate::widgets::slider_card::slider_card(
        "Brightness", app.config.screen.brightness, "%", 1..=100, Message::SetScreenBrightness, Message::ApplyScreenSettings,
    );

    let segments_card = crate::widgets::slider_card::segments_card(
        app.config.screen.segments, Message::SetScreenSegments, Message::ApplyScreenSettings,
    );

    column![
        header,
        fps_card,
        brightness_card,
        segments_card,
    ]
    .spacing(14)
    .into()
}
