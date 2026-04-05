use govee_lan::*;
use govee_lan::wayland::ScreenCapturer;
use std::process;
use std::time::{Duration, Instant};

use crate::cli::ScreenArgs;
use crate::{RUNNING, SCAN_TIMEOUT, ctrlc_setup};

pub fn run_screen(args: ScreenArgs, mirror: bool) {
    let mut capturer = match ScreenCapturer::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to initialize Wayland capture: {e}");
            eprintln!("Make sure your compositor supports wlr-screencopy-unstable-v1");
            process::exit(1);
        }
    };

    if args.verbose {
        println!("Available outputs: {:?}", capturer.outputs());
    }

    let ip = match resolve_ip(args.ip.as_deref(), SCAN_TIMEOUT) {
        Ok(ip) => ip,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };
    println!("Using device at {ip}");

    let use_razer = !args.no_razer;
    let n_seg = if use_razer { args.segments.max(1) } else { 1 };
    let interval = Duration::from_secs_f64(1.0 / args.fps.max(1) as f64);

    if let Err(e) = send_brightness(&ip, args.brightness) {
        eprintln!("Failed to set brightness: {e}");
    }

    if use_razer {
        if let Err(e) = razer_activate(&ip) {
            eprintln!("Failed to activate DreamView: {e}");
        }
        std::thread::sleep(Duration::from_millis(100));
        let mode = format!(
            "DreamView ({n_seg} segments{})",
            if args.gradient { ", gradient" } else { "" }
        );
        println!("Mode: {mode} | ~{}fps | Smoothing: {} | Brightness: {}%",
            args.fps, args.smoothing, args.brightness);
    } else {
        println!(
            "Mode: single color (colorwc) | ~{}fps | Smoothing: {} | Brightness: {}%",
            args.fps, args.smoothing, args.brightness
        );
    }
    println!("Press Ctrl+C to stop");

    let mut smoothed: Vec<(f64, f64, f64)> = vec![(0.0, 0.0, 0.0); n_seg];
    let mut last_sent: Vec<(u8, u8, u8)> = vec![(0, 0, 0); n_seg];
    let mut last_send_time = Instant::now();
    let keepalive_interval = Duration::from_secs(2);

    ctrlc_setup();

    // Seed smoothed colors
    if let Ok(frame) = capturer.capture(args.output.as_deref()) {
        let colors = frame.extract_edge_colors(n_seg);
        for (i, &(r, g, b)) in colors.iter().enumerate() {
            smoothed[i] = (r as f64, g as f64, b as f64);
        }
    }

    while RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        let t0 = Instant::now();

        let frame = match capturer.capture(args.output.as_deref()) {
            Ok(f) => f,
            Err(e) => {
                if args.verbose {
                    eprintln!("Capture error: {e}");
                }
                std::thread::sleep(interval);
                continue;
            }
        };

        let raw_colors = frame.extract_edge_colors(n_seg);
        let mut any_changed = false;
        let mut current_colors = Vec::with_capacity(n_seg);

        for i in 0..n_seg {
            let mut raw = raw_colors[i];
            if args.saturate != 1.0 {
                raw = saturate_color(raw, args.saturate);
            }

            smoothed[i] = smooth(smoothed[i], raw, args.smoothing);
            let current = (
                smoothed[i].0 as u8,
                smoothed[i].1 as u8,
                smoothed[i].2 as u8,
            );
            current_colors.push(current);

            if color_distance(current, last_sent[i]) >= args.threshold as f64 {
                any_changed = true;
            }
        }

        let needs_keepalive = last_send_time.elapsed() >= keepalive_interval;
        if any_changed || needs_keepalive {
            if use_razer {
                let send_colors = if mirror {
                    let mut mirrored = current_colors.clone();
                    mirrored.extend(current_colors.iter().rev());
                    mirrored
                } else {
                    current_colors.clone()
                };
                let _ = send_segments(&ip, &send_colors, args.gradient);
            } else {
                let (r, g, b) = current_colors[0];
                let _ = send_color(&ip, r, g, b);
            }

            last_send_time = Instant::now();

            if args.verbose && any_changed {
                let parts: Vec<String> = current_colors
                    .iter()
                    .map(|(r, g, b)| format!("({r:3},{g:3},{b:3})"))
                    .collect();
                println!("  -> {}", parts.join(" | "));
            }

            last_sent = current_colors;
        }

        let elapsed = t0.elapsed();
        if elapsed < interval {
            std::thread::sleep(interval - elapsed);
        }
    }

    println!();
    if use_razer {
        println!("Deactivating DreamView mode...");
        let _ = razer_deactivate(&ip);
    }
    println!("Stopped.");
}
