mod style;

use iced::widget::{column, container, text};
use iced::{Element, Length, Size, Task, Theme};

fn main() -> iced::Result {
    iced::application("Govee", App::update, App::view)
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
        .run_with(App::new)
}

struct App;

#[derive(Debug, Clone)]
enum Message {}

impl App {
    fn new() -> (Self, Task<Message>) {
        (App, Task::none())
    }

    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        let content = column![text("Govee GUI").size(24),].spacing(style::SPACING);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }
}
