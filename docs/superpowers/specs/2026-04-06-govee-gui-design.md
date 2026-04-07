# Govee GUI — Design Spec

## Overview

A standalone GUI application (`govee-gui`) for controlling Govee LED strip lights over LAN. Built with Iced (Rust) in an Elm-architecture pattern. Lives alongside the existing CLI tool in a Cargo workspace, sharing the `govee-lan` library crate for direct device communication.

**Target platform:** Arch Linux, Wayland  
**Visual style:** Sleek minimal dark (Spotify/Discord aesthetic), purple accent (#7c3aed)  
**Framework:** Iced (pure Rust, Wayland-native via wgpu)

## Scope

### In scope
- Power, brightness, color, color temperature controls
- Theme browser with category filtering and palette preview cards
- Screen capture, audio reactive, and ambient mode launchers with settings
- Device discovery and selection (2-3 devices)
- Settings persistence (~/.config/govee/gui.toml)

### Out of scope
- System tray / compact mode
- In-process continuous mode loops (v2 — subprocess in v1)
- Custom theme editor (users edit TOML files directly)
- Multi-device group control

## Workspace Structure

```
govee/
├── Cargo.toml              # workspace root
├── govee-lan/              # existing library (moved from current root)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── protocol.rs
│       ├── discovery.rs
│       ├── colors.rs
│       ├── wayland.rs
│       └── audio.rs
├── govee-cli/              # existing binary (moved from current root)
│   ├── Cargo.toml          # depends on govee-lan
│   └── src/
│       ├── main.rs
│       ├── cli.rs
│       ├── themes.rs       # runtime loop logic (run_theme, ctrlc, RUNNING)
│       ├── theme_defs.rs
│       ├── theme_loader.rs
│       ├── ui.rs
│       ├── ambient.rs
│       ├── screen.rs
│       └── audio_cmd.rs
├── govee-gui/              # new GUI binary
│   ├── Cargo.toml          # depends on govee-lan, iced
│   └── src/
│       ├── main.rs         # app entry point
│       ├── app.rs          # top-level state, Message enum, routing
│       ├── style.rs        # dark theme, color constants, spacing
│       ├── pages/
│       │   ├── controls.rs
│       │   ├── themes.rs
│       │   ├── screen.rs
│       │   ├── audio.rs
│       │   └── ambient.rs
│       ├── widgets/
│       │   ├── sidebar.rs
│       │   ├── status_bar.rs
│       │   ├── color_picker.rs
│       │   └── slider.rs
│       ├── subprocess.rs   # spawn/kill/monitor CLI subprocesses (v1)
│       └── config.rs       # GUI settings persistence
```

### Crate responsibilities

**govee-lan** (library) — Pure device communication. UDP protocol, discovery, color utilities. No CLI deps, no ctrlc, no AtomicBool. Both CLI and GUI depend on this.

**govee-cli** (binary) — CLI interface. Owns the continuous mode loops (screen, audio, ambient, animated themes) with ctrlc/RUNNING signal handling. Depends on govee-lan.

**govee-gui** (binary) — GUI interface. Uses govee-lan directly for simple commands. Shells out to `govee` CLI binary for continuous modes in v1. Depends on govee-lan + iced.

### Theme data migration

Theme definitions (`ThemeDef`, `ThemeKind`, `Behavior`, `Delay`, `PA`, `WaveParam`) and the builtin list (`builtin_themes()`) plus TOML loader move to `govee-lan` so the GUI can read theme names, palettes, and categories for the card grid. The runtime loop (`run_theme`) stays in `govee-cli` since it depends on ctrlc/RUNNING.

## App Layout

### Window
- Standard window, normal taskbar entry, alt-tab behavior
- Default size ~900x600, resizable

### Structure
```
┌──────────┬──────────────────────────────────┐
│ Sidebar  │ Content Area                     │
│          │                                  │
│ GOVEE    │ (page-specific content)          │
│ H6159    │                                  │
│ 192.168… │                                  │
│          │                                  │
│ Controls │                                  │
│ Themes   │                                  │
│ Screen   │                                  │
│ Audio    │                                  │
│ Ambient  │                                  │
│          │                                  │
│ [Device] │                                  │
├──────────┴──────────────────────────────────┤
│ Status Bar: ● Connected | Mode: Static      │
└─────────────────────────────────────────────┘
```

**Sidebar (200px fixed):**
- App title + device model/IP at top
- Navigation links: Controls, Themes, Screen, Audio, Ambient
- Active page highlighted with accent background
- Device selector dropdown at bottom

**Status bar:**
- Connection state (Connected/Disconnected/Discovering)
- Current mode (Static Color, Theme — name, Screen Capture, Audio Reactive, Ambient)

## Pages

### Controls (default page)

- **Power toggle** — top right, prominent on/off switch
- **Brightness slider** — horizontal, 1-100, shows percentage value
- **Color picker** — current color swatch (large) + grid of preset color swatches. Click preset to apply, or click the large swatch to open a full color picker
- **Color temperature slider** — horizontal, 2000K-9000K, gradient track from warm to cool white

All controls fire immediately via `govee-lan` protocol functions (UDP send, fire-and-forget).

### Themes

- **Category filter tabs** — All, Static, Nature, Vibes, Functional, Seasonal, User
- **Card grid** — responsive grid (auto-fill, min 140px per card)
  - Each card shows: color band (solid for static, gradient for animated), theme name, "Animated" badge if applicable
  - Active theme: purple border (#7c3aed) + green "Active" indicator
  - Click to apply: static themes send `colorwc` directly, animated themes spawn `govee theme <name>` subprocess
- Theme data loaded from `govee-lan` (builtins + user TOML themes)

### Screen

- **Start/Stop button** — large, obvious toggle
- **Status** — Running (with elapsed time) or Stopped
- **Settings:**
  - FPS slider (1-30, default 10)
  - Brightness slider (1-100, default 80)
  - Smoothing slider (0.0-1.0, default 0.3)
  - Threshold slider (0-50, default 10)
  - Segments count (1-127, default 5)
  - Saturation slider (0.5-2.0, default 1.0)
  - Monitor/output dropdown (populated from system)
  - Toggles: Gradient, Mirror, No-DreamView
- Start spawns: `govee screen --fps <n> --brightness <n> ...`
- Stop kills the subprocess

### Audio

- **Start/Stop button + status** (same pattern as Screen)
- **Settings:**
  - Visualization mode picker: Energy, Frequency, Beat, Drop (radio buttons or segmented control)
  - Palette picker: card-style grid similar to themes, showing palette gradient
  - Brightness slider (1-100, default 80)
  - Smoothing slider (0.0-1.0, default 0.3)
  - Sensitivity slider (0.1-3.0, default 1.0)
  - Segments count (1-127, default 5)
  - Toggles: Gradient, Mirror, No-DreamView
- Start spawns: `govee audio --mode <m> --palette <p> ...`

### Ambient

- **Start/Stop button + status**
- **Settings:**
  - Color source picker: primary, secondary, tertiary, etc. (from Caelestia scheme.json)
  - Brightness slider (1-100, default 80)
  - Dim variant toggle
- Start spawns: `govee ambient --color <c> --brightness <n> ...`

## Iced Application Structure

### State
```rust
struct App {
    page: Page,                        // Controls, Themes, Screen, Audio, Ambient
    device: Option<DeviceInfo>,        // selected device
    devices: Vec<DeviceInfo>,          // discovered devices
    power: bool,
    brightness: u8,
    color: (u8, u8, u8),
    color_temp: u16,
    themes: Vec<ThemeDef>,            // from govee-lan
    active_theme: Option<String>,
    subprocess: Option<Child>,         // running continuous mode
    subprocess_mode: Option<Mode>,     // what's running
    subprocess_start: Option<Instant>, // for elapsed time
    config: GuiConfig,                 // persisted settings
}
```

### Messages
```rust
enum Message {
    Navigate(Page),
    SelectDevice(usize),
    TogglePower,
    SetBrightness(u8),
    SetColor(u8, u8, u8),
    SetColorTemp(u16),
    ApplyTheme(String),
    StartMode { kind: Mode, args: Vec<String> },
    StopMode,
    SubprocessExited(Result<ExitStatus, String>),
    DevicesDiscovered(Vec<DeviceInfo>),
    Tick(Instant),  // for elapsed time display
    ConfigSaved,
}
```

### Threading model
- **Main thread** — Iced UI event loop
- **Discovery** — `iced::Subscription` running `scan_devices` on startup + periodic refresh
- **Direct commands** (on/off/color/brightness) — `iced::Task` wrapping non-blocking UDP sends
- **Subprocess** — `std::process::Command` spawn/kill, monitored via subscription polling child status
- **Tick** — `iced::time::every(Duration::from_secs(1))` subscription for elapsed time display when a mode is running

### Styling
- Custom Iced theme struct implementing the palette:
  - Background: #1a1a2e
  - Sidebar: #12122a  
  - Surface: #2a2a4a (borders, dividers)
  - Accent: #7c3aed (active states, highlights)
  - Accent light: #a78bfa (hover states)
  - Text primary: #e0e0ff
  - Text secondary: #8888aa
  - Text muted: #6a6a9a
  - Success: #44dd88 (connected, active indicators)
- Constants in `style.rs` for spacing (8px grid), border radius (6-8px), sidebar width (200px)

## Config Persistence

File: `~/.config/govee/gui.toml`

```toml
[general]
last_device_ip = "192.168.1.42"
last_page = "controls"

[controls]
brightness = 80
color = [124, 58, 237]
color_temp = 4000

[screen]
fps = 10
brightness = 80
smoothing = 0.3
threshold = 10
segments = 5
saturation = 1.0
gradient = false
mirror = false
no_dreamview = false

[audio]
mode = "energy"
palette = "fire"
brightness = 80
smoothing = 0.3
sensitivity = 1.0
segments = 5
gradient = false
mirror = false
no_dreamview = false

[ambient]
color = "primary"
brightness = 80
dim = false
```

Saved on every settings change (debounced). Loaded on startup.

## v1 → v2 Migration Path

v1 launches continuous modes as CLI subprocesses. Code should be structured for easy migration:

- `subprocess.rs` module handles all spawn/kill/monitor logic
- Each mode page builds a `ModeConfig` struct, which `subprocess.rs` converts to CLI args
- v2 replaces `subprocess.rs` internals with in-process loops from `govee-lan`, using the same `ModeConfig` interface
- Mark all subprocess callsites with `// TODO: v2 — move to in-process for live previews`

## Dependencies (govee-gui)

```toml
[dependencies]
govee-lan = { path = "../govee-lan" }
iced = { version = "0.13", features = ["wgpu", "tokio"] }
tokio = { version = "1", features = ["process", "time"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dirs = "6"
```
