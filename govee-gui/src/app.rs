use govee_lan::DeviceInfo;
use iced::widget::{column, container, row};
use iced::{Element, Length, Task};
use crate::config::GuiConfig;
use crate::pages;
use crate::widgets::{sidebar, status_bar};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Controls,
    Themes,
    Screen,
    Audio,
    Ambient,
}

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Page),
}

pub struct App {
    pub page: Page,
    pub device: Option<DeviceInfo>,
    pub devices: Vec<DeviceInfo>,
    pub config: GuiConfig,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let config = GuiConfig::load();
        let page = match config.general.last_page.as_str() {
            "themes" => Page::Themes,
            "screen" => Page::Screen,
            "audio" => Page::Audio,
            "ambient" => Page::Ambient,
            _ => Page::Controls,
        };
        (
            Self {
                page,
                device: None,
                devices: vec![],
                config,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(page) => {
                self.page = page;
                self.config.general.last_page = match page {
                    Page::Controls => "controls",
                    Page::Themes => "themes",
                    Page::Screen => "screen",
                    Page::Audio => "audio",
                    Page::Ambient => "ambient",
                }
                .into();
                self.config.save();
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let device_label = self
            .device
            .as_ref()
            .map(|d| format!("{} • {}", d.sku, d.ip))
            .unwrap_or_else(|| "No device".into());

        let sidebar = sidebar::view(self.page, &device_label);

        let page_content: Element<Message> = match self.page {
            Page::Controls => pages::controls::view(),
            Page::Themes => pages::themes::view(),
            Page::Screen => pages::screen::view(),
            Page::Audio => pages::audio::view(),
            Page::Ambient => pages::ambient::view(),
        };

        let content = container(page_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20);

        let mode_label = "Idle";
        let main = column![
            row![sidebar, content].height(Length::Fill),
            status_bar::view(self.device.is_some(), mode_label),
        ];

        container(main)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
