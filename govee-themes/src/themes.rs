use serde::{Deserialize, Serialize};

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

pub fn palette_sample(anchors: &[PA], t: f64) -> Rgb {
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
            return lerp_rgb((a.r, a.g, a.b), (b.r, b.g, b.b), f);
        }
    }
    let l = anchors.last().unwrap();
    (l.r, l.g, l.b)
}

pub fn lerp_rgb(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    (
        (a.0 as f64 + (b.0 as f64 - a.0 as f64) * t) as u8,
        (a.1 as f64 + (b.1 as f64 - a.1 as f64) * t) as u8,
        (a.2 as f64 + (b.2 as f64 - a.2 as f64) * t) as u8,
    )
}

pub fn hsv_to_rgb(h: f64, s: f64, v: f64) -> Rgb {
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
