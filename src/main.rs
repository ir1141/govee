mod cli;
mod theme_defs;
mod theme_loader;
mod themes;
mod ambient;
mod screen;
mod audio_cmd;
mod ui;

use clap::Parser;
use govee_lan::*;
use std::process;
use std::time::Duration;

use cli::{Cli, Command};
use colored::Colorize;

const SCAN_TIMEOUT: Duration = Duration::from_secs(2);

static RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

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
                    println!("{details}");
                }
            }
        }
        Command::On { ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            send_command(&ip, "turn", serde_json::json!({"value": 1}), cli.debug);
            ui::info("Power", &format!("{}", "ON".green()));
        }
        Command::Off { ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            send_command(&ip, "turn", serde_json::json!({"value": 0}), cli.debug);
            ui::info("Power", &format!("{}", "OFF".red()));
        }
        Command::Brightness { value, ip } => {
            if !(1..=100).contains(&value) {
                ui::error("Brightness must be 1-100");
                process::exit(1);
            }
            let ip = resolve_or_exit(ip.as_deref());
            send_command(&ip, "brightness", serde_json::json!({"value": value}), cli.debug);
            ui::info("Brightness", &ui::brightness_bar(value));
        }
        Command::Color { r, g, b, ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            send_command(
                &ip,
                "colorwc",
                serde_json::json!({"color": {"r": r, "g": g, "b": b}, "colorTemInKelvin": 0}),
                cli.debug,
            );
            ui::info("Color", &ui::color_swatch(r, g, b));
        }
        Command::Temp { kelvin, ip } => {
            if !(2000..=9000).contains(&kelvin) {
                ui::error("Color temperature must be 2000-9000K");
                process::exit(1);
            }
            let ip = resolve_or_exit(ip.as_deref());
            send_command(
                &ip,
                "colorwc",
                serde_json::json!({"color": {"r": 0, "g": 0, "b": 0}, "colorTemInKelvin": kelvin}),
                cli.debug,
            );
            ui::info("Temp", &format!("{}", format!("{kelvin}K").yellow()));
        }
        Command::Status { ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            let status = send_command(&ip, "devStatus", serde_json::json!({}), cli.debug);
            match status {
                Some(data) => {
                    let on = data.get("onOff").and_then(|v| v.as_i64()) == Some(1);
                    let power_str = if on {
                        format!("{}", "ON".green())
                    } else {
                        format!("{}", "OFF".red())
                    };
                    ui::info("Power", &power_str);

                    let brightness = data
                        .get("brightness")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as u8;
                    ui::info("Brightness", &ui::brightness_bar(brightness));

                    let temp = data
                        .get("colorTemInKelvin")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);
                    if temp > 0 {
                        ui::info("Temp", &format!("{}", format!("{temp}K").yellow()));
                    } else {
                        let color = data.get("color").cloned().unwrap_or(serde_json::json!({}));
                        let r = color.get("r").and_then(|v| v.as_i64()).unwrap_or(0) as u8;
                        let g = color.get("g").and_then(|v| v.as_i64()).unwrap_or(0) as u8;
                        let b = color.get("b").and_then(|v| v.as_i64()).unwrap_or(0) as u8;
                        ui::info("Color", &ui::color_swatch_full(r, g, b));
                    }

                    ui::info("Device", &format!("{} {}", ip.cyan(), data.get("sku").and_then(|v| v.as_str()).unwrap_or("").dimmed()));
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
        Command::Sleep { ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            send_command(
                &ip,
                "colorwc",
                serde_json::json!({"color": {"r": 0, "g": 0, "b": 0}, "colorTemInKelvin": 0}),
                cli.debug,
            );
            send_command(&ip, "brightness", serde_json::json!({"value": 1}), cli.debug);
            ui::info("Sleep", &format!("{}", "dark but responsive".dimmed()));
        }
        Command::Reset { ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            let _ = razer_deactivate(&ip);
            send_command(&ip, "turn", serde_json::json!({"value": 1}), cli.debug);
            send_command(&ip, "brightness", serde_json::json!({"value": 100}), cli.debug);
            send_command(
                &ip,
                "colorwc",
                serde_json::json!({"color": {"r": 0, "g": 0, "b": 0}, "colorTemInKelvin": 4000}),
                cli.debug,
            );
            ui::info("Reset", &format!("{}", "on · 100% · 4000K warm white".dimmed()));
        }
        Command::Theme { name, ip, brightness, segments } => {
            themes::run_theme(&name.to_lowercase(), ip, brightness, segments, cli.mirror, cli.debug);
        }
        Command::Ambient(args) => ambient::run_ambient(args),
        Command::Screen(args) => screen::run_screen(args, cli.mirror),
        Command::Audio(args) => audio_cmd::run_audio(args, cli.mirror),
    }
}
