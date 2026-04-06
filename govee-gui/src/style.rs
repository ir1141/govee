use iced::widget::{button, container};
use iced::{Background, Border, Color, Shadow, Vector};

// ── Palette ─────────────────────────────────────────────────────────────────

pub const BG: Color = Color::from_rgb(0.07, 0.07, 0.09);
pub const SIDEBAR_BG: Color = Color::from_rgb(0.09, 0.09, 0.12);
pub const SURFACE: Color = Color::from_rgb(0.13, 0.13, 0.18);
pub const SURFACE_HOVER: Color = Color::from_rgb(0.16, 0.16, 0.22);
pub const ACCENT: Color = Color::from_rgb(0.55, 0.35, 0.95);
pub const ACCENT_LIGHT: Color = Color::from_rgb(0.70, 0.58, 1.0);
pub const ACCENT_DIM: Color = Color {
    r: 0.55,
    g: 0.35,
    b: 0.95,
    a: 0.15,
};
pub const TEXT_PRIMARY: Color = Color::from_rgb(0.90, 0.90, 0.96);
pub const TEXT_SECONDARY: Color = Color::from_rgb(0.55, 0.55, 0.68);
pub const TEXT_MUTED: Color = Color::from_rgb(0.42, 0.42, 0.56);
pub const SUCCESS: Color = Color::from_rgb(0.30, 0.88, 0.55);
pub const DANGER: Color = Color::from_rgb(0.90, 0.25, 0.25);
pub const DANGER_HOVER: Color = Color::from_rgb(0.80, 0.20, 0.20);

// ── Spacing & sizing ───────────────────────────────────────────────────────

pub const SPACING: f32 = 10.0;
pub const RADIUS: f32 = 12.0;
pub const RADIUS_SM: f32 = 8.0;
pub const RADIUS_LG: f32 = 16.0;
pub const SIDEBAR_WIDTH: f32 = 220.0;

// ── Shadows ────────────────────────────────────────────────────────────────

const CARD_SHADOW: Shadow = Shadow {
    color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
    offset: Vector::new(0.0, 2.0),
    blur_radius: 8.0,
};

const SIDEBAR_SHADOW: Shadow = Shadow {
    color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
    offset: Vector::new(2.0, 0.0),
    blur_radius: 12.0,
};

// ── Reusable container styles ──────────────────────────────────────────────

pub fn card_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        border: Border {
            radius: RADIUS_LG.into(),
            color: Color::TRANSPARENT,
            width: 0.0,
        },
        shadow: CARD_SHADOW,
        text_color: Some(TEXT_PRIMARY),
    }
}

pub fn sidebar_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SIDEBAR_BG)),
        border: Border {
            radius: 0.0.into(),
            color: Color::TRANSPARENT,
            width: 0.0,
        },
        shadow: SIDEBAR_SHADOW,
        text_color: Some(TEXT_PRIMARY),
    }
}

pub fn status_bar_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SIDEBAR_BG)),
        border: Border {
            radius: 0.0.into(),
            color: Color {
                r: SURFACE.r,
                g: SURFACE.g,
                b: SURFACE.b,
                a: 0.5,
            },
            width: 1.0,
        },
        shadow: Shadow::default(),
        text_color: Some(TEXT_PRIMARY),
    }
}

// ── Reusable button styles ─────────────────────────────────────────────────

pub fn nav_button_style(active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = if active {
            ACCENT_DIM
        } else {
            match status {
                button::Status::Hovered => Color { a: 0.08, ..ACCENT_DIM },
                _ => Color::TRANSPARENT,
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if active { ACCENT_LIGHT } else { TEXT_SECONDARY },
            border: Border {
                radius: RADIUS.into(),
                color: Color::TRANSPARENT,
                width: 0.0,
            },
            ..Default::default()
        }
    }
}

pub fn pill_button(active: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = if active {
            ACCENT
        } else {
            match status {
                button::Status::Hovered => SURFACE_HOVER,
                _ => SURFACE,
            }
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if active { Color::WHITE } else { TEXT_SECONDARY },
            border: Border {
                radius: RADIUS_SM.into(),
                color: Color::TRANSPARENT,
                width: 0.0,
            },
            ..Default::default()
        }
    }
}

pub fn action_button(base: Color, hover: Color) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = match status {
            button::Status::Hovered => hover,
            _ => base,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: Color::WHITE,
            border: Border {
                radius: RADIUS.into(),
                color: Color::TRANSPARENT,
                width: 0.0,
            },
            ..Default::default()
        }
    }
}

pub fn accent_action_button() -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    let hover = Color {
        r: ACCENT.r + 0.08,
        g: ACCENT.g + 0.08,
        b: ACCENT.b + 0.08,
        a: 1.0,
    };
    action_button(ACCENT, hover)
}

pub fn danger_action_button() -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    action_button(DANGER, DANGER_HOVER)
}
