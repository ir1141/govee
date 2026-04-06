use govee_lan::DeviceInfo;
use iced::widget::{button, column, container, text};
use iced::{Background, Border, Color, Element, Length};
use crate::app::{Message, Page};
use crate::style;

fn nav_button(label: &'static str, page: Page, active: bool) -> Element<'static, Message> {
    let btn = button(text(label))
        .width(Length::Fill)
        .on_press(Message::Navigate(page));

    if active {
        let accent_bg = Color {
            r: style::ACCENT.r,
            g: style::ACCENT.g,
            b: style::ACCENT.b,
            a: 0.2,
        };
        container(btn)
            .width(Length::Fill)
            .style(move |_theme| container::Style {
                background: Some(Background::Color(accent_bg)),
                border: Border {
                    radius: style::RADIUS.into(),
                    color: Color::TRANSPARENT,
                    width: 0.0,
                },
                ..Default::default()
            })
            .into()
    } else {
        container(btn).width(Length::Fill).into()
    }
}

pub fn view(current_page: Page, device_label: &str, devices: &[DeviceInfo], current_device_ip: Option<&str>) -> Element<'static, Message> {
    let device_label: String = device_label.to_string();

    let header = column![
        text("GOVEE").size(18),
        text(device_label).size(12),
    ]
    .spacing(4.0);

    let nav = column![
        nav_button("Controls", Page::Controls, current_page == Page::Controls),
        nav_button("Themes", Page::Themes, current_page == Page::Themes),
        nav_button("Screen", Page::Screen, current_page == Page::Screen),
        nav_button("Audio", Page::Audio, current_page == Page::Audio),
        nav_button("Ambient", Page::Ambient, current_page == Page::Ambient),
    ]
    .spacing(4.0);

    let mut content_col = column![header, nav].spacing(16.0).padding(12.0);

    if devices.len() > 1 {
        let current_ip: Option<String> = current_device_ip.map(|s| s.to_string());
        let mut device_list = column![text("Devices").size(11).color(style::TEXT_SECONDARY)]
            .spacing(2.0);

        for (idx, dev) in devices.iter().enumerate() {
            let is_selected = current_ip.as_deref() == Some(dev.ip.as_str());
            let label = format!("{} {}", dev.sku, dev.ip);
            let btn = button(text(label).size(11))
                .width(Length::Fill)
                .on_press(Message::SelectDevice(idx));

            let entry: Element<'static, Message> = if is_selected {
                let accent_bg = Color {
                    r: style::ACCENT.r,
                    g: style::ACCENT.g,
                    b: style::ACCENT.b,
                    a: 0.2,
                };
                container(btn)
                    .width(Length::Fill)
                    .style(move |_theme| container::Style {
                        background: Some(Background::Color(accent_bg)),
                        border: Border {
                            radius: style::RADIUS.into(),
                            color: Color::TRANSPARENT,
                            width: 0.0,
                        },
                        ..Default::default()
                    })
                    .into()
            } else {
                container(btn).width(Length::Fill).into()
            };

            device_list = device_list.push(entry);
        }

        content_col = content_col.push(device_list);
    }

    container(content_col)
        .width(Length::Fixed(style::SIDEBAR_WIDTH))
        .height(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(style::SIDEBAR_BG)),
            border: Border {
                radius: 0.0.into(),
                color: Color::TRANSPARENT,
                width: 0.0,
            },
            ..Default::default()
        })
        .into()
}
