mod app;
mod config;
mod pages;
mod style;
mod widgets;

use iced::{Size, Theme};

fn main() -> iced::Result {
    iced::application("Govee", app::App::update, app::App::view)
        .theme(|_| {
            Theme::custom(
                "Govee Dark".into(),
                iced::theme::Palette {
                    background: style::BG,
                    text: style::TEXT_PRIMARY,
                    primary: style::ACCENT,
                    success: style::SUCCESS,
                    danger: iced::Color::from_rgb(1.0, 0.3, 0.3),
                },
            )
        })
        .window_size(Size::new(900.0, 600.0))
        .run_with(app::App::new)
}
