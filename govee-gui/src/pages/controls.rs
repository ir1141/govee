//! Controls page: power toggle, brightness slider, color presets, and color temperature.

use iced::widget::{button, column, container, horizontal_space, row, slider, text, toggler};
use iced::{Alignment, Color, Element, Length};
use crate::app::{App, Message};
use crate::style;

const PRESET_COLORS: [(u8, u8, u8); 8] = [
    (255, 68, 68),   // red
    (255, 136, 0),   // orange
    (255, 204, 0),   // yellow
    (68, 255, 68),   // green
    (0, 204, 255),   // cyan
    (68, 68, 255),   // blue
    (204, 68, 255),  // purple
    (255, 68, 170),  // pink
];

pub fn view(app: &App) -> Element<'_, Message> {
    let (r, g, b) = app.color;

    // Power row
    let power_row = row![
        text("Controls").size(24).color(style::TEXT_PRIMARY),
        horizontal_space(),
        text("Power").size(14).color(style::TEXT_SECONDARY),
        toggler(app.power)
            .on_toggle(|_| Message::TogglePower),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let brightness_card = crate::widgets::slider_card::slider_card(
        "Brightness", app.brightness, "%", 1..=100, Message::SetBrightness, Message::SaveConfig,
    );

    // Color card
    let color_swatch = container(text(""))
        .width(48)
        .height(48)
        .style(move |_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(r, g, b))),
            border: iced::Border {
                radius: style::RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let mut preset_buttons = row![].spacing(8).align_y(Alignment::Center);
    for (pr, pg, pb) in PRESET_COLORS {
        let swatch = container(text(""))
            .width(32)
            .height(32)
            .style(move |_theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(pr, pg, pb))),
                border: iced::Border {
                    radius: style::RADIUS_SM.into(),
                    ..Default::default()
                },
                ..Default::default()
            });
        let btn = button(swatch)
            .padding(0)
            .on_press(Message::SetColor(pr, pg, pb));
        preset_buttons = preset_buttons.push(btn);
    }

    let color_card = container(
        column![
            row![
                text("Color").size(14).color(style::TEXT_PRIMARY),
                horizontal_space(),
                color_swatch,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            preset_buttons,
        ]
        .spacing(12),
    )
    .padding([16, 18])
    .style(style::card_style);

    // Temperature card
    let temp_card = container(
        column![
            row![
                text("Color Temperature").size(14).color(style::TEXT_PRIMARY),
                horizontal_space(),
                text(format!("{}K", app.color_temp)).size(14).color(style::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
            slider(2000u16..=9000u16, app.color_temp, Message::SetColorTemp)
                .on_release(Message::SaveConfig)
                .width(Length::Fill),
            row![
                text("2000K").size(11).color(style::TEXT_MUTED),
                horizontal_space(),
                text("9000K").size(11).color(style::TEXT_MUTED),
            ],
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    column![
        power_row,
        brightness_card,
        color_card,
        temp_card,
    ]
    .spacing(14)
    .into()
}
