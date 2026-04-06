use govee_lan::*;
use rand::RngExt;
use std::time::Duration;

use crate::{RUNNING, ctrlc_setup, resolve_or_exit};

// ── Types ───────────────────────────────────────────────────────────────────

pub type Rgb = (u8, u8, u8);

/// Palette anchor: (position 0.0–1.0, r, g, b)
type PA = (f64, u8, u8, u8);

// ── Color utilities ─────────────────────────────────────────────────────────

fn palette_sample(anchors: &[PA], t: f64) -> Rgb {
    if anchors.is_empty() {
        return (0, 0, 0);
    }
    if anchors.len() == 1 {
        return (anchors[0].1, anchors[0].2, anchors[0].3);
    }
    let t = t.clamp(0.0, 1.0);
    // Below the first anchor: return first color
    if t <= anchors[0].0 {
        return (anchors[0].1, anchors[0].2, anchors[0].3);
    }
    for i in 0..anchors.len() - 1 {
        let (pa, ra, ga, ba) = anchors[i];
        let (pb, rb, gb, bb) = anchors[i + 1];
        if t <= pb {
            let f = if (pb - pa).abs() < 1e-9 {
                0.0
            } else {
                ((t - pa) / (pb - pa)).clamp(0.0, 1.0)
            };
            return (
                (ra as f64 + (rb as f64 - ra as f64) * f) as u8,
                (ga as f64 + (gb as f64 - ga as f64) * f) as u8,
                (ba as f64 + (bb as f64 - ba as f64) * f) as u8,
            );
        }
    }
    let l = anchors.last().unwrap();
    (l.1, l.2, l.3)
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

pub enum Delay {
    Fixed(u64),
    Random(u64, u64),
}

pub enum Behavior {
    /// Heat diffusion: fireplace, candlelight, campfire, lava
    Heat {
        palette: &'static [PA],
        volatility: f64,
        spark_chance: f64,
        spark_boost: f64,
        dim_chance: f64,
        dim_range: (f64, f64),
        diffusion: f64,
    },
    /// Overlapping sine waves: ocean, aurora, northern lights
    Wave {
        palette: &'static [PA],
        waves: &'static [(f64, f64, f64)], // (time_speed, spatial_freq, phase_offset)
        weights: &'static [f64],
    },
    /// Global breathing pulse: breathing, romantic, cozy
    Breathe {
        palette: &'static [PA],
        speed: f64,
        power: u32,
    },
    /// Lightning/flash on ambient base: storm, lightning, thunderstorm
    Flash {
        base_palette: &'static [PA],
        flash_palette: &'static [PA],
        decay: f64,
        flash_chance: f64,
        spread: (usize, usize),
        base_wave_speed: f64,
        base_spatial_freq: f64,
        flash_threshold: f64,
    },
    /// Falling/cascading particles: rain, snowfall
    Particles {
        bg: Rgb,
        palette: &'static [PA],
        speed: f64,
        spawn_chance: f64,
        bright_chance: f64,
    },
    /// Random twinkling points: starfield
    Twinkle {
        bg: Rgb,
        colors: &'static [Rgb],
        on_chance: f64,
        fade_speed: f64,
    },
    /// Rotating rainbow hue: rainbow
    HueRotate {
        speed: f64,
        saturation: f64,
        value: f64,
    },
    /// Two-color oscillating gradient: gradient-wave
    GradientWave {
        color_a: Rgb,
        color_b: Rgb,
        speed: f64,
    },
    /// Fast color cycling with flash bursts: nightclub
    Strobe {
        colors: &'static [Rgb],
        cycle_speed: f64,
        flash_chance: f64,
    },
    /// Alternating colors with sparkle: christmas, halloween
    Alternating {
        colors: &'static [Rgb],
        sparkle: Rgb,
        sparkle_chance: f64,
        shift_speed: f64,
    },
    /// Continuous palette drift: cyberpunk, vaporwave
    Drift {
        palette: &'static [PA],
        speed: f64,
    },
    /// Pulse radiating from center outward
    RadiatePulse {
        color: Rgb,
        speed: f64,
        width: f64,
    },
    /// Timed color progression: sunrise
    Progression {
        palette: &'static [PA],
        duration_secs: f64,
        spatial_spread: f64,
    },
}

pub enum ThemeKind {
    Solid { color: Rgb },
    Animated { behavior: Behavior, delay: Delay },
}

pub struct ThemeDef {
    pub name: &'static str,
    pub category: &'static str,
    pub kind: ThemeKind,
}

// ── Theme registry (definitions in theme_defs.rs) ──────────────────────────

use crate::theme_defs::THEMES;

// ── Lookup ───────────────────────────────────────────────────────────────────

pub fn get_theme(name: &str) -> Option<&'static ThemeDef> {
    THEMES.iter().find(|t| t.name == name)
}

pub fn theme_list_display() -> String {
    let themes: Vec<(&str, &str)> = THEMES.iter().map(|t| (t.name, t.category)).collect();
    crate::ui::theme_list_help(&themes)
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
        Behavior::Heat {
            palette, volatility, spark_chance, spark_boost,
            dim_chance, dim_range, diffusion,
        } => {
            for s in state.iter_mut().take(n_seg) {
                *s += rng.random_range(-*volatility..*volatility);
                *s = s.clamp(0.0, 1.0);
            }
            if *dim_chance > 0.0 && rng.random_range(0.0..1.0) < *dim_chance {
                let idx = rng.random_range(0..n_seg);
                state[idx] *= rng.random_range(dim_range.0..dim_range.1);
            }
            if rng.random_range(0.0..1.0) < *spark_chance {
                let idx = rng.random_range(0..n_seg);
                state[idx] = (state[idx] + spark_boost).min(1.0);
            }
            if *diffusion > 0.0 {
                let snap: Vec<f64> = state[..n_seg].to_vec();
                for i in 0..n_seg {
                    let left = if i > 0 { snap[i - 1] } else { snap[i] };
                    let right = if i < n_seg - 1 { snap[i + 1] } else { snap[i] };
                    state[i] = snap[i] * (1.0 - 2.0 * diffusion) + left * diffusion + right * diffusion;
                }
            }
            state[..n_seg].iter().map(|&h| palette_sample(palette, h)).collect()
        }

        Behavior::Wave { palette, waves, weights } => {
            (0..n_seg)
                .map(|i| {
                    let mut val = 0.0;
                    let mut tw = 0.0;
                    for (j, &(spd, freq, off)) in waves.iter().enumerate() {
                        let w = weights.get(j).copied().unwrap_or(1.0);
                        val += ((t * spd + i as f64 * freq + off).sin() * 0.5 + 0.5) * w;
                        tw += w;
                    }
                    palette_sample(palette, val / tw)
                })
                .collect()
        }

        Behavior::Breathe { palette, speed, power } => {
            let breath = ((t * speed).sin() * 0.5 + 0.5).powi(*power as i32);
            let color = palette_sample(palette, breath);
            vec![color; n_seg]
        }

        Behavior::Flash {
            base_palette, flash_palette, decay, flash_chance, spread,
            base_wave_speed, base_spatial_freq, flash_threshold,
        } => {
            for s in state.iter_mut().take(n_seg) {
                *s *= decay;
            }
            if rng.random_range(0.0..1.0) < *flash_chance {
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
                    if state[i] > *flash_threshold {
                        palette_sample(flash_palette, state[i])
                    } else {
                        let bp = (t * base_wave_speed + i as f64 * base_spatial_freq).sin() * 0.15 + 0.5;
                        palette_sample(base_palette, bp)
                    }
                })
                .collect()
        }

        Behavior::Particles { bg, palette, speed, spawn_chance, bright_chance } => {
            // Fade first so shifted values lose brightness as they travel
            for s in state.iter_mut().take(n_seg) {
                *s *= 1.0 - speed;
            }
            // Shift particles toward higher indices (cascade down)
            for i in (1..n_seg).rev() {
                state[i] = state[i - 1];
            }
            state[0] = 0.0;
            // Spawn at top (after shift so it renders at full brightness)
            if rng.random_range(0.0..1.0) < *spawn_chance {
                state[0] = if rng.random_range(0.0..1.0) < *bright_chance {
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
                        *bg
                    }
                })
                .collect()
        }

        Behavior::Twinkle { bg, colors, on_chance, fade_speed } => {
            for s in state.iter_mut().take(n_seg) {
                if *s > 0.0 {
                    *s = (*s - fade_speed).max(0.0);
                } else if rng.random_range(0.0..1.0) < *on_chance {
                    *s = 1.0;
                }
            }
            (0..n_seg)
                .map(|i| {
                    if state[i] > 0.02 {
                        let c = colors[i % colors.len()];
                        lerp_rgb(*bg, c, state[i])
                    } else {
                        *bg
                    }
                })
                .collect()
        }

        Behavior::HueRotate { speed, saturation, value } => {
            (0..n_seg)
                .map(|i| {
                    let hue = (t * speed + i as f64 / n_seg as f64).fract();
                    hsv_to_rgb(hue, *saturation, *value)
                })
                .collect()
        }

        Behavior::GradientWave { color_a, color_b, speed } => {
            (0..n_seg)
                .map(|i| {
                    let wave =
                        (t * speed + i as f64 * std::f64::consts::PI / n_seg as f64).sin() * 0.5
                            + 0.5;
                    lerp_rgb(*color_a, *color_b, wave)
                })
                .collect()
        }

        Behavior::Strobe { colors, cycle_speed, flash_chance } => {
            (0..n_seg)
                .map(|i| {
                    if rng.random_range(0.0..1.0) < *flash_chance {
                        (255, 255, 255)
                    } else {
                        let idx = ((t * cycle_speed) as usize + i) % colors.len();
                        colors[idx]
                    }
                })
                .collect()
        }

        Behavior::Alternating { colors, sparkle, sparkle_chance, shift_speed } => {
            let shift = (t * shift_speed) as usize;
            (0..n_seg)
                .map(|i| {
                    if rng.random_range(0.0..1.0) < *sparkle_chance {
                        *sparkle
                    } else {
                        colors[(i + shift) % colors.len()]
                    }
                })
                .collect()
        }

        Behavior::Drift { palette, speed } => {
            (0..n_seg)
                .map(|i| {
                    let pos = ((i as f64 / n_seg as f64) + t * speed).fract();
                    palette_sample(palette, pos)
                })
                .collect()
        }

        Behavior::RadiatePulse { color, speed, width } => {
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

        Behavior::Progression { palette, duration_secs, spatial_spread } => {
            let progress = (t / duration_secs).min(1.0);
            (0..n_seg)
                .map(|i| {
                    let p = (progress + (i as f64 - n_seg as f64 / 2.0) * spatial_spread)
                        .clamp(0.0, 1.0);
                    palette_sample(palette, p)
                })
                .collect()
        }
    }
}

// ── Run loop ────────────────────────────────────────────────────────────────

pub fn run_theme(
    name: &str,
    ip: Option<String>,
    brightness: u8,
    segments: usize,
    mirror: bool,
    debug: bool,
) {
    let theme = match get_theme(name) {
        Some(t) => t,
        None => {
            crate::ui::error_hint(
                &format!("Unknown theme \"{name}\""),
                "Run govee theme --list to see available themes",
            );
            std::process::exit(1);
        }
    };

    let ip = resolve_or_exit(ip.as_deref());

    match &theme.kind {
        ThemeKind::Solid { color } => {
            send_command(&ip, "turn", serde_json::json!({"value": 1}), debug);
            send_command(
                &ip,
                "brightness",
                serde_json::json!({"value": brightness}),
                debug,
            );
            send_command(
                &ip,
                "colorwc",
                serde_json::json!({
                    "color": {"r": color.0, "g": color.1, "b": color.2},
                    "colorTemInKelvin": 0,
                }),
                debug,
            );
            {
                use colored::Colorize;
                crate::ui::info("Theme", &format!("{} {}", name.white().bold(), format!("[{}]", theme.category).dimmed()));
                crate::ui::info("Brightness", &crate::ui::brightness_bar(brightness));
            }
        }
        ThemeKind::Animated { behavior, delay } => {
            if let Err(e) = send_brightness(&ip, brightness) {
                eprintln!("Failed to set brightness: {e}");
            }
            if let Err(e) = razer_activate(&ip) {
                eprintln!("Failed to activate DreamView: {e}");
            }
            std::thread::sleep(Duration::from_millis(100));

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

                let send_colors = if mirror {
                    let mut m = colors.clone();
                    m.extend(colors.iter().rev());
                    m
                } else {
                    colors
                };

                let _ = send_segments(&ip, &send_colors, true);
                crate::ui::status_line(&send_colors, "");

                let delay_ms = get_delay(delay, &mut rng);
                std::thread::sleep(Duration::from_millis(delay_ms));
                t_acc += delay_ms as f64 / 1000.0;
            }

            crate::ui::status_line_finish();
            crate::ui::deactivating();
            let _ = razer_deactivate(&ip);
            crate::ui::stopped();
        }
    }
}
