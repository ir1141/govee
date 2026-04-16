//! Screen capture mode: captures Wayland frames, extracts per-segment colors
//! with oversampling, smooths transitions, and streams to the strip via DreamView.

use govee_lan::*;
use govee_lan::wayland::ScreenCapturer;
use govee_lan::UdpSender;
use std::process;
use std::time::{Duration, Instant};

use crate::cli::ScreenArgs;
use crate::{RUNNING, ctrlc_setup, resolve_or_exit};

/// Oversample multiplier: sample this many screen columns per hardware segment,
/// then merge for finer color resolution within the device's segment limit.
const OVERSAMPLE: usize = 4;

/// Merge oversampled colors down to `n` segments by averaging each group.
/// Splits the input into `n` contiguous slices using integer scaling so that
/// remainder pixels are distributed evenly instead of dropped, and tolerates
/// `colors.len() < n` (including empty) without panicking.
fn merge_segments(colors: &[(u8, u8, u8)], n: usize) -> Vec<(u8, u8, u8)> {
    if n == 0 {
        return Vec::new();
    }
    if colors.is_empty() {
        return vec![(0, 0, 0); n];
    }
    let len = colors.len();
    (0..n)
        .map(|i| {
            let start = len * i / n;
            let mut end = len * (i + 1) / n;
            if end <= start {
                end = (start + 1).min(len);
            }
            let chunk = &colors[start..end];
            let (r, g, b) = chunk.iter().fold((0u32, 0u32, 0u32), |(r, g, b), &(cr, cg, cb)| {
                (r + cr as u32, g + cg as u32, b + cb as u32)
            });
            let clen = chunk.len() as u32;
            ((r / clen) as u8, (g / clen) as u8, (b / clen) as u8)
        })
        .collect()
}

/// Run the screen capture loop, streaming segment colors to the device.
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
    crate::ui::ctrlc_hint();

    let mut smoothed: Vec<(f64, f64, f64)> = vec![(0.0, 0.0, 0.0); n_seg];
    let mut last_sent: Vec<(u8, u8, u8)> = vec![(0, 0, 0); n_seg];
    let mut last_send_time = Instant::now();
    let keepalive_interval = Duration::from_secs(2);

    ctrlc_setup();

    // Seed smoothed colors
    if let Ok(frame) = capturer.capture(args.output.as_deref()) {
        let colors = merge_segments(&frame.extract_segment_colors(n_seg * OVERSAMPLE), n_seg);
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

        let raw_colors = merge_segments(&frame.extract_segment_colors(n_seg * OVERSAMPLE), n_seg);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_segments_even_chunks() {
        let colors = vec![
            (10, 0, 0), (30, 0, 0),
            (0, 10, 0), (0, 30, 0),
        ];
        assert_eq!(merge_segments(&colors, 2), vec![(20, 0, 0), (0, 20, 0)]);
    }

    #[test]
    fn merge_segments_fewer_inputs_than_n() {
        // Should not panic and should produce n outputs.
        let colors = vec![(100, 100, 100), (200, 200, 200)];
        let out = merge_segments(&colors, 5);
        assert_eq!(out.len(), 5);
    }

    #[test]
    fn merge_segments_empty_input() {
        let out = merge_segments(&[], 4);
        assert_eq!(out, vec![(0, 0, 0); 4]);
    }

    #[test]
    fn merge_segments_zero_n() {
        assert!(merge_segments(&[(1, 2, 3)], 0).is_empty());
    }
}
