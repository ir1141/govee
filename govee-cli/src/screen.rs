use govee_lan::*;
use govee_lan::wayland::ScreenCapturer;
use govee_lan::UdpSender;
use std::process;
use std::time::{Duration, Instant};

use crate::cli::ScreenArgs;
use crate::{RUNNING, ctrlc_setup, resolve_or_exit};

pub fn run_screen(args: ScreenArgs, ip: Option<String>, mirror: bool) {
    let mut capturer = match ScreenCapturer::new() {
        Ok(c) => c,
        Err(e) => {
            crate::ui::error_hint(
                &format!("Failed to initialize Wayland capture: {e}"),
                "Make sure your compositor supports wlr-screencopy-unstable-v1",
            );
            process::exit(1);
        }
    };

    if args.verbose {
        println!("Available outputs: {:?}", capturer.outputs());
    }

    let ip = resolve_or_exit(ip.as_deref());
    let use_razer = !args.no_dreamview;
    let n_seg = crate::dreamview::segment_count(use_razer, args.segments);
    let interval = Duration::from_secs_f64(1.0 / args.fps.max(1) as f64);

    let sender = UdpSender::new(&ip).expect("Failed to create UDP sender");

    crate::dreamview::activate(&ip, args.brightness, use_razer);

    if use_razer {
        use colored::Colorize;
        let mode = format!(
            "DreamView ({n_seg} segments{})",
            if args.gradient { ", gradient" } else { "" }
        );
        crate::ui::info("Mode", &format!("{} {}", mode.white(), format!("~{}fps · smooth: {}", args.fps, args.smoothing).dimmed()));
        crate::ui::info("Brightness", &crate::ui::brightness_bar(args.brightness));
    } else {
        use colored::Colorize;
        crate::ui::info("Mode", &format!("{} {}", "single color".white(), format!("~{}fps · smooth: {}", args.fps, args.smoothing).dimmed()));
        crate::ui::info("Brightness", &crate::ui::brightness_bar(args.brightness));
    }
    {
        use colored::Colorize;
        println!("  {}", "Press Ctrl+C to stop".dimmed());
    }

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
                let send_colors = crate::dreamview::apply_mirror(&current_colors, mirror);
                let _ = sender.send_segments(&send_colors, args.gradient);
            } else {
                let (r, g, b) = current_colors[0];
                let _ = sender.send_color(r, g, b);
            }

            last_send_time = Instant::now();

            if any_changed {
                let meta = format!("{}fps · smooth: {}", args.fps, args.smoothing);
                crate::ui::status_line(&current_colors, &meta);
            }

            last_sent = current_colors;
        }

        let elapsed = t0.elapsed();
        if elapsed < interval {
            std::thread::sleep(interval - elapsed);
        }
    }

    crate::dreamview::shutdown(&ip, use_razer);
}
