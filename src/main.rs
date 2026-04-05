mod cli;
mod scenes;
mod ambient;
mod screen;
mod audio_cmd;

use clap::Parser;
use govee_lan::*;
use std::process;
use std::time::Duration;

use cli::{Cli, Command};
use scenes::ANIMATED_SCENES;

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
    match resolve_ip(ip, SCAN_TIMEOUT) {
        Ok(ip) => ip,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan => {
            let devices = scan_devices(SCAN_TIMEOUT);
            if devices.is_empty() {
                println!("No devices found. Ensure LAN API is enabled in the Govee Home app.");
                return;
            }
            println!("Found {} device(s):\n", devices.len());
            for d in &devices {
                println!("  IP:     {}", d.ip);
                println!("  SKU:    {}", if d.sku.is_empty() { "unknown" } else { &d.sku });
                println!("  Device: {}", if d.device.is_empty() { "unknown" } else { &d.device });
                println!(
                    "  WiFi:   {}  BLE: {}",
                    if d.wifi_version.is_empty() { "?" } else { &d.wifi_version },
                    if d.ble_version.is_empty() { "?" } else { &d.ble_version }
                );
                println!();
            }
        }
        Command::On { ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            send_command(&ip, "turn", serde_json::json!({"value": 1}), cli.debug);
            println!("Turned ON ({ip})");
        }
        Command::Off { ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            send_command(&ip, "turn", serde_json::json!({"value": 0}), cli.debug);
            println!("Turned OFF ({ip})");
        }
        Command::Brightness { value, ip } => {
            if !(1..=100).contains(&value) {
                eprintln!("Brightness must be 1-100");
                process::exit(1);
            }
            let ip = resolve_or_exit(ip.as_deref());
            send_command(&ip, "brightness", serde_json::json!({"value": value}), cli.debug);
            println!("Brightness set to {value}% ({ip})");
        }
        Command::Color { r, g, b, ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            send_command(
                &ip,
                "colorwc",
                serde_json::json!({"color": {"r": r, "g": g, "b": b}, "colorTemInKelvin": 0}),
                cli.debug,
            );
            println!("Color set to ({r}, {g}, {b}) ({ip})");
        }
        Command::Temp { kelvin, ip } => {
            if !(2000..=9000).contains(&kelvin) {
                eprintln!("Color temperature must be 2000-9000K");
                process::exit(1);
            }
            let ip = resolve_or_exit(ip.as_deref());
            send_command(
                &ip,
                "colorwc",
                serde_json::json!({"color": {"r": 0, "g": 0, "b": 0}, "colorTemInKelvin": kelvin}),
                cli.debug,
            );
            println!("Color temperature set to {kelvin}K ({ip})");
        }
        Command::Status { ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            let status = send_command(&ip, "devStatus", serde_json::json!({}), cli.debug);
            match status {
                Some(data) => {
                    let on_off = if data.get("onOff").and_then(|v| v.as_i64()) == Some(1) {
                        "ON"
                    } else {
                        "OFF"
                    };
                    let brightness = data
                        .get("brightness")
                        .and_then(|v| v.as_i64())
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "?".into());
                    let temp = data
                        .get("colorTemInKelvin")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    println!("  Power:       {on_off}");
                    println!("  Brightness:  {brightness}%");
                    if temp > 0 {
                        println!("  Color Temp:  {temp}K");
                    } else {
                        let color = data.get("color").cloned().unwrap_or(serde_json::json!({}));
                        let r = color.get("r").and_then(|v| v.as_i64()).unwrap_or(0);
                        let g = color.get("g").and_then(|v| v.as_i64()).unwrap_or(0);
                        let b = color.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
                        println!("  Color:       ({r}, {g}, {b})");
                    }
                }
                None => {
                    eprintln!("No response from {ip}");
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
            println!("Sleep mode (dark but responsive) ({ip})");
        }
        Command::Scene { name, ip, brightness } => {
            let name_lower = name.to_lowercase();

            if ANIMATED_SCENES.contains(&name_lower.as_str()) {
                scenes::run_animated_scene(&name_lower, ip, brightness, cli.mirror);
                return;
            }

            let scene = match get_scene(&name_lower) {
                Some(s) => s,
                None => {
                    let all: Vec<&str> = SCENE_NAMES.iter()
                        .chain(ANIMATED_SCENES.iter())
                        .copied()
                        .collect();
                    eprintln!(
                        "Unknown scene '{name}'. Available: {}",
                        all.join(", ")
                    );
                    process::exit(1);
                }
            };
            let ip = resolve_or_exit(ip.as_deref());
            send_command(&ip, "turn", serde_json::json!({"value": 1}), cli.debug);
            send_command(&ip, "brightness", serde_json::json!({"value": brightness}), cli.debug);
            if scene.temp > 0 {
                send_command(
                    &ip,
                    "colorwc",
                    serde_json::json!({"color": {"r": 0, "g": 0, "b": 0}, "colorTemInKelvin": scene.temp}),
                    cli.debug,
                );
            } else {
                send_command(
                    &ip,
                    "colorwc",
                    serde_json::json!({"color": {"r": scene.r, "g": scene.g, "b": scene.b}, "colorTemInKelvin": 0}),
                    cli.debug,
                );
            }
            println!("Scene '{name_lower}' applied ({ip})");
        }
        Command::Ambient(args) => ambient::run_ambient(args),
        Command::Screen(args) => screen::run_screen(args, cli.mirror),
        Command::Audio(args) => audio_cmd::run_audio(args, cli.mirror),
    }
}
