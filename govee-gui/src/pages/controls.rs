use iced::widget::{button, column, container, horizontal_space, row, slider, text, toggler};
use iced::{Alignment, Color, Element, Length};
use crate::app::{App, Message};

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
        text("Controls").size(22),
        horizontal_space(),
        text("Power").size(14),
        toggler(app.power)
            .on_toggle(|_| Message::TogglePower),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    // Brightness section
    let brightness_row = row![
        text("Brightness").size(14),
        horizontal_space(),
        text(format!("{}%", app.brightness)).size(14),
    ]
    .align_y(Alignment::Center);

    let brightness_slider = slider(
        1u8..=100u8,
        app.brightness,
        Message::SetBrightness,
    )
    .width(Length::Fill);

    // Color section
    let color_swatch = container(text(""))
        .width(40)
        .height(40)
        .style(move |_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(r, g, b))),
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let mut preset_buttons = row![].spacing(6).align_y(Alignment::Center);
    for (pr, pg, pb) in PRESET_COLORS {
        let swatch = container(text(""))
            .width(28)
            .height(28)
            .style(move |_theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(pr, pg, pb))),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });
        let btn = button(swatch)
            .padding(0)
            .on_press(Message::SetColor(pr, pg, pb));
        preset_buttons = preset_buttons.push(btn);
    }

    let color_row = row![
        text("Color").size(14),
        horizontal_space(),
        color_swatch,
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    // Color temperature section
    let temp_row = row![
        text("Color Temperature").size(14),
        horizontal_space(),
        text(format!("{}K", app.color_temp)).size(14),
    ]
    .align_y(Alignment::Center);

    let temp_slider = slider(
        2000u16..=9000u16,
        app.color_temp,
        Message::SetColorTemp,
    )
    .width(Length::Fill);

    let temp_labels = row![
        text("2000K").size(11),
        horizontal_space(),
        text("9000K").size(11),
    ];

    column![
        power_row,
        iced::widget::rule::Rule::horizontal(1),
        brightness_row,
        brightness_slider,
        iced::widget::rule::Rule::horizontal(1),
        color_row,
        preset_buttons,
        iced::widget::rule::Rule::horizontal(1),
        temp_row,
        temp_slider,
        temp_labels,
    ]
    .spacing(12)
    .into()
}
