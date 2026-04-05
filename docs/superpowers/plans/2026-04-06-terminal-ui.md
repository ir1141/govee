# Terminal UI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the "Branded Accent" visual identity to all govee CLI output — purple diamonds, colored bars, segment blocks, live status line.

**Architecture:** New `src/ui.rs` module centralizes all formatting via the `colored` crate. Every existing `println!`/`eprintln!` for user-facing output gets replaced with `ui::*` calls. Debug (`--debug`) raw protocol output stays plain.

**Tech Stack:** Rust, `colored ^2` crate (ANSI terminal styling with truecolor support)

---

### Task 1: Add `colored` dependency and create `src/ui.rs` with core formatting functions

**Files:**
- Modify: `Cargo.toml` (add `colored` dependency)
- Create: `src/ui.rs`
- Modify: `src/main.rs:1` (add `mod ui;`)

- [ ] **Step 1: Add `colored` to Cargo.toml**

Add after the `ctrlc` line in `[dependencies]`:

```toml
colored = "2"
```

- [ ] **Step 2: Create `src/ui.rs` with all formatting functions**

```rust
use colored::Colorize;

const DIAMOND: &str = "◆";
const ERROR_X: &str = "✖";
const FILLED: char = '█';
const EMPTY: char = '░';

// ── Banner ─────────────────────────────────────────────────────────────────

pub fn banner() {
    println!(
        "{} {} {}",
        "░▒▓".purple(),
        "govee".purple().bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).dimmed()
    );
    println!("{}", "LAN control · no cloud · no keys".dimmed());
    println!("{}", "─────────────────────────────────".dimmed());
}

// ── Info / status lines ────────────────────────────────────────────────────

pub fn info(label: &str, value: &str) {
    println!("{} {} {}", DIAMOND.purple(), label, value);
}

// ── Errors ─────────────────────────────────────────────────────────────────

pub fn error(msg: &str) {
    eprintln!("{} {}", ERROR_X.red(), msg.red());
}

pub fn error_hint(msg: &str, hint: &str) {
    eprintln!("{} {}", ERROR_X.red(), msg.red());
    eprintln!("  {}", hint.dimmed());
}

// ── Brightness bar ─────────────────────────────────────────────────────────

pub fn brightness_bar(percent: u8) -> String {
    let filled = (percent as usize + 9) / 10; // round up: 1% = 1 block, 100% = 10
    let filled = filled.min(10);
    let empty = 10 - filled;
    format!(
        "{}{} {}",
        FILLED.to_string().repeat(filled).yellow(),
        EMPTY.to_string().repeat(empty).dimmed(),
        format!("{percent}%").yellow()
    )
}

// ── Color swatch ───────────────────────────────────────────────────────────

pub fn color_swatch(r: u8, g: u8, b: u8) -> String {
    format!(
        "{} {}",
        "██".truecolor(r, g, b),
        format!("#{r:02X}{g:02X}{b:02X}").dimmed()
    )
}

pub fn color_swatch_full(r: u8, g: u8, b: u8) -> String {
    format!(
        "{} {} {}",
        "██".truecolor(r, g, b),
        format!("#{r:02X}{g:02X}{b:02X}").dimmed(),
        format!("({r}, {g}, {b})").dimmed()
    )
}

// ── Segment blocks ─────────────────────────────────────────────────────────

pub fn segment_blocks(colors: &[(u8, u8, u8)]) -> String {
    colors
        .iter()
        .map(|&(r, g, b)| format!("{}", "██".truecolor(r, g, b)))
        .collect::<String>()
}

// ── Live status line (continuous modes) ────────────────────────────────────

pub fn status_line(segments: &[(u8, u8, u8)], meta: &str) {
    let blocks = segment_blocks(segments);
    print!("\r{} {}", blocks, meta.dimmed());
    use std::io::Write;
    std::io::stdout().flush().ok();
}

pub fn status_line_finish() {
    println!();
}

// ── Discovery ──────────────────────────────────────────────────────────────

pub fn discovery_scanning() {
    eprintln!("{} {}", DIAMOND.purple(), "Scanning for devices...".dimmed());
}

pub fn discovery_found(name: &str, ip: &str) {
    eprintln!(
        "{} {} {} {} {}",
        DIAMOND.cyan(),
        "Found",
        name.white().bold(),
        "at".dimmed(),
        ip.cyan()
    );
}

// ── Theme list ─────────────────────────────────────────────────────────────

fn category_color(category: &str) -> colored::Color {
    match category {
        "static" => colored::Color::Magenta,
        "nature" => colored::Color::Cyan,
        "vibes" => colored::Color::Yellow,
        "functional" => colored::Color::Green,
        "seasonal" => colored::Color::Red,
        _ => colored::Color::White,
    }
}

fn category_label(category: &str) -> &str {
    match category {
        "static" => "STATIC",
        "nature" => "NATURE",
        "vibes" => "VIBES",
        "functional" => "FUNCTIONAL",
        "seasonal" => "SEASONAL",
        _ => category,
    }
}

pub fn theme_list(themes: &[(& str, &str)]) {
    println!("{}", "THEMES".purple().bold());
    let categories = ["static", "nature", "vibes", "functional", "seasonal"];
    for cat in &categories {
        let names: Vec<&str> = themes
            .iter()
            .filter(|(_, c)| c == cat)
            .map(|(n, _)| *n)
            .collect();
        if names.is_empty() {
            continue;
        }
        let color = category_color(cat);
        let border = "│".color(color);
        let label = category_label(cat).color(color);
        let joined = names.join(&format!(" {} ", "·".dimmed()));
        println!("{border} {label}");
        println!("{border} {joined}");
    }
}

/// Returns theme list as a plain string for clap help text.
/// Uses ANSI colors — `colored` auto-disables when not a TTY.
pub fn theme_list_help(themes: &[(&str, &str)]) -> String {
    let categories = ["static", "nature", "vibes", "functional", "seasonal"];
    let mut out = format!("{}\n", "THEMES".purple().bold());
    for cat in &categories {
        let names: Vec<&str> = themes
            .iter()
            .filter(|(_, c)| c == cat)
            .map(|(n, _)| *n)
            .collect();
        if names.is_empty() {
            continue;
        }
        let color = category_color(cat);
        let border = "│".color(color);
        let label = category_label(cat).color(color);
        let joined = names.join(&format!(" {} ", "·".dimmed()));
        out.push_str(&format!("{border} {label}\n"));
        out.push_str(&format!("{border} {joined}\n"));
    }
    out
}

// ── Shutdown messages ──────────────────────────────────────────────────────

pub fn deactivating() {
    println!("{}", "Deactivating DreamView mode...".dimmed());
}

pub fn stopped() {
    println!("{}", "Stopped.".dimmed());
}
```

- [ ] **Step 3: Add `mod ui;` to `src/main.rs`**

Add `mod ui;` after the existing module declarations at line 1:

```rust
mod cli;
mod themes;
mod ambient;
mod screen;
mod audio_cmd;
mod ui;
```

- [ ] **Step 4: Build to verify compilation**

Run: `cargo build 2>&1`
Expected: compiles successfully with no errors (warnings about unused functions are fine at this stage)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/ui.rs src/main.rs
git commit -m "feat: add ui module with branded accent formatting functions"
```

---

### Task 2: Integrate UI into `discovery.rs`

**Files:**
- Modify: `src/discovery.rs:82-88`

- [ ] **Step 1: Replace discovery output with UI calls**

In `src/discovery.rs`, the `resolve_ip` function (line 78-90) currently uses `eprintln!` directly. Replace lines 82-88:

Old code:
```rust
    eprintln!("Scanning for devices...");
    match discover_device(timeout) {
        Some(ip) => {
            eprintln!("Found device at {ip}");
            Ok(ip)
        }
```

New code:
```rust
    crate::ui::discovery_scanning();
    match discover_device(timeout) {
        Some(ip) => {
            crate::ui::discovery_found("device", &ip);
            Ok(ip)
        }
```

Note: `discovery.rs` is in the `govee_lan` library crate (`src/lib.rs`), not the binary. It uses `crate::` which refers to the library. Since `ui` is in the binary, we can't call `crate::ui` from here. Instead, keep discovery output as-is and override at the call site. **Correction**: Actually, `discovery.rs` IS part of the library (`lib.rs` exports it). The `ui` module is in the binary. So we need to handle this differently.

**Revised approach**: Leave `discovery.rs` unchanged. Override discovery messages at the binary level. In `src/main.rs`, the `resolve_or_exit` function calls `resolve_ip` which prints to stderr. We'll redirect by changing how we resolve IPs. The simplest approach: just let the current discovery messages stay (they go to stderr), and add styled output on top in the binary. Actually, let's just update the eprintln messages in discovery.rs to be slightly better but keep them plain since they're in the lib crate. The branded styling happens in the binary modules.

**Actually**, the cleanest fix: move the user-facing messages out of `discovery.rs` (library) and into the binary's `resolve_or_exit`. The library function should just return results silently.

Replace `resolve_ip` in `src/discovery.rs` (lines 78-90):

Old:
```rust
pub fn resolve_ip(ip: Option<&str>, timeout: Duration) -> Result<String> {
    if let Some(ip) = ip {
        return Ok(ip.to_string());
    }
    eprintln!("Scanning for devices...");
    match discover_device(timeout) {
        Some(ip) => {
            eprintln!("Found device at {ip}");
            Ok(ip)
        }
        None => anyhow::bail!("No Govee devices found. Make sure LAN API is enabled in the Govee app."),
    }
}
```

New:
```rust
pub fn resolve_ip(ip: Option<&str>, timeout: Duration) -> Result<String> {
    if let Some(ip) = ip {
        return Ok(ip.to_string());
    }
    match discover_device(timeout) {
        Some(ip) => Ok(ip),
        None => anyhow::bail!("No Govee devices found"),
    }
}
```

- [ ] **Step 2: Update `resolve_or_exit` in `src/main.rs` to show styled discovery messages**

Replace `resolve_or_exit` (lines 29-37):

Old:
```rust
fn resolve_or_exit(ip: Option<&str>) -> String {
    match resolve_ip(ip, SCAN_TIMEOUT) {
        Ok(ip) => ip,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    }
}
```

New:
```rust
fn resolve_or_exit(ip: Option<&str>) -> String {
    if ip.is_none() {
        ui::discovery_scanning();
    }
    match resolve_ip(ip, SCAN_TIMEOUT) {
        Ok(ip) => {
            if ip.as_str() != ip.as_str() { unreachable!() } // appease unused warning
            ui::discovery_found("device", &ip);
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
```

Wait, that has a nonsensical line. Simpler:

```rust
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
```

- [ ] **Step 3: Build and verify**

Run: `cargo build 2>&1`
Expected: compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/discovery.rs src/main.rs
git commit -m "feat(ui): style device discovery messages"
```

---

### Task 3: Integrate UI into `src/main.rs` — one-shot commands

**Files:**
- Modify: `src/main.rs:42-174`

- [ ] **Step 1: Replace Scan output (lines 44-61)**

Old:
```rust
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
```

New:
```rust
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
                    use colored::Colorize;
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
```

- [ ] **Step 2: Replace On/Off output (lines 62-71)**

Old:
```rust
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
```

New:
```rust
        Command::On { ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            send_command(&ip, "turn", serde_json::json!({"value": 1}), cli.debug);
            use colored::Colorize;
            ui::info("Power", &format!("{}", "ON".green()));
        }
        Command::Off { ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            send_command(&ip, "turn", serde_json::json!({"value": 0}), cli.debug);
            use colored::Colorize;
            ui::info("Power", &format!("{}", "OFF".red()));
        }
```

- [ ] **Step 3: Replace Brightness output (lines 72-80)**

Old:
```rust
        Command::Brightness { value, ip } => {
            if !(1..=100).contains(&value) {
                eprintln!("Brightness must be 1-100");
                process::exit(1);
            }
            let ip = resolve_or_exit(ip.as_deref());
            send_command(&ip, "brightness", serde_json::json!({"value": value}), cli.debug);
            println!("Brightness set to {value}% ({ip})");
        }
```

New:
```rust
        Command::Brightness { value, ip } => {
            if !(1..=100).contains(&value) {
                ui::error("Brightness must be 1-100");
                process::exit(1);
            }
            let ip = resolve_or_exit(ip.as_deref());
            send_command(&ip, "brightness", serde_json::json!({"value": value}), cli.debug);
            ui::info("Brightness", &ui::brightness_bar(value));
        }
```

- [ ] **Step 4: Replace Color output (lines 81-90)**

Old:
```rust
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
```

New:
```rust
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
```

- [ ] **Step 5: Replace Temp output (lines 91-104)**

Old:
```rust
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
```

New:
```rust
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
            use colored::Colorize;
            ui::info("Temp", &format!("{}", format!("{kelvin}K").yellow()));
        }
```

- [ ] **Step 6: Replace Status output (lines 105-142)**

Old:
```rust
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
```

New:
```rust
        Command::Status { ip } => {
            let ip = resolve_or_exit(ip.as_deref());
            let status = send_command(&ip, "devStatus", serde_json::json!({}), cli.debug);
            match status {
                Some(data) => {
                    use colored::Colorize;
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
```

- [ ] **Step 7: Replace Sleep and Reset output (lines 143-166)**

Old Sleep:
```rust
            println!("Sleep mode (dark but responsive) ({ip})");
```

New Sleep:
```rust
            use colored::Colorize;
            ui::info("Sleep", &format!("{}", "dark but responsive".dimmed()));
```

Old Reset:
```rust
            println!("Reset to known good state: on, 100%, 4000K warm white ({ip})");
```

New Reset:
```rust
            use colored::Colorize;
            ui::info("Reset", &format!("{}", "on · 100% · 4000K warm white".dimmed()));
```

- [ ] **Step 8: Build and verify**

Run: `cargo build 2>&1`
Expected: compiles successfully

- [ ] **Step 9: Commit**

```bash
git add src/main.rs
git commit -m "feat(ui): style one-shot command output in main.rs"
```

---

### Task 4: Integrate UI into `src/themes.rs`

**Files:**
- Modify: `src/themes.rs:502-524` (theme_list_display)
- Modify: `src/themes.rs:765-849` (run_theme)
- Modify: `src/cli.rs:8` (after_help reference)

- [ ] **Step 1: Replace `theme_list_display` (lines 502-524)**

Old:
```rust
pub fn theme_list_display() -> String {
    let categories = ["static", "nature", "vibes", "functional", "seasonal"];
    let mut out = String::new();
    for cat in &categories {
        let names: Vec<&str> = THEMES
            .iter()
            .filter(|t| t.category == *cat)
            .map(|t| t.name)
            .collect();
        if !names.is_empty() {
            let label = match *cat {
                "static" => "Static",
                "nature" => "Nature",
                "vibes" => "Vibes",
                "functional" => "Functional",
                "seasonal" => "Seasonal",
                _ => cat,
            };
            out.push_str(&format!("  {label}: {}\n", names.join(", ")));
        }
    }
    out
}
```

New:
```rust
pub fn theme_list_display() -> String {
    let themes: Vec<(&str, &str)> = THEMES.iter().map(|t| (t.name, t.category)).collect();
    crate::ui::theme_list_help(&themes)
}

pub fn print_theme_list() {
    let themes: Vec<(&str, &str)> = THEMES.iter().map(|t| (t.name, t.category)).collect();
    crate::ui::theme_list(&themes);
}
```

- [ ] **Step 2: Replace `run_theme` error output (line 776)**

Old:
```rust
            eprintln!("Unknown theme '{name}'.\n\nAvailable themes:\n{}", theme_list_display());
```

New:
```rust
            crate::ui::error_hint(
                &format!("Unknown theme \"{name}\""),
                "Run govee theme --list to see available themes",
            );
```

- [ ] **Step 3: Replace `run_theme` solid theme output (lines 782-803)**

Old `println!` calls inside `ThemeKind::Solid` match arm:
```rust
            println!("Using device at {ip}");
```
and:
```rust
            println!("Theme '{name}' applied ({ip})");
```

New:
```rust
// Remove the "Using device at" line (resolve_or_exit already shows discovery)
```
and:
```rust
            use colored::Colorize;
            crate::ui::info("Theme", &format!("{} {}", name.white().bold(), format!("[{}]", theme.category).dimmed()));
            crate::ui::info("Brightness", &crate::ui::brightness_bar(brightness));
```

- [ ] **Step 4: Replace `run_theme` animated theme output (lines 804-849)**

Old `println!` at line 782:
```rust
    println!("Using device at {ip}");
```

Remove this line (resolve_or_exit handles it).

Old animated startup (lines 813-816):
```rust
            println!(
                "Theme '{name}' [{}] | Brightness: {brightness}% | {segments} segments | Press Ctrl+C to stop",
                theme.category
            );
```

New:
```rust
            {
                use colored::Colorize;
                crate::ui::info("Theme", &format!("{} {}", name.white().bold(), format!("[{}]", theme.category).dimmed()));
                crate::ui::info("Brightness", &crate::ui::brightness_bar(brightness));
                crate::ui::info("Segments", &format!("{segments}"));
                println!("  {}", "Press Ctrl+C to stop".dimmed());
            }
```

Old shutdown (lines 843-846):
```rust
            println!();
            println!("Deactivating DreamView mode...");
            let _ = razer_deactivate(&ip);
            println!("Stopped.");
```

New:
```rust
            println!();
            crate::ui::deactivating();
            let _ = razer_deactivate(&ip);
            crate::ui::stopped();
```

- [ ] **Step 5: Add live status bar to animated theme loop**

Inside the `while RUNNING` loop (after `send_segments` at line 836), add the status line update:

After:
```rust
                let _ = send_segments(&ip, &send_colors, true);
```

Add:
```rust
                crate::ui::status_line(&send_colors, "");
```

And before the shutdown section, add:
```rust
            crate::ui::status_line_finish();
```

- [ ] **Step 6: Build and verify**

Run: `cargo build 2>&1`
Expected: compiles successfully

- [ ] **Step 7: Commit**

```bash
git add src/themes.rs src/cli.rs
git commit -m "feat(ui): style theme list and theme activation output"
```

---

### Task 5: Integrate UI into `src/screen.rs`

**Files:**
- Modify: `src/screen.rs`

- [ ] **Step 1: Replace startup output (lines 10-57)**

Old error (lines 12-16):
```rust
            eprintln!("Failed to initialize Wayland capture: {e}");
            eprintln!("Make sure your compositor supports wlr-screencopy-unstable-v1");
```

New:
```rust
            crate::ui::error_hint(
                &format!("Failed to initialize Wayland capture: {e}"),
                "Make sure your compositor supports wlr-screencopy-unstable-v1",
            );
```

Old (line 31):
```rust
    println!("Using device at {ip}");
```

Remove (resolve_or_exit handles it). The resolve call at line 23-29 also needs to use `crate::resolve_or_exit`:

Old error (line 37):
```rust
        eprintln!("Failed to set brightness: {e}");
```

New:
```rust
        crate::ui::error(&format!("Failed to set brightness: {e}"));
```

Old error (line 42):
```rust
            eprintln!("Failed to activate DreamView: {e}");
```

New:
```rust
            crate::ui::error(&format!("Failed to activate DreamView: {e}"));
```

Old mode display (lines 45-56):
```rust
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
```

New:
```rust
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
```

- [ ] **Step 2: Add live status bar to capture loop**

Replace the verbose output (lines 129-135):
```rust
            if args.verbose && any_changed {
                let parts: Vec<String> = current_colors
                    .iter()
                    .map(|(r, g, b)| format!("({r:3},{g:3},{b:3})"))
                    .collect();
                println!("  -> {}", parts.join(" | "));
            }
```

New:
```rust
            if any_changed {
                let meta = format!("{}fps · smooth: {}", args.fps, args.smoothing);
                crate::ui::status_line(&current_colors, &meta);
            }
```

- [ ] **Step 3: Replace shutdown output (lines 146-152)**

Old:
```rust
    println!();
    if use_razer {
        println!("Deactivating DreamView mode...");
        let _ = razer_deactivate(&ip);
    }
    println!("Stopped.");
```

New:
```rust
    crate::ui::status_line_finish();
    if use_razer {
        crate::ui::deactivating();
        let _ = razer_deactivate(&ip);
    }
    crate::ui::stopped();
```

- [ ] **Step 4: Build and verify**

Run: `cargo build 2>&1`
Expected: compiles successfully

- [ ] **Step 5: Commit**

```bash
git add src/screen.rs
git commit -m "feat(ui): style screen capture mode output with live status bar"
```

---

### Task 6: Integrate UI into `src/audio_cmd.rs`

**Files:**
- Modify: `src/audio_cmd.rs`

- [ ] **Step 1: Replace startup output**

Old (line 11):
```rust
    println!("Using device at {ip}");
```

Remove (resolve_or_exit handles it).

Old error (line 17):
```rust
        eprintln!("Failed to set brightness: {e}");
```

New:
```rust
        crate::ui::error(&format!("Failed to set brightness: {e}"));
```

Old error (line 22):
```rust
            eprintln!("Failed to activate DreamView: {e}");
```

New:
```rust
            crate::ui::error(&format!("Failed to activate DreamView: {e}"));
```

Old error (lines 30-31):
```rust
            eprintln!("Failed to start audio capture: {e}");
            eprintln!("Make sure PulseAudio is running and audio is playing");
```

New:
```rust
            crate::ui::error_hint(
                &format!("Failed to start audio capture: {e}"),
                "Make sure PulseAudio is running and audio is playing",
            );
```

Old mode display (lines 47-51):
```rust
    println!(
        "Mode: {:?} | Palette: {:?} | {} | Sensitivity: {} | Brightness: {}%",
        args.mode, args.palette, mode_str, args.sensitivity, args.brightness
    );
    println!("Press Ctrl+C to stop");
```

New:
```rust
    {
        use colored::Colorize;
        crate::ui::info("Mode", &format!("{} {}", format!("{:?}", args.mode).white(), format!("{} · {:?} · sens: {}", mode_str, args.palette, args.sensitivity).dimmed()));
        crate::ui::info("Brightness", &crate::ui::brightness_bar(args.brightness));
        println!("  {}", "Press Ctrl+C to stop".dimmed());
    }
```

- [ ] **Step 2: Replace verbose output with live status bar**

Old (lines 105-118):
```rust
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
```

New:
```rust
        {
            let meta = format!(
                "E:{:.1} beat:{} · {:?}",
                audio.energy, audio.beat, args.palette
            );
            crate::ui::status_line(&current_colors, &meta);
        }
```

- [ ] **Step 3: Replace shutdown output (lines 127-134)**

Old:
```rust
    println!();
    drop(analyzer);
    if use_razer {
        println!("Deactivating DreamView mode...");
        let _ = razer_deactivate(&ip);
    }
    println!("Stopped.");
```

New:
```rust
    crate::ui::status_line_finish();
    drop(analyzer);
    if use_razer {
        crate::ui::deactivating();
        let _ = razer_deactivate(&ip);
    }
    crate::ui::stopped();
```

- [ ] **Step 4: Build and verify**

Run: `cargo build 2>&1`
Expected: compiles successfully

- [ ] **Step 5: Commit**

```bash
git add src/audio_cmd.rs
git commit -m "feat(ui): style audio reactive mode output with live status bar"
```

---

### Task 7: Integrate UI into `src/ambient.rs`

**Files:**
- Modify: `src/ambient.rs`

- [ ] **Step 1: Replace all output**

Old error (lines 20-26):
```rust
        eprintln!(
            "Invalid color '{}'. Available: {}",
            args.color,
            valid_colors.join(", ")
        );
        process::exit(1);
```

New:
```rust
        crate::ui::error_hint(
            &format!("Invalid color '{}'", args.color),
            &format!("Available: {}", valid_colors.join(", ")),
        );
        process::exit(1);
```

Old resolve error (lines 31-34):
```rust
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
```

New:
```rust
        Err(_) => {
            crate::ui::error_hint("No device found", "Is the strip powered on and connected to WiFi?");
            process::exit(1);
        }
```

But also add discovery scanning before the resolve call (insert before line 28):
```rust
    if args.ip.is_none() {
        crate::ui::discovery_scanning();
    }
```

And add discovery_found after the `Ok(ip)` (line 29-30, replace):
```rust
        Ok(ip) => {
            if args.ip.is_none() {
                crate::ui::discovery_found("device", &ip);
            }
            ip
        }
```

Old (line 35):
```rust
    println!("Using device at {ip}");
```

Remove.

Old (line 47):
```rust
        eprintln!("Failed to set brightness: {e}");
```

New:
```rust
        crate::ui::error(&format!("Failed to set brightness: {e}"));
```

Old scheme error (lines 51-53):
```rust
            eprintln!("{e}");
            process::exit(1);
```

New:
```rust
            crate::ui::error(&format!("{e}"));
            process::exit(1);
```

Old initial color verbose (lines 63-64):
```rust
        if args.verbose {
            println!("Initial color: ({r}, {g}, {b}) from {color_key}");
        }
```

New:
```rust
        if args.verbose {
            crate::ui::info("Color", &crate::ui::color_swatch_full(r, g, b));
        }
```

Old watch status (lines 68-72):
```rust
    println!(
        "Watching {} for theme changes (Ctrl+C to stop)",
        path.display()
    );
    println!("Color key: {color_key} | Brightness: {}%", args.brightness);
```

New:
```rust
    {
        use colored::Colorize;
        crate::ui::info("Watching", &format!("{}", path.display()));
        crate::ui::info("Color key", &color_key);
        crate::ui::info("Brightness", &crate::ui::brightness_bar(args.brightness));
        println!("  {}", "Press Ctrl+C to stop".dimmed());
    }
```

Old send error (line 114):
```rust
                            eprintln!("Failed to send color: {e}");
```

New:
```rust
                            crate::ui::error(&format!("Failed to send color: {e}"));
```

Old verbose update (lines 118-120):
```rust
                        if args.verbose {
                            println!("Updated: ({r}, {g}, {b})");
                        }
```

New:
```rust
                        if args.verbose {
                            crate::ui::info("Updated", &crate::ui::color_swatch(r, g, b));
                        }
```

Old inotify error (line 124):
```rust
                eprintln!("inotify error: {e}");
```

New:
```rust
                crate::ui::error(&format!("inotify error: {e}"));
```

Old initial send error (line 60):
```rust
            eprintln!("Failed to send color: {e}");
```

New:
```rust
            crate::ui::error(&format!("Failed to send color: {e}"));
```

Old stopped (line 129):
```rust
    println!("\nStopped.");
```

New:
```rust
    println!();
    crate::ui::stopped();
```

- [ ] **Step 2: Build and verify**

Run: `cargo build 2>&1`
Expected: compiles successfully

- [ ] **Step 3: Commit**

```bash
git add src/ambient.rs
git commit -m "feat(ui): style ambient mode output"
```

---

### Task 8: Final build, clippy, and manual test

**Files:** None (verification only)

- [ ] **Step 1: Run clippy**

Run: `cargo clippy 2>&1`
Expected: no errors, no new warnings from our changes

- [ ] **Step 2: Run tests**

Run: `cargo test 2>&1`
Expected: all tests pass

- [ ] **Step 3: Test basic command output visually**

Run: `cargo run -- --help 2>&1 | head -30`
Expected: should show the themed help text with colored theme list

- [ ] **Step 4: Fix any clippy warnings or build issues**

Address any warnings from steps 1-2.

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "fix: address clippy warnings from UI integration"
```

(Skip this step if no fixes needed.)
