use iced::widget::{button, column, container, horizontal_space, row, scrollable, text};
use iced::{Border, Color, Element, Length};
use iced::widget::Row;
use govee_lan::ThemeKind;
use crate::app::{App, Message};
use crate::style;

const CATEGORIES: &[&str] = &["all", "static", "nature", "vibes", "functional", "seasonal", "custom"];
const CARD_WIDTH: f32 = 140.0;
const COLOR_BAND_HEIGHT: f32 = 32.0;
const CARDS_PER_ROW: usize = 4;

pub fn view(app: &App) -> Element<'_, Message> {
    // ── Header row ─────────────────────────────────────────────────────────
    let title = text("Themes").size(22).color(style::TEXT_PRIMARY);

    let mut tab_row = row![].spacing(4);
    for &cat in CATEGORIES {
        let is_active = app.theme_filter == cat;
        let btn = button(text(cat).size(12))
            .padding([4, 10])
            .on_press(Message::ThemeFilterChanged(cat.to_string()))
            .style(move |_theme, status| {
                let base = button::Style {
                    background: Some(iced::Background::Color(if is_active {
                        style::ACCENT
                    } else {
                        style::SURFACE
                    })),
                    text_color: if is_active {
                        Color::WHITE
                    } else {
                        style::TEXT_SECONDARY
                    },
                    border: Border {
                        radius: style::RADIUS.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                match status {
                    button::Status::Hovered => button::Style {
                        background: Some(iced::Background::Color(if is_active {
                            style::ACCENT
                        } else {
                            Color {
                                r: style::SURFACE.r + 0.05,
                                g: style::SURFACE.g + 0.05,
                                b: style::SURFACE.b + 0.05,
                                a: 1.0,
                            }
                        })),
                        ..base
                    },
                    _ => base,
                }
            });
        tab_row = tab_row.push(btn);
    }

    let header = row![title, horizontal_space(), tab_row]
        .align_y(iced::Alignment::Center)
        .spacing(style::SPACING);

    // ── Stop button (shown when a theme is active) ─────────────────────────
    let mut content_col = column![header].spacing(16);

    if app.active_theme.is_some() {
        let stop_btn = button(text("■ Stop").size(12).color(Color::WHITE))
            .padding([4, 12])
            .on_press(Message::StopMode)
            .style(|_theme, status| button::Style {
                background: Some(iced::Background::Color(match status {
                    button::Status::Hovered => Color::from_rgb(0.8, 0.2, 0.2),
                    _ => Color::from_rgb(0.6, 0.15, 0.15),
                })),
                text_color: Color::WHITE,
                border: Border {
                    radius: style::RADIUS.into(),
                    ..Default::default()
                },
                ..Default::default()
            });
        content_col = content_col.push(stop_btn);
    }

    // ── Determine which categories to show ─────────────────────────────────
    let is_custom = |c: &str| !govee_lan::BUILTIN_CATEGORIES.contains(&c);

    let display_cats: Vec<String> = if app.theme_filter == "all" {
        let mut cats: Vec<String> = govee_lan::BUILTIN_CATEGORIES.iter().map(|s: &&str| s.to_string()).collect();
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

    // ── Build card grid per category ────────────────────────────────────────
    for cat in display_cats.iter() {
        let cat_themes: Vec<&govee_lan::ThemeDef> = app
            .themes
            .iter()
            .filter(|t| t.category == cat.as_str())
            .collect();

        if cat_themes.is_empty() {
            continue;
        }

        let cat_label = text(cat.to_uppercase())
            .size(11)
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

    scrollable(
        container(content_col)
            .width(Length::Fill)
            .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 20.0, left: 0.0 }),
    )
    .into()
}

fn extract_preview_colors(behavior: &govee_lan::Behavior) -> Vec<Color> {
    use govee_lan::themes::palette_sample;
    use govee_lan::Behavior;

    match behavior {
        Behavior::Heat { palette, .. }
        | Behavior::Wave { palette, .. }
        | Behavior::Breathe { palette, .. }
        | Behavior::Drift { palette, .. }
        | Behavior::Progression { palette, .. } => {
            (0..4).map(|i| {
                let (r, g, b) = palette_sample(palette, i as f64 / 3.0);
                Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
            }).collect()
        }
        Behavior::Flash { base_palette, .. } => {
            (0..4).map(|i| {
                let (r, g, b) = palette_sample(base_palette, i as f64 / 3.0);
                Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
            }).collect()
        }
        Behavior::Particles { bg, palette, .. } => {
            let mut colors = vec![Color::from_rgb(bg.0 as f32 / 255.0, bg.1 as f32 / 255.0, bg.2 as f32 / 255.0)];
            for i in 0..3 {
                let (r, g, b) = palette_sample(palette, i as f64 / 2.0);
                colors.push(Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0));
            }
            colors
        }
        Behavior::Twinkle { bg, colors, .. } => {
            let mut out = vec![Color::from_rgb(bg.0 as f32 / 255.0, bg.1 as f32 / 255.0, bg.2 as f32 / 255.0)];
            for &(r, g, b) in colors.iter().take(3) {
                out.push(Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0));
            }
            out
        }
        Behavior::GradientWave { color_a, color_b, .. } => {
            vec![
                Color::from_rgb(color_a.0 as f32 / 255.0, color_a.1 as f32 / 255.0, color_a.2 as f32 / 255.0),
                Color::from_rgb(color_b.0 as f32 / 255.0, color_b.1 as f32 / 255.0, color_b.2 as f32 / 255.0),
            ]
        }
        Behavior::Strobe { colors, .. } | Behavior::Alternating { colors, .. } => {
            colors.iter().take(4).map(|&(r, g, b)| {
                Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
            }).collect()
        }
        Behavior::HueRotate { saturation, value, .. } => {
            (0..4).map(|i| {
                let (r, g, b) = govee_lan::themes::hsv_to_rgb(i as f64 / 4.0, *saturation, *value);
                Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
            }).collect()
        }
        Behavior::RadiatePulse { color, .. } => {
            vec![Color::from_rgb(color.0 as f32 / 255.0, color.1 as f32 / 255.0, color.2 as f32 / 255.0)]
        }
    }
}

fn theme_card<'a>(theme: &govee_lan::ThemeDef, is_active: bool) -> Element<'a, Message> {
    let name = theme.name.clone();

    // Color band — show palette preview
    let band: Element<'a, Message> = match &theme.kind {
        ThemeKind::Solid { color } => {
            let (r, g, b) = *color;
            let c = Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
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

    let inner = column![band, container(info_col).padding([6, 8])]
        .spacing(0);

    let border_color = if is_active { style::ACCENT } else { style::SURFACE };

    button(inner)
        .width(CARD_WIDTH)
        .padding(0)
        .on_press(Message::ApplyTheme(name))
        .style(move |_theme, status| {
            let bg = match status {
                button::Status::Hovered => Color {
                    r: style::SURFACE.r + 0.05,
                    g: style::SURFACE.g + 0.05,
                    b: style::SURFACE.b + 0.05,
                    a: 1.0,
                },
                _ => style::SURFACE,
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: style::TEXT_PRIMARY,
                border: Border {
                    color: border_color,
                    width: if is_active { 2.0 } else { 1.0 },
                    radius: style::RADIUS.into(),
                },
                ..Default::default()
            }
        })
        .into()
}
