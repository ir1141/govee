//! Themes page: filterable grid of theme cards with palette previews and one-click activation.

use iced::widget::{button, column, container, horizontal_space, row, scrollable, text};
use iced::{Border, Color, Element, Length, Shadow, Vector};
use iced::widget::Row;
use govee_themes::ThemeKind;
use crate::app::{App, Message};
use crate::style;

fn rgb_color(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

fn categories() -> Vec<&'static str> {
    let mut cats = vec!["all"];
    cats.extend_from_slice(govee_themes::BUILTIN_CATEGORIES);
    cats.push("custom");
    cats
}
const CARD_WIDTH: f32 = 150.0;
const COLOR_BAND_HEIGHT: f32 = 36.0;
const CARDS_PER_ROW: usize = 4;

pub fn view(app: &App) -> Element<'_, Message> {
    // Header row
    let categories = categories();
    let mut tab_row = row![].spacing(6);
    for &cat in &categories {
        let is_active = app.theme_filter == cat;
        let btn = button(text(cat).size(12))
            .padding([6, 14])
            .on_press(Message::ThemeFilterChanged(cat.to_string()))
            .style(style::pill_button(is_active));
        tab_row = tab_row.push(btn);
    }

    let header = row![
        text("Themes").size(24).color(style::TEXT_PRIMARY),
        horizontal_space(),
        tab_row,
    ]
    .align_y(iced::Alignment::Center)
    .spacing(style::SPACING);

    // Stop button (shown when a theme is active)
    let mut header_col = column![header].spacing(10);

    if app.active_theme.is_some() {
        let stop_btn = button(text("■ Stop").size(12).color(Color::WHITE))
            .padding([6, 14])
            .on_press(Message::StopMode)
            .style(style::danger_action_button());
        header_col = header_col.push(stop_btn);
    }

    let mut content_col = column![].spacing(16);

    // Determine which categories to show
    let is_custom = |c: &str| !govee_themes::BUILTIN_CATEGORIES.contains(&c);

    let display_cats: Vec<String> = if app.theme_filter == "all" {
        let mut cats: Vec<String> = govee_themes::BUILTIN_CATEGORIES.iter().map(|s: &&str| s.to_string()).collect();
        for t in &app.themes {
            if is_custom(&t.category) && !cats.iter().any(|c| c == &t.category) {
                cats.push(t.category.clone());
            }
        }
        cats
    } else if app.theme_filter == "custom" {
        let mut cats: Vec<String> = Vec::new();
        for t in &app.themes {
            if is_custom(&t.category) && !cats.iter().any(|c| c == &t.category) {
                cats.push(t.category.clone());
            }
        }
        cats
    } else {
        vec![app.theme_filter.clone()]
    };

    // Build card grid per category
    for cat in display_cats.iter() {
        let cat_themes: Vec<&govee_themes::ThemeDef> = app
            .themes
            .iter()
            .filter(|t| t.category == cat.as_str())
            .collect();

        if cat_themes.is_empty() {
            continue;
        }

        let cat_label = text(cat.to_uppercase())
            .size(12)
            .color(style::TEXT_MUTED);

        let mut grid = column![].spacing(8);
        let mut current_row: Vec<Element<Message>> = vec![];

        for (i, theme) in cat_themes.iter().enumerate() {
            let is_active = app.active_theme.as_deref() == Some(&theme.name);
            current_row.push(theme_card(theme, is_active));
            if current_row.len() == CARDS_PER_ROW || i == cat_themes.len() - 1 {
                let r = Row::with_children(current_row).spacing(8);
                grid = grid.push(r);
                current_row = vec![];
            }
        }

        content_col = content_col.push(
            column![cat_label, grid].spacing(6)
        );
    }

    column![
        header_col,
        scrollable(
            container(content_col)
                .width(Length::Fill)
                .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 20.0, left: 0.0 }),
        ),
    ]
    .spacing(16)
    .into()
}

/// Sample up to 4 representative colors from a behavior's palette for card previews.
fn extract_preview_colors(behavior: &govee_themes::Behavior) -> Vec<Color> {
    use govee_themes::themes::palette_sample;
    use govee_themes::Behavior;

    match behavior {
        Behavior::Heat { palette, .. }
        | Behavior::Wave { palette, .. }
        | Behavior::Breathe { palette, .. }
        | Behavior::Drift { palette, .. }
        | Behavior::Progression { palette, .. } => {
            (0..4).map(|i| {
                let (r, g, b) = palette_sample(palette, i as f64 / 3.0);
                rgb_color(r, g, b)
            }).collect()
        }
        Behavior::Flash { base_palette, .. } => {
            (0..4).map(|i| {
                let (r, g, b) = palette_sample(base_palette, i as f64 / 3.0);
                rgb_color(r, g, b)
            }).collect()
        }
        Behavior::Particles { bg, palette, .. } => {
            let mut colors = vec![rgb_color(bg.0, bg.1, bg.2)];
            for i in 0..3 {
                let (r, g, b) = palette_sample(palette, i as f64 / 2.0);
                colors.push(rgb_color(r, g, b));
            }
            colors
        }
        Behavior::Twinkle { bg, colors, .. } => {
            let mut out = vec![rgb_color(bg.0, bg.1, bg.2)];
            for &(r, g, b) in colors.iter().take(3) {
                out.push(rgb_color(r, g, b));
            }
            out
        }
        Behavior::GradientWave { color_a, color_b, .. } => {
            vec![
                rgb_color(color_a.0, color_a.1, color_a.2),
                rgb_color(color_b.0, color_b.1, color_b.2),
            ]
        }
        Behavior::Strobe { colors, .. } | Behavior::Alternating { colors, .. } => {
            colors.iter().take(4).map(|&(r, g, b)| {
                rgb_color(r, g, b)
            }).collect()
        }
        Behavior::HueRotate { saturation, value, .. } => {
            (0..4).map(|i| {
                let (r, g, b) = govee_themes::themes::hsv_to_rgb(i as f64 / 4.0, *saturation, *value);
                rgb_color(r, g, b)
            }).collect()
        }
        Behavior::RadiatePulse { color, .. } => {
            vec![rgb_color(color.0, color.1, color.2)]
        }
    }
}

/// Render a single theme card with color band preview and active indicator.
fn theme_card<'a>(theme: &govee_themes::ThemeDef, is_active: bool) -> Element<'a, Message> {
    let name = theme.name.clone();

    // Color band — show palette preview
    let band: Element<'a, Message> = match &theme.kind {
        ThemeKind::Solid { color } => {
            let (r, g, b) = *color;
            let c = rgb_color(r, g, b);
            container(iced::widget::Space::new(Length::Fill, COLOR_BAND_HEIGHT))
                .width(Length::Fill)
                .style(move |_theme| container::Style {
                    background: Some(iced::Background::Color(c)),
                    ..Default::default()
                })
                .into()
        }
        ThemeKind::Animated { behavior, .. } => {
            let colors = extract_preview_colors(behavior);
            let segments: Vec<Element<'a, Message>> = colors.into_iter().map(|c| {
                container(iced::widget::Space::new(Length::Fill, COLOR_BAND_HEIGHT))
                    .width(Length::Fill)
                    .style(move |_theme| container::Style {
                        background: Some(iced::Background::Color(c)),
                        ..Default::default()
                    })
                    .into()
            }).collect();
            Row::with_children(segments).spacing(0).into()
        }
    };

    // Info section
    let mut info_col = column![
        text(theme.name.clone()).size(13).color(style::TEXT_PRIMARY)
    ]
    .spacing(2);

    if matches!(&theme.kind, ThemeKind::Animated { .. }) {
        info_col = info_col.push(
            text("Animated").size(11).color(style::TEXT_MUTED),
        );
    }

    if is_active {
        info_col = info_col.push(
            text("● Active").size(11).color(style::ACCENT_LIGHT),
        );
    }

    let inner = column![band, container(info_col).padding([8, 10])]
        .spacing(0);

    let border_color = if is_active { style::ACCENT } else { Color::TRANSPARENT };

    let active_shadow = if is_active {
        Shadow {
            color: Color { a: 0.3, ..style::ACCENT },
            offset: Vector::new(0.0, 0.0),
            blur_radius: 12.0,
        }
    } else {
        Shadow::default()
    };

    button(inner)
        .width(CARD_WIDTH)
        .padding(0)
        .on_press(Message::ApplyTheme(name))
        .style(move |_theme, status| {
            let bg = match status {
                button::Status::Hovered => style::SURFACE_HOVER,
                _ => style::SURFACE,
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: style::TEXT_PRIMARY,
                border: Border {
                    color: border_color,
                    width: if is_active { 2.0 } else { 0.0 },
                    radius: style::RADIUS_LG.into(),
                },
                shadow: active_shadow,
            }
        })
        .into()
}
