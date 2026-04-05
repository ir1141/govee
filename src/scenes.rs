use govee_lan::*;
use rand::RngExt;
use std::time::Duration;

use crate::{RUNNING, ctrlc_setup, resolve_or_exit};

pub fn run_animated_scene(name: &str, ip: Option<String>, brightness: u8, mirror: bool) {
    let ip = resolve_or_exit(ip.as_deref());
    println!("Using device at {ip}");

    if let Err(e) = send_brightness(&ip, brightness) {
        eprintln!("Failed to set brightness: {e}");
    }

    if let Err(e) = razer_activate(&ip) {
        eprintln!("Failed to activate DreamView: {e}");
    }
    std::thread::sleep(Duration::from_millis(100));

    println!("Scene '{name}' | Brightness: {brightness}% | Press Ctrl+C to stop");

    ctrlc_setup();

    let mut rng = rand::rng();
    let n_seg: usize = 5;
    let mut phase: Vec<f64> = (0..n_seg).map(|i| i as f64 * 0.4).collect();
    let mut t: f64 = 0.0;

    while RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        let colors: Vec<(u8, u8, u8)> = match name {
            "fireplace" => scene_fireplace(&mut rng, &mut phase, n_seg),
            "storm" => scene_storm(&mut rng, &mut phase, n_seg, t),
            "ocean" => scene_ocean(n_seg, t),
            "aurora" => scene_aurora(n_seg, t),
            "lava" => scene_lava(&mut rng, &mut phase, n_seg),
            "breathing" => scene_breathing(n_seg, t),
            "sunrise" => scene_sunrise(n_seg, t),
            _ => unreachable!(),
        };

        let send_colors = if mirror {
            let mut mirrored = colors.clone();
            mirrored.extend(colors.iter().rev());
            mirrored
        } else {
            colors
        };

        let _ = send_segments(&ip, &send_colors, true);

        let delay = match name {
            "fireplace" | "lava" => rng.random_range(80..180),
            "storm" => rng.random_range(50..150),
            "sunrise" => 500,
            _ => 80,
        };
        std::thread::sleep(Duration::from_millis(delay));
        t += delay as f64 / 1000.0;
    }

    println!();
    println!("Deactivating DreamView mode...");
    let _ = razer_deactivate(&ip);
    println!("Stopped.");
}

pub const ANIMATED_SCENES: &[&str] = &[
    "fireplace", "storm", "ocean", "aurora", "lava", "breathing", "sunrise",
];

fn scene_fireplace(rng: &mut impl rand::RngExt, heat: &mut [f64], n_seg: usize) -> Vec<(u8, u8, u8)> {
    for h in heat.iter_mut() {
        *h += rng.random_range(-0.15..0.15);
        *h = h.clamp(0.0, 1.0);
    }
    if rng.random_range(0.0..1.0) < 0.2 {
        let idx = rng.random_range(0..n_seg);
        heat[idx] *= rng.random_range(0.2..0.6);
    }
    if rng.random_range(0.0..1.0) < 0.1 {
        let idx = rng.random_range(0..n_seg);
        heat[idx] = (heat[idx] + 0.4).min(1.0);
    }
    heat.iter()
        .map(|&h| {
            let r = (80.0 + h * 175.0) as u8;
            let g = (h * h * 140.0) as u8;
            let b = (h * h * h * 30.0) as u8;
            (r, g, b)
        })
        .collect()
}

fn scene_storm(rng: &mut impl rand::RngExt, intensity: &mut [f64], n_seg: usize, t: f64) -> Vec<(u8, u8, u8)> {
    for v in intensity.iter_mut() {
        *v *= 0.85;
    }
    if rng.random_range(0.0..1.0) < 0.08 {
        let idx = rng.random_range(0..n_seg);
        let spread = rng.random_range(1..=2);
        for v in intensity.iter_mut().take((idx + spread).min(n_seg - 1) + 1).skip(idx.saturating_sub(spread)) {
            *v = rng.random_range(0.7..1.0);
        }
    }
    (0..n_seg)
        .map(|i| {
            let base_pulse = (t * 0.3 + i as f64 * 0.5).sin() * 0.15 + 0.5;
            let flash = intensity[i];
            if flash > 0.3 {
                let v = (180.0 + flash * 75.0) as u8;
                (v, v, 255)
            } else {
                let r = (10.0 * base_pulse) as u8;
                let g = (5.0 * base_pulse) as u8;
                let b = (40.0 + 30.0 * base_pulse) as u8;
                (r, g, b)
            }
        })
        .collect()
}

fn scene_ocean(n_seg: usize, t: f64) -> Vec<(u8, u8, u8)> {
    (0..n_seg)
        .map(|i| {
            let wave1 = (t * 0.8 + i as f64 * 0.7).sin() * 0.5 + 0.5;
            let wave2 = (t * 0.5 + i as f64 * 1.2 + 1.0).sin() * 0.3 + 0.5;
            let combined = wave1 * 0.6 + wave2 * 0.4;
            let r = 0u8;
            let g = (40.0 + combined * 120.0) as u8;
            let b = (80.0 + combined * 140.0) as u8;
            (r, g, b)
        })
        .collect()
}

fn scene_aurora(n_seg: usize, t: f64) -> Vec<(u8, u8, u8)> {
    (0..n_seg)
        .map(|i| {
            let drift = (t * 0.3 + i as f64 * 0.8).sin() * 0.5 + 0.5;
            let shimmer = (t * 1.5 + i as f64 * 2.0).sin() * 0.2 + 0.8;
            let r = (drift * 120.0 * shimmer) as u8;
            let g = ((1.0 - drift) * 200.0 * shimmer) as u8;
            let b = (60.0 + drift * 140.0 * shimmer) as u8;
            (r, g, b)
        })
        .collect()
}

fn scene_lava(rng: &mut impl rand::RngExt, heat: &mut [f64], n_seg: usize) -> Vec<(u8, u8, u8)> {
    for h in heat.iter_mut() {
        *h += rng.random_range(-0.08..0.08);
        *h = h.clamp(0.0, 1.0);
    }
    let snapshot: Vec<f64> = heat.to_vec();
    for i in 0..n_seg {
        let left = if i > 0 { snapshot[i - 1] } else { snapshot[i] };
        let right = if i < n_seg - 1 { snapshot[i + 1] } else { snapshot[i] };
        heat[i] = snapshot[i] * 0.6 + left * 0.2 + right * 0.2;
    }
    if rng.random_range(0.0..1.0) < 0.05 {
        let idx = rng.random_range(0..n_seg);
        heat[idx] = 1.0;
    }
    heat.iter()
        .map(|&h| {
            let r = (120.0 + h * 135.0) as u8;
            let g = (h * h * 60.0) as u8;
            let b = 0u8;
            (r, g, b)
        })
        .collect()
}

fn scene_breathing(n_seg: usize, t: f64) -> Vec<(u8, u8, u8)> {
    let breath = ((t * 0.4).sin() * 0.5 + 0.5).powi(2);
    let r = (40.0 + breath * 200.0) as u8;
    let g = (10.0 + breath * 80.0) as u8;
    let b = (breath * 20.0) as u8;
    vec![(r, g, b); n_seg]
}

fn scene_sunrise(n_seg: usize, t: f64) -> Vec<(u8, u8, u8)> {
    let progress = (t / 600.0).min(1.0);
    (0..n_seg)
        .map(|i| {
            let p = (progress + (i as f64 - n_seg as f64 / 2.0) * 0.02).clamp(0.0, 1.0);
            if p < 0.33 {
                let sub = p / 0.33;
                let r = (60.0 + sub * 195.0) as u8;
                let g = (sub * 80.0) as u8;
                let b = 0u8;
                (r, g, b)
            } else if p < 0.66 {
                let sub = (p - 0.33) / 0.33;
                let r = 255;
                let g = (80.0 + sub * 120.0) as u8;
                let b = (sub * 30.0) as u8;
                (r, g, b)
            } else {
                let sub = (p - 0.66) / 0.34;
                let r = 255;
                let g = (200.0 + sub * 40.0) as u8;
                let b = (30.0 + sub * 150.0) as u8;
                (r, g, b)
            }
        })
        .collect()
}
