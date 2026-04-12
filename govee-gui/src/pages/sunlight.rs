//! Sunlight mode settings page: preset, location or manual times, brightness,
//! segments, transition, and an optional simple-preset card.

use iced::widget::{button, column, container, horizontal_space, row, text, text_input, toggler};
use iced::{Alignment, Element, Length};
use crate::app::{App, Message};
use crate::style;

const PRESETS: &[&str] = &["coastal", "arctic", "ember", "simple"];

pub fn view(app: &App) -> Element<'_, Message> {
    let is_active = app.active_mode.as_deref() == Some("sunlight");

    let start_stop_btn = crate::widgets::slider_card::start_stop_button(
        is_active, "Sunlight", Message::StartSunlight,
    );

    let header = row![
        text("Sunlight").size(24).color(style::TEXT_PRIMARY),
        horizontal_space(),
        start_stop_btn,
    ]
    .align_y(Alignment::Center)
    .spacing(style::SPACING);

    // Preset picker
    let mut preset_row = row![].spacing(6);
    for &preset in PRESETS {
        let is_preset_active = app.config.sunlight.preset == preset;
        let btn = button(text(preset).size(12))
            .padding([6, 14])
            .on_press(Message::SetSunlightPreset(preset.to_string()))
            .style(style::pill_button(is_preset_active));
        preset_row = preset_row.push(btn);
    }

    let preset_card = container(
        column![
            text("Preset").size(14).color(style::TEXT_PRIMARY),
            preset_row,
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    // Location/Manual mode toggle
    let use_location = app.config.sunlight.use_location;
    let mode_toggle = row![
        button(text("Location").size(12))
            .padding([6, 14])
            .on_press(Message::SetSunlightUseLocation(true))
            .style(style::pill_button(use_location)),
        button(text("Manual times").size(12))
            .padding([6, 14])
            .on_press(Message::SetSunlightUseLocation(false))
            .style(style::pill_button(!use_location)),
    ]
    .spacing(6);

    let input_pair: Element<Message> = if use_location {
        column![
            labeled_input(
                "Latitude",
                &app.sunlight_inputs.lat,
                "-90.0 .. 90.0",
                Message::EditSunlightLat,
                app.sunlight_errors.lat.as_deref(),
                app.config.sunlight.lat.map(|v| format!("saved: {v}")),
            ),
            labeled_input(
                "Longitude",
                &app.sunlight_inputs.lon,
                "-180.0 .. 180.0",
                Message::EditSunlightLon,
                app.sunlight_errors.lon.as_deref(),
                app.config.sunlight.lon.map(|v| format!("saved: {v}")),
            ),
        ]
        .spacing(10)
        .into()
    } else {
        column![
            labeled_input(
                "Sunrise",
                &app.sunlight_inputs.sunrise,
                "HH:MM",
                Message::EditSunlightSunrise,
                app.sunlight_errors.sunrise.as_deref(),
                app.config.sunlight.sunrise.as_ref().map(|v| format!("saved: {v}")),
            ),
            labeled_input(
                "Sunset",
                &app.sunlight_inputs.sunset,
                "HH:MM",
                Message::EditSunlightSunset,
                app.sunlight_errors.sunset.as_deref(),
                app.config.sunlight.sunset.as_ref().map(|v| format!("saved: {v}")),
            ),
        ]
        .spacing(10)
        .into()
    };

    let location_card = container(
        column![
            text("Schedule source").size(14).color(style::TEXT_PRIMARY),
            mode_toggle,
            input_pair,
        ]
        .spacing(10),
    )
    .padding([16, 18])
    .style(style::card_style);

    let brightness_card = crate::widgets::slider_card::slider_card(
        "Brightness", app.config.sunlight.brightness, "%", 1..=100, Message::SetSunlightBrightness, Message::ApplySunlightSettings,
    );

    let segments_card = crate::widgets::slider_card::segments_card(
        app.config.sunlight.segments, Message::SetSunlightSegments, Message::ApplySunlightSettings,
    );

    let transition_card = crate::widgets::slider_card::slider_card(
        "Transition", app.config.sunlight.transition as u8, "min", 10..=120,
        |v| Message::SetSunlightTransition(v as u32), Message::ApplySunlightSettings,
    );

    let mut col = column![
        header,
        preset_card,
        location_card,
        brightness_card,
        segments_card,
        transition_card,
    ]
    .spacing(14);

    if app.config.sunlight.preset == "simple" {
        col = col.push(simple_preset_card(app));
    }

    col.into()
}

fn labeled_input<'a>(
    label: &'a str,
    value: &'a str,
    placeholder: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    error: Option<&'a str>,
    confirmation: Option<String>,
) -> Element<'a, Message> {
    let input = text_input(placeholder, value)
        .on_input(on_input)
        .on_submit(Message::SubmitSunlightCoords)
        .padding(8)
        .width(Length::Fill);

    let mut col = column![
        text(label).size(13).color(style::TEXT_SECONDARY),
        input,
    ]
    .spacing(4);

    if let Some(err) = error {
        col = col.push(text(err).size(12).color(style::INPUT_ERROR));
    } else if let Some(conf) = confirmation {
        col = col.push(text(conf).size(11).color(style::TEXT_MUTED));
    }

    col.into()
}

fn simple_preset_card(app: &App) -> Element<'_, Message> {
    let day_temp = app.config.sunlight.day_temp;
    let night_temp = app.config.sunlight.night_temp;
    let nb_on = app.config.sunlight.night_brightness.is_some();
    let nb_value = app.config.sunlight.night_brightness.unwrap_or(30);

    let day_slider = column![
        row![
            text("Day temp").size(14).color(style::TEXT_PRIMARY),
            horizontal_space(),
            text(format!("{day_temp}K")).size(14).color(style::TEXT_SECONDARY),
        ]
        .align_y(Alignment::Center),
        iced::widget::slider(2700u16..=6500u16, day_temp, Message::SetSunlightDayTemp)
            .step(100u16)
            .on_release(Message::ApplySunlightSettings)
            .width(Length::Fill),
    ]
    .spacing(6);

    let night_slider = column![
        row![
            text("Night temp").size(14).color(style::TEXT_PRIMARY),
            horizontal_space(),
            text(format!("{night_temp}K")).size(14).color(style::TEXT_SECONDARY),
        ]
        .align_y(Alignment::Center),
        iced::widget::slider(1800u16..=5000u16, night_temp, Message::SetSunlightNightTemp)
            .step(100u16)
            .on_release(Message::ApplySunlightSettings)
            .width(Length::Fill),
    ]
    .spacing(6);

    let override_row = row![
        text("Night brightness override").size(14).color(style::TEXT_PRIMARY),
        horizontal_space(),
        toggler(nb_on).on_toggle(Message::ToggleSunlightNightBrightnessOverride),
    ]
    .align_y(Alignment::Center);

    let mut body = column![
        text("Simple preset").size(14).color(style::TEXT_PRIMARY),
        day_slider,
        night_slider,
        override_row,
    ]
    .spacing(12);

    if nb_on {
        let nb_slider = column![
            row![
                text("Night brightness").size(14).color(style::TEXT_PRIMARY),
                horizontal_space(),
                text(format!("{nb_value}%")).size(14).color(style::TEXT_SECONDARY),
            ]
            .align_y(Alignment::Center),
            iced::widget::slider(1u8..=100u8, nb_value, Message::SetSunlightNightBrightness)
                .on_release(Message::ApplySunlightSettings)
                .width(Length::Fill),
        ]
        .spacing(6);
        body = body.push(nb_slider);
    }

    container(body)
        .padding([16, 18])
        .style(style::card_style)
        .into()
}
