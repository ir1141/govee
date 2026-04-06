use govee_lan::*;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{RUNNING, ctrlc_setup, resolve_or_exit};

// ── Types ───────────────────────────────────────────────────────────────────

pub type Rgb = (u8, u8, u8);

/// Palette anchor: position 0.0–1.0 with an RGB color.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PA {
    pub pos: f64,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Wave parameters for the Wave behavior.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WaveParam {
    pub time_speed: f64,
    pub spatial_freq: f64,
    pub phase_offset: f64,
}

// ── Helpers for compact builtin definitions ─────────────────────────────────

pub fn pa(pos: f64, r: u8, g: u8, b: u8) -> PA {
    PA { pos, r, g, b }
}

pub fn wp(time_speed: f64, spatial_freq: f64, phase_offset: f64) -> WaveParam {
    WaveParam { time_speed, spatial_freq, phase_offset }
}

// ── Color utilities ─────────────────────────────────────────────────────────

fn palette_sample(anchors: &[PA], t: f64) -> Rgb {
    if anchors.is_empty() {
        return (0, 0, 0);
    }
    if anchors.len() == 1 {
        return (anchors[0].r, anchors[0].g, anchors[0].b);
    }
    let t = t.clamp(0.0, 1.0);
    if t <= anchors[0].pos {
        return (anchors[0].r, anchors[0].g, anchors[0].b);
    }
    for i in 0..anchors.len() - 1 {
        let a = &anchors[i];
        let b = &anchors[i + 1];
        if t <= b.pos {
            let f = if (b.pos - a.pos).abs() < 1e-9 {
                0.0
            } else {
                ((t - a.pos) / (b.pos - a.pos)).clamp(0.0, 1.0)
            };
            return (
                (a.r as f64 + (b.r as f64 - a.r as f64) * f) as u8,
                (a.g as f64 + (b.g as f64 - a.g as f64) * f) as u8,
                (a.b as f64 + (b.b as f64 - a.b as f64) * f) as u8,
            );
        }
    }
    let l = anchors.last().unwrap();
    (l.r, l.g, l.b)
}

fn lerp_rgb(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    (
        (a.0 as f64 + (b.0 as f64 - a.0 as f64) * t) as u8,
        (a.1 as f64 + (b.1 as f64 - a.1 as f64) * t) as u8,
        (a.2 as f64 + (b.2 as f64 - a.2 as f64) * t) as u8,
    )
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> Rgb {
    let h = (h.fract() + 1.0).fract() * 6.0;
    let f = h.fract();
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let tv = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match h as u32 {
        0 => (v, tv, p),
        1 => (q, v, p),
        2 => (p, v, tv),
        3 => (p, q, v),
        4 => (tv, p, v),
        _ => (v, p, q),
    };
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

// ── Behavior engine ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Delay {
    Fixed(u64),
    Random(u64, u64),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Behavior {
    Heat {
        palette: Vec<PA>,
        volatility: f64,
        spark_chance: f64,
        spark_boost: f64,
        dim_chance: f64,
        dim_range: (f64, f64),
        diffusion: f64,
    },
    Wave {
        palette: Vec<PA>,
        waves: Vec<WaveParam>,
        weights: Vec<f64>,
    },
    Breathe {
        palette: Vec<PA>,
        speed: f64,
        power: u32,
    },
    Flash {
        base_palette: Vec<PA>,
        flash_palette: Vec<PA>,
        decay: f64,
        flash_chance: f64,
        spread: (usize, usize),
        base_wave_speed: f64,
        base_spatial_freq: f64,
        flash_threshold: f64,
    },
    Particles {
        bg: Rgb,
        palette: Vec<PA>,
        speed: f64,
        spawn_chance: f64,
        bright_chance: f64,
    },
    Twinkle {
        bg: Rgb,
        colors: Vec<Rgb>,
        on_chance: f64,
        fade_speed: f64,
    },
    HueRotate {
        speed: f64,
        saturation: f64,
        value: f64,
    },
    GradientWave {
        color_a: Rgb,
        color_b: Rgb,
        speed: f64,
    },
    Strobe {
        colors: Vec<Rgb>,
        cycle_speed: f64,
        flash_chance: f64,
    },
    Alternating {
        colors: Vec<Rgb>,
        sparkle: Rgb,
        sparkle_chance: f64,
        shift_speed: f64,
    },
    Drift {
        palette: Vec<PA>,
        speed: f64,
    },
    RadiatePulse {
        color: Rgb,
        speed: f64,
        width: f64,
    },
    Progression {
        palette: Vec<PA>,
        duration_secs: f64,
        spatial_spread: f64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ThemeKind {
    Solid { color: Rgb },
    Animated { behavior: Behavior, delay: Delay },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeDef {
    pub name: String,
    pub category: String,
    pub kind: ThemeKind,
}

// ── Theme registry ──────────────────────────────────────────────────────────

pub fn get_theme(name: &str) -> Option<ThemeDef> {
    crate::theme_loader::load_all_themes()
        .into_iter()
        .find(|t| t.name == name)
}

pub fn theme_list_display() -> String {
    let themes = crate::theme_loader::load_all_themes();
    let pairs: Vec<(&str, &str)> = themes.iter().map(|t| (t.name.as_str(), t.category.as_str())).collect();
    crate::ui::theme_list_help(&pairs)
}

// ── State initialization ────────────────────────────────────────────────────

fn init_state(behavior: &Behavior, n_seg: usize) -> Vec<f64> {
    match behavior {
        Behavior::Heat { .. } => (0..n_seg).map(|i| (i as f64 * 0.4).fract().max(0.3)).collect(),
        _ => vec![0.0; n_seg],
    }
}

// ── Frame rendering ─────────────────────────────────────────────────────────

fn get_delay(delay: &Delay, rng: &mut impl RngExt) -> u64 {
    match delay {
        Delay::Fixed(ms) => *ms,
        Delay::Random(lo, hi) => rng.random_range(*lo..=*hi),
    }
}

fn render_frame(
    behavior: &Behavior,
    rng: &mut impl RngExt,
    state: &mut [f64],
    n_seg: usize,
    t: f64,
) -> Vec<Rgb> {
    match behavior {
        Behavior::Heat { palette, volatility, spark_chance, spark_boost, dim_chance, dim_range, diffusion } =>
            render_heat(palette, *volatility, *spark_chance, *spark_boost, *dim_chance, *dim_range, *diffusion, rng, state, n_seg),
        Behavior::Wave { palette, waves, weights } =>
            render_wave(palette, waves, weights, n_seg, t),
        Behavior::Breathe { palette, speed, power } =>
            render_breathe(palette, *speed, *power, n_seg, t),
        Behavior::Flash { base_palette, flash_palette, decay, flash_chance, spread, base_wave_speed, base_spatial_freq, flash_threshold } =>
            render_flash(base_palette, flash_palette, *decay, *flash_chance, *spread, *base_wave_speed, *base_spatial_freq, *flash_threshold, rng, state, n_seg, t),
        Behavior::Particles { bg, palette, speed, spawn_chance, bright_chance } =>
            render_particles(*bg, palette, *speed, *spawn_chance, *bright_chance, rng, state, n_seg),
        Behavior::Twinkle { bg, colors, on_chance, fade_speed } =>
            render_twinkle(*bg, colors, *on_chance, *fade_speed, rng, state, n_seg),
        Behavior::HueRotate { speed, saturation, value } =>
            render_hue_rotate(*speed, *saturation, *value, n_seg, t),
        Behavior::GradientWave { color_a, color_b, speed } =>
            render_gradient_wave(*color_a, *color_b, *speed, n_seg, t),
        Behavior::Strobe { colors, cycle_speed, flash_chance } =>
            render_strobe(colors, *cycle_speed, *flash_chance, rng, n_seg, t),
        Behavior::Alternating { colors, sparkle, sparkle_chance, shift_speed } =>
            render_alternating(colors, *sparkle, *sparkle_chance, *shift_speed, rng, n_seg, t),
        Behavior::Drift { palette, speed } =>
            render_drift(palette, *speed, n_seg, t),
        Behavior::RadiatePulse { color, speed, width } =>
            render_radiate_pulse(*color, *speed, *width, n_seg, t),
        Behavior::Progression { palette, duration_secs, spatial_spread } =>
            render_progression(palette, *duration_secs, *spatial_spread, n_seg, t),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_heat(
    palette: &[PA], volatility: f64, spark_chance: f64, spark_boost: f64,
    dim_chance: f64, dim_range: (f64, f64), diffusion: f64,
    rng: &mut impl RngExt, state: &mut [f64], n_seg: usize,
) -> Vec<Rgb> {
    for s in state.iter_mut().take(n_seg) {
        *s += rng.random_range(-volatility..volatility);
        *s = s.clamp(0.0, 1.0);
    }
    if dim_chance > 0.0 && rng.random_range(0.0..1.0) < dim_chance {
        let idx = rng.random_range(0..n_seg);
        state[idx] *= rng.random_range(dim_range.0..dim_range.1);
    }
    if rng.random_range(0.0..1.0) < spark_chance {
        let idx = rng.random_range(0..n_seg);
        state[idx] = (state[idx] + spark_boost).min(1.0);
    }
    if diffusion > 0.0 {
        let snap: Vec<f64> = state[..n_seg].to_vec();
        for i in 0..n_seg {
            let left = if i > 0 { snap[i - 1] } else { snap[i] };
            let right = if i < n_seg - 1 { snap[i + 1] } else { snap[i] };
            state[i] = snap[i] * (1.0 - 2.0 * diffusion) + left * diffusion + right * diffusion;
        }
    }
    state[..n_seg].iter().map(|&h| palette_sample(palette, h)).collect()
}

fn render_wave(palette: &[PA], waves: &[WaveParam], weights: &[f64], n_seg: usize, t: f64) -> Vec<Rgb> {
    (0..n_seg)
        .map(|i| {
            let mut val = 0.0;
            let mut tw = 0.0;
            for (j, w_param) in waves.iter().enumerate() {
                let w = weights.get(j).copied().unwrap_or(1.0);
                val += ((t * w_param.time_speed + i as f64 * w_param.spatial_freq + w_param.phase_offset).sin() * 0.5 + 0.5) * w;
                tw += w;
            }
            palette_sample(palette, val / tw)
        })
        .collect()
}

fn render_breathe(palette: &[PA], speed: f64, power: u32, n_seg: usize, t: f64) -> Vec<Rgb> {
    let breath = ((t * speed).sin() * 0.5 + 0.5).powi(power as i32);
    let color = palette_sample(palette, breath);
    vec![color; n_seg]
}

#[allow(clippy::too_many_arguments)]
fn render_flash(
    base_palette: &[PA], flash_palette: &[PA], decay: f64, flash_chance: f64,
    spread: (usize, usize), base_wave_speed: f64, base_spatial_freq: f64, flash_threshold: f64,
    rng: &mut impl RngExt, state: &mut [f64], n_seg: usize, t: f64,
) -> Vec<Rgb> {
    for s in state.iter_mut().take(n_seg) {
        *s *= decay;
    }
    if rng.random_range(0.0..1.0) < flash_chance {
        let center = rng.random_range(0..n_seg);
        let sp = rng.random_range(spread.0..=spread.1);
        let lo = center.saturating_sub(sp);
        let hi = (center + sp).min(n_seg - 1);
        for s in state[lo..=hi].iter_mut() {
            *s = rng.random_range(0.7..1.0);
        }
    }
    (0..n_seg)
        .map(|i| {
            if state[i] > flash_threshold {
                palette_sample(flash_palette, state[i])
            } else {
                let bp = (t * base_wave_speed + i as f64 * base_spatial_freq).sin() * 0.15 + 0.5;
                palette_sample(base_palette, bp)
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn render_particles(
    bg: Rgb, palette: &[PA], speed: f64, spawn_chance: f64, bright_chance: f64,
    rng: &mut impl RngExt, state: &mut [f64], n_seg: usize,
) -> Vec<Rgb> {
    for s in state.iter_mut().take(n_seg) {
        *s *= 1.0 - speed;
    }
    for i in (1..n_seg).rev() {
        state[i] = state[i - 1];
    }
    state[0] = 0.0;
    if rng.random_range(0.0..1.0) < spawn_chance {
        state[0] = if rng.random_range(0.0..1.0) < bright_chance {
            1.0
        } else {
            rng.random_range(0.3..0.7)
        };
    }
    state[..n_seg]
        .iter()
        .map(|&v| {
            if v > 0.02 {
                palette_sample(palette, v)
            } else {
                bg
            }
        })
        .collect()
}

fn render_twinkle(
    bg: Rgb, colors: &[Rgb], on_chance: f64, fade_speed: f64,
    rng: &mut impl RngExt, state: &mut [f64], n_seg: usize,
) -> Vec<Rgb> {
    for s in state.iter_mut().take(n_seg) {
        if *s > 0.0 {
            *s = (*s - fade_speed).max(0.0);
        } else if rng.random_range(0.0..1.0) < on_chance {
            *s = 1.0;
        }
    }
    (0..n_seg)
        .map(|i| {
            if state[i] > 0.02 {
                let c = colors[i % colors.len()];
                lerp_rgb(bg, c, state[i])
            } else {
                bg
            }
        })
        .collect()
}

fn render_hue_rotate(speed: f64, saturation: f64, value: f64, n_seg: usize, t: f64) -> Vec<Rgb> {
    (0..n_seg)
        .map(|i| {
            let hue = (t * speed + i as f64 / n_seg as f64).fract();
            hsv_to_rgb(hue, saturation, value)
        })
        .collect()
}

fn render_gradient_wave(color_a: Rgb, color_b: Rgb, speed: f64, n_seg: usize, t: f64) -> Vec<Rgb> {
    (0..n_seg)
        .map(|i| {
            let wave =
                (t * speed + i as f64 * std::f64::consts::PI / n_seg as f64).sin() * 0.5 + 0.5;
            lerp_rgb(color_a, color_b, wave)
        })
        .collect()
}

fn render_strobe(
    colors: &[Rgb], cycle_speed: f64, flash_chance: f64,
    rng: &mut impl RngExt, n_seg: usize, t: f64,
) -> Vec<Rgb> {
    (0..n_seg)
        .map(|i| {
            if rng.random_range(0.0..1.0) < flash_chance {
                (255, 255, 255)
            } else {
                let idx = ((t * cycle_speed) as usize + i) % colors.len();
                colors[idx]
            }
        })
        .collect()
}

fn render_alternating(
    colors: &[Rgb], sparkle: Rgb, sparkle_chance: f64, shift_speed: f64,
    rng: &mut impl RngExt, n_seg: usize, t: f64,
) -> Vec<Rgb> {
    let shift = (t * shift_speed) as usize;
    (0..n_seg)
        .map(|i| {
            if rng.random_range(0.0..1.0) < sparkle_chance {
                sparkle
            } else {
                colors[(i + shift) % colors.len()]
            }
        })
        .collect()
}

fn render_drift(palette: &[PA], speed: f64, n_seg: usize, t: f64) -> Vec<Rgb> {
    (0..n_seg)
        .map(|i| {
            let pos = ((i as f64 / n_seg as f64) + t * speed).fract();
            palette_sample(palette, pos)
        })
        .collect()
}

fn render_radiate_pulse(color: Rgb, speed: f64, width: f64, n_seg: usize, t: f64) -> Vec<Rgb> {
    let center = n_seg as f64 / 2.0;
    let pulse_pos = (t * speed).fract();
    (0..n_seg)
        .map(|i| {
            let dist = (i as f64 - center).abs() / center;
            let diff = (dist - pulse_pos).abs();
            let bright = (1.0 - diff / width).max(0.0);
            (
                (color.0 as f64 * bright) as u8,
                (color.1 as f64 * bright) as u8,
                (color.2 as f64 * bright) as u8,
            )
        })
        .collect()
}

fn render_progression(palette: &[PA], duration_secs: f64, spatial_spread: f64, n_seg: usize, t: f64) -> Vec<Rgb> {
    let progress = (t / duration_secs).min(1.0);
    (0..n_seg)
        .map(|i| {
            let p = (progress + (i as f64 - n_seg as f64 / 2.0) * spatial_spread)
                .clamp(0.0, 1.0);
            palette_sample(palette, p)
        })
        .collect()
}

// ── Run loop ────────────────────────────────────────────────────────────────

pub fn run_theme(
    name: &str,
    ip: Option<String>,
    brightness: u8,
    segments: usize,
    mirror: bool,
    _debug: bool,
) {
    let theme = match get_theme(name) {
        Some(t) => t,
        None => {
            crate::ui::error_hint(
                &format!("Unknown theme \"{name}\""),
                "Run govee theme --help to see available themes",
            );
            std::process::exit(1);
        }
    };

    let ip = resolve_or_exit(ip.as_deref());

    match &theme.kind {
        ThemeKind::Solid { color } => {
            let _ = razer_deactivate(&ip);
            send_turn(&ip, true).ok();
            std::thread::sleep(Duration::from_millis(50));
            send_brightness(&ip, brightness).ok();
            std::thread::sleep(Duration::from_millis(50));
            send_color(&ip, color.0, color.1, color.2).ok();
            {
                use colored::Colorize;
                crate::ui::info("Theme", &format!("{} {}", name.white().bold(), format!("[{}]", theme.category).dimmed()));
                crate::ui::info("Brightness", &crate::ui::brightness_bar(brightness));
            }
        }
        ThemeKind::Animated { behavior, delay } => {
            crate::dreamview::activate(&ip, brightness, true);

            {
                use colored::Colorize;
                crate::ui::info("Theme", &format!("{} {}", name.white().bold(), format!("[{}]", theme.category).dimmed()));
                crate::ui::info("Brightness", &crate::ui::brightness_bar(brightness));
                crate::ui::info("Segments", &format!("{segments}"));
                println!("  {}", "Press Ctrl+C to stop".dimmed());
            }

            ctrlc_setup();

            let mut rng = rand::rng();
            let n_seg = segments;
            let mut state = init_state(behavior, n_seg);
            let mut t_acc: f64 = 0.0;

            while RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                let colors = render_frame(behavior, &mut rng, &mut state, n_seg, t_acc);

                let send_colors = crate::dreamview::apply_mirror(&colors, mirror);

                let _ = send_segments(&ip, &send_colors, true);
                crate::ui::status_line(&send_colors, "");

                let delay_ms = get_delay(delay, &mut rng);
                std::thread::sleep(Duration::from_millis(delay_ms));
                t_acc += delay_ms as f64 / 1000.0;
            }

            crate::dreamview::shutdown(&ip, true);
        }
    }
}
