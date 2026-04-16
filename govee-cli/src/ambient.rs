//! Ambient sync mode: watches Caelestia's `scheme.json` via inotify and updates
//! the LED strip color when the desktop wallpaper theme changes.

use govee_lan::{hex_to_rgb, send_brightness, send_color};
use inotify::{Inotify, WatchMask};
use std::path::PathBuf;
use std::process;
use std::time::Duration;

use crate::cli::AmbientArgs;
use crate::{RUNNING, ctrlc_setup};

/// Run the ambient sync loop, watching for wallpaper theme changes.
pub fn run_ambient(args: AmbientArgs, ip: Option<String>) {
    let valid_colors = [
        "primary",
        "secondary",
        "tertiary",
        "primaryContainer",
        "tertiaryContainer",
        "surfaceTint",
    ];
    if !valid_colors.contains(&args.color.as_str()) {
        crate::ui::error_hint(
            &format!("Invalid color '{}'", args.color),
            &format!("Available: {}", valid_colors.join(", ")),
        );
        process::exit(1);
    }

    let ip = crate::resolve_or_exit(ip.as_deref());

    ctrlc_setup();

    let color_key = if args.dim {
        format!("{}Dim", args.color)
    } else {
        args.color.clone()
    };

    if let Err(e) = send_brightness(&ip, args.brightness) {
        crate::ui::error(&format!("Failed to set brightness: {e}"));
    }

    let path = match scheme_path() {
        Ok(p) => p,
        Err(e) => {
            crate::ui::error(&format!("{e}"));
            process::exit(1);
        }
    };

    let mut last_rgb = None;
    if let Some((r, g, b)) = read_scheme_color(&path, &color_key) {
        if let Err(e) = send_color(&ip, r, g, b) {
            crate::ui::error(&format!("Failed to send color: {e}"));
        }
        if args.verbose {
            crate::ui::info("Color", &crate::ui::color_swatch_full(r, g, b));
        }
        last_rgb = Some((r, g, b));
    }

    crate::ui::info("Watching", &format!("{}", path.display()));
    crate::ui::info("Color key", &color_key);
    crate::ui::info("Brightness", &crate::ui::brightness_bar(args.brightness));
    crate::ui::ctrlc_hint();

    let mut inotify = Inotify::init().expect("Failed to initialize inotify");

    let watch_path = path.parent().unwrap_or(&path);
    inotify
        .watches()
        .add(
            watch_path,
            WatchMask::MODIFY | WatchMask::CLOSE_WRITE | WatchMask::MOVED_TO,
        )
        .expect("Failed to add inotify watch");

    use std::os::unix::io::AsRawFd;
    let raw_fd = inotify.as_raw_fd();
    let mut buffer = [0u8; 4096];
    while RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
        use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
        // SAFETY: `raw_fd` is from `inotify.as_raw_fd()` above and `inotify` lives
        // for the entire loop. The BorrowedFd does not outlive the owning Inotify.
        let borrowed_fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(raw_fd) };
        let mut poll_fds = [PollFd::new(borrowed_fd, PollFlags::POLLIN)];
        match poll(&mut poll_fds, PollTimeout::from(1000u16)) {
            Ok(0) => continue,
            Err(_) => break,
            Ok(_) => {}
        }
        match inotify.read_events(&mut buffer) {
            Ok(events) => {
                let scheme_changed = events.into_iter().any(|ev| {
                    ev.name
                        .map(|n| n.to_string_lossy().contains("scheme.json"))
                        .unwrap_or(false)
                });
                if !scheme_changed {
                    continue;
                }

                std::thread::sleep(Duration::from_millis(100));

                if let Some((r, g, b)) = read_scheme_color(&path, &color_key) {
                    let rgb = (r, g, b);
                    if Some(rgb) != last_rgb {
                        if let Err(e) = send_color(&ip, r, g, b) {
                            crate::ui::error(&format!("Failed to send color: {e}"));
                        }
                        last_rgb = Some(rgb);
                        if args.verbose {
                            crate::ui::info("Updated", &crate::ui::color_swatch(r, g, b));
                        }
                    }
                }
            }
            Err(e) => {
                crate::ui::error(&format!("inotify error: {e}"));
                break;
            }
        }
    }
    crate::ui::status_line_finish();
    crate::ui::stopped();
}

/// Path to Caelestia's wallpaper color scheme file.
fn scheme_path() -> anyhow::Result<PathBuf> {
    Ok(dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".local/state/caelestia/scheme.json"))
}

/// Read a hex color from the scheme JSON at the given key.
fn read_scheme_color(path: &std::path::Path, color_key: &str) -> Option<(u8, u8, u8)> {
    let text = std::fs::read_to_string(path).ok()?;
    let scheme: serde_json::Value = serde_json::from_str(&text).ok()?;
    let hex = scheme.get("colours")?.get(color_key)?.as_str()?;
    hex_to_rgb(hex)
}
