use govee_lan::*;
use govee_lan::audio::{AudioAnalyzer, map_colors};
use std::process;
use std::time::{Duration, Instant};

use crate::cli::AudioArgs;
use crate::{RUNNING, ctrlc_setup, resolve_or_exit};

pub fn run_audio(args: AudioArgs, mirror: bool) {
    let ip = resolve_or_exit(args.ip.as_deref());
    println!("Using device at {ip}");

    let use_razer = !args.no_dreamview;
    let n_seg = if use_razer { args.segments.max(1) } else { 1 };

    if let Err(e) = send_brightness(&ip, args.brightness) {
        eprintln!("Failed to set brightness: {e}");
    }

    if use_razer {
        if let Err(e) = razer_activate(&ip) {
            eprintln!("Failed to activate DreamView: {e}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let analyzer = match AudioAnalyzer::new() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Failed to start audio capture: {e}");
            eprintln!("Make sure PulseAudio is running and audio is playing");
            if use_razer {
                let _ = razer_deactivate(&ip);
            }
            process::exit(1);
        }
    };

    let mode_str = if use_razer {
        format!(
            "DreamView ({n_seg} segments{})",
            if args.gradient { ", gradient" } else { "" }
        )
    } else {
        "single color".to_string()
    };
    println!(
        "Mode: {:?} | Palette: {:?} | {} | Sensitivity: {} | Brightness: {}%",
        args.mode, args.palette, mode_str, args.sensitivity, args.brightness
    );
    println!("Press Ctrl+C to stop");

    ctrlc_setup();

    let mut smoothed: Vec<(f64, f64, f64)> = vec![(0.0, 0.0, 0.0); n_seg];
    let tick = Duration::from_secs_f64(1.0 / 60.0);
    let mut t: f64 = 0.0;
    let mut beat_hue: f64 = 0.0;
    let mut beat_decay: f64 = 0.0;

    while RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        let t0 = Instant::now();

        let mut audio = analyzer.get_state();

        audio.energy = (audio.energy * args.sensitivity).clamp(0.0, 1.0);
        for band in audio.bands.iter_mut() {
            *band = (*band * args.sensitivity).clamp(0.0, 1.0);
        }

        let raw_colors = map_colors(
            &audio,
            args.mode,
            args.palette,
            n_seg,
            t,
            &mut beat_hue,
            &mut beat_decay,
        );

        let mut current_colors = Vec::with_capacity(n_seg);
        for i in 0..n_seg {
            smoothed[i] = smooth(smoothed[i], raw_colors[i], args.smoothing);
            current_colors.push((
                smoothed[i].0 as u8,
                smoothed[i].1 as u8,
                smoothed[i].2 as u8,
            ));
        }

        let send_colors = if mirror {
            let mut mirrored = current_colors.clone();
            mirrored.extend(current_colors.iter().rev());
            mirrored
        } else {
            current_colors.clone()
        };

        if use_razer {
            let _ = send_segments(&ip, &send_colors, args.gradient);
        } else {
            let (r, g, b) = send_colors[0];
            let _ = send_color(&ip, r, g, b);
        }
        if args.verbose {
            let parts: Vec<String> = current_colors
                .iter()
                .map(|(r, g, b)| format!("({r:3},{g:3},{b:3})"))
                .collect();
            println!(
                "  E:{:.2} B:[{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}] beat:{} -> {}",
                audio.energy,
                audio.bands[0], audio.bands[1], audio.bands[2],
                audio.bands[3], audio.bands[4], audio.bands[5],
                audio.beat,
                parts.join(" | ")
            );
        }

        t += tick.as_secs_f64();

        let elapsed = t0.elapsed();
        if elapsed < tick {
            std::thread::sleep(tick - elapsed);
        }
    }

    println!();
    drop(analyzer);
    if use_razer {
        println!("Deactivating DreamView mode...");
        let _ = razer_deactivate(&ip);
    }
    println!("Stopped.");
}
