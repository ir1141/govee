use clap::{Args, Parser, Subcommand};
use govee_lan::*;
use govee_lan::audio::{AudioAnalyzer, VisMode, Palette, map_colors};
use govee_lan::wayland::ScreenCapturer;
use inotify::{Inotify, WatchMask};
use rand::RngExt;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, Instant};

const SCAN_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Parser)]
#[command(name = "govee", about = "Control Govee LED strip lights over LAN")]
#[command(after_help = format!("Scenes: {}", SCENE_NAMES.join(", ")))]
struct Cli {
    #[arg(long, global = true, help = "Show raw UDP messages")]
    debug: bool,

    #[arg(long, global = true, help = "Mirror segments for U-shaped strip layout")]
    mirror: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Discover Govee devices on the network
    Scan,
    /// Turn on
    On {
        #[arg(long, help = "Device IP (auto-discovers if omitted)")]
        ip: Option<String>,
    },
    /// Turn off
    Off {
        #[arg(long, help = "Device IP (auto-discovers if omitted)")]
        ip: Option<String>,
    },
    /// Set brightness (1-100)
    Brightness {
        value: u8,
        #[arg(long)]
        ip: Option<String>,
    },
    /// Set RGB color
    Color {
        r: u8,
        g: u8,
        b: u8,
        #[arg(long)]
        ip: Option<String>,
    },
    /// Set color temperature (2000-9000K)
    Temp {
        kelvin: u16,
        #[arg(long)]
        ip: Option<String>,
    },
    /// Query device status
    Status {
        #[arg(long)]
        ip: Option<String>,
    },
    /// Dark but stays responsive (use instead of off)
    Sleep {
        #[arg(long)]
        ip: Option<String>,
    },
    /// Apply a scene (static or animated)
    Scene {
        name: String,
        #[arg(long)]
        ip: Option<String>,
        #[arg(long, default_value_t = 60, help = "Strip brightness 1-100")]
        brightness: u8,
    },
    /// Sync Govee strip with Caelestia wallpaper theme
    Ambient(AmbientArgs),
    /// Sync Govee strip with screen content (Ambilight-style)
    Screen(ScreenArgs),
    /// React to system audio with LED visualizations
    Audio(AudioArgs),
}

#[derive(Args)]
struct AmbientArgs {
    #[arg(long, help = "Device IP (auto-discovers if omitted)")]
    ip: Option<String>,

    #[arg(
        long,
        default_value = "primary",
        help = "Which theme color to use"
    )]
    color: String,

    #[arg(long, default_value_t = 80, help = "Strip brightness 1-100")]
    brightness: u8,

    #[arg(long, help = "Use the Dim variant of the color")]
    dim: bool,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,
}

#[derive(Args)]
struct ScreenArgs {
    #[arg(long, help = "Device IP (auto-discovers if omitted)")]
    ip: Option<String>,

    #[arg(long, default_value_t = 10, help = "Screen capture rate")]
    fps: u32,

    #[arg(long, default_value_t = 80, help = "Strip brightness 1-100")]
    brightness: u8,

    #[arg(
        long,
        default_value_t = 0.3,
        help = "Color transition speed 0.0-1.0 (higher=faster)"
    )]
    smoothing: f64,

    #[arg(long, default_value_t = 10, help = "Min color change to send update")]
    threshold: u32,

    #[arg(long, help = "Wayland output/monitor name (e.g. DP-1, HDMI-A-1)")]
    output: Option<String>,

    #[arg(
        long,
        default_value_t = 5,
        value_parser = |s: &str| -> Result<usize, String> {
            let v: usize = s.parse().map_err(|_| format!("invalid number '{s}'"))?;
            if v < 1 || v > 127 { return Err(format!("segments must be 1-127, got {v}")); }
            Ok(v)
        },
        help = "Number of color zones across top edge (max 127 for mirror support)"
    )]
    segments: usize,

    #[arg(long, help = "Interpolate between segment colors")]
    gradient: bool,

    #[arg(
        long,
        default_value_t = 1.0,
        help = "Boost color saturation (1.0=normal, 1.5=vivid)"
    )]
    saturate: f64,

    #[arg(
        long,
        help = "Use basic colorwc instead of DreamView (single color, no segments)"
    )]
    no_razer: bool,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,
}

#[derive(Args)]
struct AudioArgs {
    #[arg(long, help = "Device IP (auto-discovers if omitted)")]
    ip: Option<String>,

    #[arg(long, default_value = "energy", help = "Visualization mode")]
    mode: VisMode,

    #[arg(long, default_value = "fire", help = "Color palette")]
    palette: Palette,

    #[arg(long, default_value_t = 80, help = "Strip brightness 1-100")]
    brightness: u8,

    #[arg(long, default_value_t = 5, value_parser = |s: &str| -> Result<usize, String> {
        let v: usize = s.parse().map_err(|_| format!("invalid number '{s}'"))?;
        if v < 1 || v > 127 { return Err(format!("segments must be 1-127, got {v}")); }
        Ok(v)
    }, help = "Number of DreamView segments (max 127 for mirror support)")]
    segments: usize,

    #[arg(long, default_value_t = 0.3, help = "Color transition speed 0.0-1.0")]
    smoothing: f64,

    #[arg(long, default_value_t = 1.0, help = "Audio gain multiplier")]
    sensitivity: f64,

    #[arg(long, help = "Single-color mode instead of DreamView")]
    no_razer: bool,

    #[arg(long, help = "Interpolate between segment colors")]
    gradient: bool,

    #[arg(short, long, help = "Verbose output")]
    verbose: bool,
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

            // Check for animated scenes first
            const ANIMATED_SCENES: &[&str] = &[
                "fireplace", "storm", "ocean", "aurora", "lava", "breathing", "sunrise",
            ];
            if ANIMATED_SCENES.contains(&name_lower.as_str()) {
                run_animated_scene(&name_lower, ip, brightness, cli.mirror);
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
        Command::Ambient(args) => run_ambient(args),
        Command::Screen(args) => run_screen(args, cli.mirror),
        Command::Audio(args) => run_audio(args, cli.mirror),
    }
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

// --- Animated scenes ---

fn run_animated_scene(name: &str, ip: Option<String>, brightness: u8, mirror: bool) {
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
    // Deep blue/purple base with lightning flashes
    for v in intensity.iter_mut() {
        *v *= 0.85; // decay flashes
    }
    // Random lightning strike
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
                // Lightning — bright white/blue
                let v = (180.0 + flash * 75.0) as u8;
                (v, v, 255)
            } else {
                // Dark stormy base
                let r = (10.0 * base_pulse) as u8;
                let g = (5.0 * base_pulse) as u8;
                let b = (40.0 + 30.0 * base_pulse) as u8;
                (r, g, b)
            }
        })
        .collect()
}

fn scene_ocean(n_seg: usize, t: f64) -> Vec<(u8, u8, u8)> {
    // Rolling waves of blue-green shifting across segments
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
    // Slow-moving greens and purples drifting across
    (0..n_seg)
        .map(|i| {
            let drift = (t * 0.3 + i as f64 * 0.8).sin() * 0.5 + 0.5;
            let shimmer = (t * 1.5 + i as f64 * 2.0).sin() * 0.2 + 0.8;
            // Blend between green and purple based on drift
            let r = (drift * 120.0 * shimmer) as u8;
            let g = ((1.0 - drift) * 200.0 * shimmer) as u8;
            let b = (60.0 + drift * 140.0 * shimmer) as u8;
            (r, g, b)
        })
        .collect()
}

fn scene_lava(rng: &mut impl rand::RngExt, heat: &mut [f64], n_seg: usize) -> Vec<(u8, u8, u8)> {
    // Like fireplace but deeper reds with slow orange blobs
    for h in heat.iter_mut() {
        *h += rng.random_range(-0.08..0.08);
        *h = h.clamp(0.0, 1.0);
    }
    // Slow blob movement — blend neighbors
    let snapshot: Vec<f64> = heat.to_vec();
    for i in 0..n_seg {
        let left = if i > 0 { snapshot[i - 1] } else { snapshot[i] };
        let right = if i < n_seg - 1 { snapshot[i + 1] } else { snapshot[i] };
        heat[i] = snapshot[i] * 0.6 + left * 0.2 + right * 0.2;
    }
    // Random hot spot
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
    // Slow sine-wave pulse in warm amber
    let breath = ((t * 0.4).sin() * 0.5 + 0.5).powi(2); // ease in/out
    let r = (40.0 + breath * 200.0) as u8;
    let g = (10.0 + breath * 80.0) as u8;
    let b = (breath * 20.0) as u8;
    vec![(r, g, b); n_seg]
}

fn scene_sunrise(n_seg: usize, t: f64) -> Vec<(u8, u8, u8)> {
    // 10-minute gradual transition: deep red → orange → golden → warm white
    let progress = (t / 600.0).min(1.0); // 0→1 over 10 minutes
    (0..n_seg)
        .map(|i| {
            // Slight variation across segments
            let p = (progress + (i as f64 - n_seg as f64 / 2.0) * 0.02).clamp(0.0, 1.0);
            if p < 0.33 {
                // Deep red → orange
                let sub = p / 0.33;
                let r = (60.0 + sub * 195.0) as u8;
                let g = (sub * 80.0) as u8;
                let b = 0u8;
                (r, g, b)
            } else if p < 0.66 {
                // Orange → golden
                let sub = (p - 0.33) / 0.33;
                let r = 255;
                let g = (80.0 + sub * 120.0) as u8;
                let b = (sub * 30.0) as u8;
                (r, g, b)
            } else {
                // Golden → warm white
                let sub = (p - 0.66) / 0.34;
                let r = 255;
                let g = (200.0 + sub * 40.0) as u8;
                let b = (30.0 + sub * 150.0) as u8;
                (r, g, b)
            }
        })
        .collect()
}

// --- Ambient command ---

fn scheme_path() -> anyhow::Result<PathBuf> {
    Ok(dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .join(".local/state/caelestia/scheme.json"))
}

fn read_scheme_color(path: &std::path::Path, color_key: &str) -> Option<(u8, u8, u8)> {
    let text = std::fs::read_to_string(path).ok()?;
    let scheme: serde_json::Value = serde_json::from_str(&text).ok()?;
    let hex = scheme.get("colours")?.get(color_key)?.as_str()?;
    hex_to_rgb(hex)
}

fn run_ambient(args: AmbientArgs) {
    let valid_colors = [
        "primary",
        "secondary",
        "tertiary",
        "primaryContainer",
        "tertiaryContainer",
        "surfaceTint",
    ];
    if !valid_colors.contains(&args.color.as_str()) {
        eprintln!(
            "Invalid color '{}'. Available: {}",
            args.color,
            valid_colors.join(", ")
        );
        process::exit(1);
    }

    let ip = match resolve_ip(args.ip.as_deref(), SCAN_TIMEOUT) {
        Ok(ip) => ip,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };
    println!("Using device at {ip}");

    ctrlc_setup();

    let color_key = if args.dim {
        format!("{}Dim", args.color)
    } else {
        args.color.clone()
    };

    // Set initial brightness
    if let Err(e) = send_brightness(&ip, args.brightness) {
        eprintln!("Failed to set brightness: {e}");
    }

    let path = match scheme_path() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    // Apply current color immediately
    let mut last_rgb = None;
    if let Some((r, g, b)) = read_scheme_color(&path, &color_key) {
        if let Err(e) = send_color(&ip, r, g, b) {
            eprintln!("Failed to send color: {e}");
        }
        if args.verbose {
            println!("Initial color: ({r}, {g}, {b}) from {color_key}");
        }
        last_rgb = Some((r, g, b));
    }

    println!(
        "Watching {} for theme changes (Ctrl+C to stop)",
        path.display()
    );
    println!("Color key: {color_key} | Brightness: {}%", args.brightness);

    // Watch for scheme changes with inotify
    let mut inotify = Inotify::init().expect("Failed to initialize inotify");

    // Watch the parent directory (file may be replaced atomically)
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
        // Poll with 1-second timeout so we can check RUNNING
        use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
        let borrowed_fd = unsafe { std::os::fd::BorrowedFd::borrow_raw(raw_fd) };
        let mut poll_fds = [PollFd::new(borrowed_fd, PollFlags::POLLIN)];
        match poll(&mut poll_fds, PollTimeout::from(1000u16)) {
            Ok(0) => continue, // timeout, recheck RUNNING
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

                // Small delay for file to settle
                std::thread::sleep(Duration::from_millis(100));

                if let Some((r, g, b)) = read_scheme_color(&path, &color_key) {
                    let rgb = (r, g, b);
                    if Some(rgb) != last_rgb {
                        if let Err(e) = send_color(&ip, r, g, b) {
                            eprintln!("Failed to send color: {e}");
                        }
                        last_rgb = Some(rgb);
                        if args.verbose {
                            println!("Updated: ({r}, {g}, {b})");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("inotify error: {e}");
                break;
            }
        }
    }
    println!("\nStopped.");
}

// --- Screen command ---

static RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

fn ctrlc_setup() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        ctrlc::set_handler(|| {
            RUNNING.store(false, std::sync::atomic::Ordering::SeqCst);
        }).expect("Failed to set Ctrl+C handler");
    });
}

fn run_screen(args: ScreenArgs, mirror: bool) {
    // Initialize Wayland screen capture
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

    // Resolve device IP
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

    // Set brightness
    if let Err(e) = send_brightness(&ip, args.brightness) {
        eprintln!("Failed to set brightness: {e}");
    }

    // Activate DreamView mode
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

    // Install ctrl-c handler
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

    // Cleanup
    println!();
    if use_razer {
        println!("Deactivating DreamView mode...");
        let _ = razer_deactivate(&ip);
    }
    println!("Stopped.");
}

fn run_audio(args: AudioArgs, mirror: bool) {
    let ip = resolve_or_exit(args.ip.as_deref());
    println!("Using device at {ip}");

    let use_razer = !args.no_razer;
    let n_seg = if use_razer { args.segments.max(1) } else { 1 };

    // Set brightness
    if let Err(e) = send_brightness(&ip, args.brightness) {
        eprintln!("Failed to set brightness: {e}");
    }

    // Activate DreamView
    if use_razer {
        if let Err(e) = razer_activate(&ip) {
            eprintln!("Failed to activate DreamView: {e}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Start audio capture
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

        // Apply sensitivity
        audio.energy = (audio.energy * args.sensitivity).clamp(0.0, 1.0);
        for band in audio.bands.iter_mut() {
            *band = (*band * args.sensitivity).clamp(0.0, 1.0);
        }

        // Map to colors
        let raw_colors = map_colors(
            &audio,
            args.mode,
            args.palette,
            n_seg,
            t,
            &mut beat_hue,
            &mut beat_decay,
        );

        // Smooth transitions
        let mut current_colors = Vec::with_capacity(n_seg);
        for i in 0..n_seg {
            smoothed[i] = smooth(smoothed[i], raw_colors[i], args.smoothing);
            current_colors.push((
                smoothed[i].0 as u8,
                smoothed[i].1 as u8,
                smoothed[i].2 as u8,
            ));
        }

        // Mirror if needed
        let send_colors = if mirror {
            let mut mirrored = current_colors.clone();
            mirrored.extend(current_colors.iter().rev());
            mirrored
        } else {
            current_colors.clone()
        };

        // Send to device
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

    // Cleanup
    println!();
    drop(analyzer);
    if use_razer {
        println!("Deactivating DreamView mode...");
        let _ = razer_deactivate(&ip);
    }
    println!("Stopped.");
}
