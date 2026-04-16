# govee

Control Govee LED strip lights over your local network using the LAN API. No cloud, no API keys, just UDP. Includes a CLI and a GUI.

## Features

- **Device discovery** via multicast scan
- **On/off, brightness, RGB color, color temperature** control
- **[Themes](#themes)** — 30 builtin animated and static themes, plus [custom TOML themes](#custom-themes)
- **[Ambient mode](#ambient-wallpaper-sync)** — syncs strip color to your wallpaper theme via [Caelestia](https://github.com/caelestia-dots/)
- **[Screen mode](#screen-capture-ambilight)** — real-time ambilight from Wayland screen capture
- **[Audio mode](#audio-reactive)** — audio-reactive visualization with energy, frequency, beat, and drop modes
- **[Sunlight mode](#sunlight-daynight-cycle)** — crossfades between day and night themes based on solar position
- **DreamView** — multi-segment color control with gradient interpolation
- **Mirror mode** — doubles segments in reverse for U-shaped strip layouts (`--mirror`)
- **[GUI](#gui)** — graphical interface built with [iced](https://iced.rs) for all of the above

## Requirements

- [Rust toolchain](https://rustup.rs/) (cargo)
- A Govee LED strip with **LAN API enabled** (Govee Home app > Device > Settings > LAN Control)
- **Linux** with:
  - PulseAudio/PipeWire (for audio mode) — `libpulse-dev`
  - Wayland compositor with `wlr-screencopy-unstable-v1` (for screen mode)
  - [Caelestia](https://github.com/caelestia-dots/) or compatible scheme.json provider (for ambient mode) — generates wallpaper color schemes that ambient mode watches

## Install

```bash
git clone https://github.com/ir1141/govee.git
cd govee
cargo build --release
# Binaries at target/release/govee and target/release/govee-gui
```

## Usage

### Basic control

```bash
# Discover devices on your network
govee scan

# Turn on/off
govee on
govee off

# Set brightness (1-100)
govee brightness 60

# Set RGB color
govee color 255 100 0

# Set color temperature (2000-9000K)
govee temp 4000

# Query device status
govee status

# Sleep mode (dark but stays responsive)
govee sleep

# Reset to known good state (DreamView off, on, full brightness, warm white)
govee reset
```

All commands auto-discover the device. Use `--ip` to target a specific one:

```bash
govee on --ip 192.168.1.42
```

#### Global flags

| Flag | Description |
|------|-------------|
| `--mirror` | Double segments by appending a reversed copy (for U-shaped strip layouts) |
| `--quiet` / `-q` | Suppress informational output (errors still print) |
| `--debug` | Show raw UDP messages sent to the device |

### Screen capture (ambilight)

Captures your screen and maps colors to the LED strip in real time:

```bash
# Default: 5 segments, 10fps, DreamView mode
govee screen

# More segments, faster, with gradient blending
govee screen --segments 10 --fps 30 --gradient

# Boost color saturation
govee screen --saturate 1.5

# Mirror for U-shaped strip layout
govee --mirror screen

# Pick a specific monitor
govee screen --output DP-1

# Simple single-color mode (no DreamView)
govee screen --no-dreamview

# Verbose — show available outputs and per-frame details
govee screen --verbose
```

### Ambient wallpaper sync

Syncs strip color to your Caelestia wallpaper theme in real time:

```bash
# Primary accent color at 80% brightness (default)
govee ambient

# Tertiary color, dimmed variant, at 50%
govee ambient --color tertiary --dim --brightness 50
```

### Audio reactive

Visualize system audio on your LED strip:

```bash
# Energy mode with fire palette (default)
govee audio

# Frequency spectrum visualization
govee audio --mode frequency --palette rainbow

# Beat-flash mode
govee audio --mode beat --palette neon

# Drop mode — dark until bass/treble hits
govee audio --mode drop

# Tune sensitivity and segment count
govee audio --segments 10 --sensitivity 1.5 --gradient

# Verbose — show live energy/beat status per frame
govee audio --verbose
```

**Visualization modes:** `energy`, `frequency`, `beat`, `drop`
**Palettes:** `fire`, `ocean`, `forest`, `neon`, `ice`, `sunset`, `rainbow`

### Sunlight (day/night cycle)

Tracks the sun and crossfades the strip between a day behavior and a night behavior across sunrise and sunset windows. Each preset pairs two animated themes; `simple` instead crossfades between two color temperatures.

```bash
# Default: coastal preset (wave by day, fireplace by night)
govee sunlight

# Pick a preset
govee sunlight --preset arctic
govee sunlight --preset ember
govee sunlight --preset simple --day-temp 6500 --night-temp 2700

# Use solar calculation for your location
govee sunlight --lat 47.6 --lon -122.3

# Or override sunrise/sunset manually
govee sunlight --sunrise 06:30 --sunset 20:15

# Tune the transition window and brightness
govee sunlight --transition 60 --brightness 80 --night-brightness 30
```

**Presets:** `coastal`, `arctic`, `ember`, `simple`

| Flag | Default | Description |
|------|---------|-------------|
| `--preset` | `coastal` | Day/night behavior pair |
| `--lat` / `--lon` | — | Location for solar calculation |
| `--sunrise` / `--sunset` | — | Manual `HH:MM` override |
| `--transition` | `45` | Crossfade duration (minutes) |
| `--brightness` | `80` | Day brightness (1–100) |
| `--night-brightness` | — | Night brightness (omit to keep constant) |
| `--segments` | `15` | DreamView segments |
| `--day-temp` / `--night-temp` | `6500` / `3000` | Color temps (`simple` preset only) |

### Themes

30 builtin themes across 5 categories:

| Category | Themes |
|----------|--------|
| **Static** | movie, chill, party, sunset, forest |
| **Nature** | candlelight, fireplace, campfire, lava, ocean, aurora, northern-lights, rain |
| **Vibes** | breathing, romantic, cozy, cyberpunk, vaporwave, nightclub |
| **Functional** | storm, lightning, thunderstorm, starfield, pulse, rainbow, gradient-wave, sunrise |
| **Seasonal** | christmas, halloween, snowfall |

**Static** themes set a single color and exit. **Animated** themes use DreamView for per-segment color control and loop until Ctrl+C.

```bash
# Static — sets color and exits
govee theme sunset

# Animated — loops with DreamView segments
govee theme fireplace

# More segments, custom brightness
govee theme aurora --segments 10 --brightness 40

# Mirror for U-shaped strip layout
govee --mirror theme cyberpunk --segments 8
```

#### Behavior types

Animated themes are built from 13 behavior types that control how colors move across the strip:

| Behavior | Description |
|----------|-------------|
| **heat** | Flickering fire simulation with sparks and dim spots |
| **wave** | Layered sine waves with configurable speed and frequency |
| **breathe** | Smooth pulsing through a color palette |
| **flash** | Random flashes over a slow-moving base wave |
| **particles** | Colored dots drifting across a background |
| **twinkle** | Random pixels lighting up and fading out |
| **hue-rotate** | Continuous HSV hue rotation across all segments |
| **gradient-wave** | Two-color gradient oscillating back and forth |
| **strobe** | Rapid color cycling with random flashes |
| **alternating** | Shifting color blocks with sparkle accents |
| **drift** | Palette scrolling smoothly across the strip |
| **radiate-pulse** | Single-color pulse radiating outward from center |
| **progression** | Slow palette evolution over a set duration |

#### Custom themes

Create TOML files in `~/.config/govee/themes/` to add your own or override builtins. Each file defines one theme with a `name`, `category`, and `kind` (either `solid` or `animated`).

##### Static theme

Sets a single RGB color and exits:

```toml
# ~/.config/govee/themes/deep-purple.toml
name = "deep-purple"
category = "vibes"

[kind]
type = "solid"
color = [80, 0, 160]
```

##### Animated theme

Loops a behavior across DreamView segments until Ctrl+C:

```toml
# ~/.config/govee/themes/souls-bonfire.toml
name = "souls-bonfire"
category = "gaming"

[kind]
type = "animated"

[kind.delay]
random = [140, 300]

[kind.behavior]
type = "heat"
volatility = 0.12
spark_chance = 0.04
spark_boost = 0.8
dim_chance = 0.4
dim_range = [0.05, 0.3]
diffusion = 0.05

[[kind.behavior.palette]]
pos = 0.0
r = 30
g = 5
b = 0

[[kind.behavior.palette]]
pos = 0.3
r = 80
g = 20
b = 0

[[kind.behavior.palette]]
pos = 0.6
r = 140
g = 50
b = 5

[[kind.behavior.palette]]
pos = 0.85
r = 200
g = 90
b = 10

[[kind.behavior.palette]]
pos = 1.0
r = 255
g = 140
b = 20
```

##### Theme file structure

Every animated theme has a **delay** and a **behavior**:

**Delay** controls the tick rate between frames:

| Delay | TOML | Description |
|-------|------|-------------|
| Fixed | `fixed = 80` | Constant interval in ms |
| Random | `random = [80, 200]` | Random interval between min/max ms |

**Palette anchors** define color stops that get interpolated. Each anchor has a `pos` (0.0–1.0) and `r`, `g`, `b` (0–255):

```toml
[[kind.behavior.palette]]
pos = 0.0      # start of gradient
r = 255
g = 100
b = 0
```

**Behavior parameters** vary by type. Here are the most useful ones:

| Behavior | Key parameters |
|----------|---------------|
| **heat** | `palette`, `volatility` (flicker amount), `spark_chance`, `spark_boost`, `dim_chance`, `dim_range = [min, max]`, `diffusion` |
| **wave** | `palette`, `waves` (array of `{time_speed, spatial_freq, phase_offset}`), `weights` |
| **breathe** | `palette`, `speed` (pulse rate), `power` (curve sharpness, higher = sharper) |
| **flash** | `base_palette`, `flash_palette`, `decay`, `flash_chance`, `spread = [min, max]`, `base_wave_speed`, `base_spatial_freq`, `flash_threshold` |
| **particles** | `bg = [r, g, b]`, `palette`, `speed`, `spawn_chance`, `bright_chance` |
| **twinkle** | `bg = [r, g, b]`, `colors` (array of `[r, g, b]`), `on_chance`, `fade_speed` |
| **hue-rotate** | `speed`, `saturation` (0.0–1.0), `value` (0.0–1.0) |
| **gradient-wave** | `color_a = [r, g, b]`, `color_b = [r, g, b]`, `speed` |
| **strobe** | `colors` (array of `[r, g, b]`), `cycle_speed`, `flash_chance` |
| **alternating** | `colors` (array of `[r, g, b]`), `sparkle = [r, g, b]`, `sparkle_chance`, `shift_speed` |
| **drift** | `palette`, `speed` |
| **radiate-pulse** | `color = [r, g, b]`, `speed`, `width` |
| **progression** | `palette`, `duration_secs`, `spatial_spread` |

##### Tips

- To override a builtin, use the same name — your version takes priority
- Category can be anything; custom categories show up in `govee theme --help`
- Look at the builtin themes in `govee-themes/src/theme_defs.rs` for working examples of every behavior type
- Start with **breathe**, **drift**, or **gradient-wave** — they need the fewest parameters

## GUI

`govee-gui` is a graphical frontend built with [iced](https://iced.rs). It provides the same functionality as the CLI through a sidebar-navigated interface:

- **Controls** — power, brightness, color picker, color temperature
- **Themes** — browse and apply builtin/custom themes with category filters and palette previews
- **Screen** — configure and launch ambilight mode (FPS, segments, brightness)
- **Audio** — configure and launch audio-reactive mode (visualization mode, segments, brightness)
- **Ambient** — configure and launch wallpaper sync (brightness, dim toggle)
- **Sunlight** — configure and launch day/night cycle (preset, location, transition, brightness)

The GUI spawns the `govee` CLI as a subprocess for continuous modes (screen, audio, ambient, animated themes) and sends direct UDP commands for one-shot actions. Device discovery runs automatically on launch with periodic refresh. A global mirror toggle is available in the status bar.

Settings (last device IP, page, per-mode options) persist in `~/.config/govee/gui.toml`.

```bash
# Launch the GUI
govee-gui
```

## How it works

Govee strips with LAN API enabled listen for UDP commands on your local network:

- **Multicast** (`239.255.255.250:4001`) for device discovery
- **Unicast** (port `4003`) for control commands
- **DreamView/Razer protocol** for per-segment color control (base64-encoded binary over UDP)

## Troubleshooting

- **Device not found**: Make sure LAN Control is enabled in Govee Home app (Device > Settings > LAN Control). The device must be on the same subnet.
- **Screen mode fails**: Your Wayland compositor must support `wlr-screencopy-unstable-v1`. KDE and GNOME do not; wlroots-based compositors (Sway, Hyprland) do.
- **Audio mode fails**: Ensure PulseAudio or PipeWire is running. Check with `pactl info`. You need a monitor source (system audio output).

## Contributing

PRs are welcome.

## License

MIT
