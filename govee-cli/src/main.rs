//! CLI frontend for controlling Govee LED strips over LAN.
//!
//! Supports one-shot commands (on/off, color, brightness) and continuous modes
//! (themes, screen capture, audio reactive, ambient wallpaper sync).

mod cli;
mod themes;
mod ambient;
mod screen;
mod audio_cmd;
mod sunlight;
mod ui;
mod dreamview;

use clap::Parser;
use govee_lan::*;
use std::process;
use std::time::Duration;

use cli::{Cli, Command};
use colored::Colorize;

/// Parsed device status response from the `devStatus` command.
#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DevStatus {
    #[serde(default)]
    on_off: i64,
    #[serde(default)]
    brightness: u8,
    #[serde(default)]
    color_tem_in_kelvin: i64,
    #[serde(default)]
    color: Option<StatusColor>,
    #[serde(default)]
    sku: Option<String>,
}

/// RGB color sub-field within a device status response.
#[derive(serde::Deserialize, Default)]
struct StatusColor {
    #[serde(default)]
    r: u8,
    #[serde(default)]
    g: u8,
    #[serde(default)]
    b: u8,
}

const SCAN_TIMEOUT: Duration = Duration::from_secs(2);

/// Global flag set to `false` by the Ctrl+C handler to signal graceful shutdown.
static RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Register the Ctrl+C handler (idempotent — safe to call multiple times).
fn ctrlc_setup() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        ctrlc::set_handler(|| {
            RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
        })
        .expect("Failed to set Ctrl+C handler");
    });
}

/// Resolve the device IP or exit with a user-friendly error if none found.
fn resolve_or_exit(ip: Option<&str>) -> String {
    let auto = ip.is_none();
    if auto {
        ui::banner();
        ui::discovery_scanning();
    }
    match resolve_ip(ip, SCAN_TIMEOUT) {
        Ok(ip) => {
            if auto {
                ui::discovery_found("device", &ip);
            }
            ip
        }
        Err(_) => {
            ui::error_hint(
                "No device found",
                "Is the strip powered on and connected to WiFi?",
            );
            process::exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();
    ui::set_quiet(cli.quiet);

    let ip = cli.ip;

    match cli.command {
        Command::Scan => {
            ui::discovery_scanning();
            let devices = scan_devices(SCAN_TIMEOUT);
            if devices.is_empty() {
                ui::error_hint("No devices found", "Ensure LAN API is enabled in the Govee Home app.");
                return;
            }
            for d in &devices {
                let name = if d.sku.is_empty() { "unknown" } else { &d.sku };
                ui::discovery_found(name, &d.ip);
                if !d.device.is_empty() || !d.wifi_version.is_empty() {
                    let details = format!(
                        "  {} {}",
                        if d.device.is_empty() { "" } else { &d.device },
                        format!(
                            "WiFi:{} BLE:{}",
                            if d.wifi_version.is_empty() { "?" } else { &d.wifi_version },
                            if d.ble_version.is_empty() { "?" } else { &d.ble_version }
                        ).dimmed()
                    );
                    ui::detail(&details);
                }
            }
        }
        Command::On => {
            let ip = resolve_or_exit(ip.as_deref());
            send_turn(&ip, true).ok();
            ui::info("Power", &format!("{}", "ON".green()));
        }
        Command::Off => {
            let ip = resolve_or_exit(ip.as_deref());
            send_turn(&ip, false).ok();
            ui::info("Power", &format!("{}", "OFF".red()));
        }
        Command::Brightness { value } => {
            if !(1..=100).contains(&value) {
                ui::error("Brightness must be 1-100");
                process::exit(1);
            }
            let ip = resolve_or_exit(ip.as_deref());
            send_brightness(&ip, value).ok();
            ui::info("Brightness", &ui::brightness_bar(value));
        }
        Command::Color { r, g, b } => {
            let ip = resolve_or_exit(ip.as_deref());
            send_color(&ip, r, g, b).ok();
            ui::info("Color", &ui::color_swatch(r, g, b));
        }
        Command::Temp { kelvin } => {
            if !(2000..=9000).contains(&kelvin) {
                ui::error("Color temperature must be 2000-9000K");
                process::exit(1);
            }
            let ip = resolve_or_exit(ip.as_deref());
            send_color_temp(&ip, kelvin).ok();
            ui::info("Temp", &format!("{}", format!("{kelvin}K").yellow()));
        }
        Command::Status => {
            let ip = resolve_or_exit(ip.as_deref());
            let status = send_command(&ip, "devStatus", serde_json::json!({}), cli.debug);
            match status {
                Some(data) => {
                    let s: DevStatus = serde_json::from_value(data).unwrap_or_default();
                    let power_str = if s.on_off == 1 {
                        format!("{}", "ON".green())
                    } else {
                        format!("{}", "OFF".red())
                    };
                    ui::info("Power", &power_str);
                    ui::info("Brightness", &ui::brightness_bar(s.brightness));

                    if s.color_tem_in_kelvin > 0 {
                        ui::info("Temp", &format!("{}", format!("{}K", s.color_tem_in_kelvin).yellow()));
                    } else if let Some(c) = &s.color {
                        ui::info("Color", &ui::color_swatch_full(c.r, c.g, c.b));
                    }

                    ui::info("Device", &format!("{} {}", ip.cyan(), s.sku.as_deref().unwrap_or("").dimmed()));
                }
                None => {
                    ui::error_hint(
                        &format!("No response from {ip}"),
                        "Is the device powered on?",
                    );
                    process::exit(1);
                }
            }
        }
        Command::Sleep => {
            let ip = resolve_or_exit(ip.as_deref());
            send_color(&ip, 0, 0, 0).ok();
            std::thread::sleep(Duration::from_millis(50));
            send_brightness(&ip, 1).ok();
            ui::info("Sleep", &format!("{}", "dark but responsive".dimmed()));
        }
        Command::Reset => {
            let ip = resolve_or_exit(ip.as_deref());
            let _ = razer_deactivate(&ip);
            send_turn(&ip, true).ok();
            std::thread::sleep(Duration::from_millis(50));
            send_brightness(&ip, 100).ok();
            std::thread::sleep(Duration::from_millis(50));
            send_color_temp(&ip, 4000).ok();
            ui::info("Reset", &format!("{}", "on · 100% · 4000K warm white".dimmed()));
        }
        Command::Theme { name, brightness, segments } => {
            themes::run_theme(&name.to_lowercase(), ip, brightness, segments, cli.mirror);
        }
        Command::Ambient(args) => ambient::run_ambient(args, ip),
        Command::Screen(args) => screen::run_screen(args, ip, cli.mirror),
        Command::Audio(args) => audio_cmd::run_audio(args, ip, cli.mirror),
        Command::Sunlight(args) => sunlight::run_sunlight(args, ip, cli.mirror),
    }
}
