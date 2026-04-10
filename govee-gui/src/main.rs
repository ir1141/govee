//! Iced-based GUI frontend for Govee LED strip control.

mod app;
mod config;
mod pages;
mod style;
mod subprocess;
mod widgets;

use iced::{Size, Theme};

fn main() -> iced::Result {
    iced::application("Govee", app::App::update, app::App::view)
        .font(include_bytes!("../fonts/Inter.ttf"))
        .theme(|_| {
            Theme::custom(
                "Govee Dark".into(),
                iced::theme::Palette {
                    background: style::BG,
                    text: style::TEXT_PRIMARY,
                    primary: style::ACCENT,
                    success: style::SUCCESS,
                    danger: style::DANGER,
                },
            )
        })
        .window_size(Size::new(900.0, 620.0))
        .subscription(app::App::subscription)
        .run_with(app::App::new)
}
