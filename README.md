# govee-lan

Control Govee LED strip lights over your local network using the LAN API. No cloud, no API keys, just UDP.

## Features

- **Device discovery** via multicast scan
- **On/off, brightness, RGB color, color temperature** control
- **Preset scenes** — static (movie, chill, party, sunset, ocean, forest, candlelight, aurora) and animated (fireplace, storm, lava, breathing, sunrise)
- **Ambient mode** — syncs strip color to your desktop wallpaper theme via [Caelestia](https://github.com/caelestia-dots/)
- **Screen mode** — real-time ambilight from Wayland screen capture (wlr-screencopy)
- **Audio mode** — audio-reactive visualization with energy, frequency, beat, and drop modes
- **DreamView** — multi-segment color control with optional gradient interpolation and mirror layout

## Requirements

- Rust toolchain (cargo)
- A Govee LED strip with **LAN API enabled** (Govee Home app > Device > Settings > LAN Control)
- **Linux** with:
  - PulseAudio/PipeWire (for audio mode) — `libpulse-dev`
  - Wayland compositor with `wlr-screencopy-unstable-v1` (for screen mode)
  - [Caelestia](https://github.com/caelestia-dots/) or compatible scheme.json provider (for ambient mode)

## Install

```bash
git clone https://github.com/ir1141/govee.git
cd govee
cargo build --release
# Binary at target/release/govee
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

# Apply a preset scene
govee scene sunset

# Animated scenes run until Ctrl+C
govee scene fireplace

# Sleep mode (dark but stays responsive)
govee sleep
```

All commands auto-discover the device. Use `--ip` to target a specific one:

```bash
govee on --ip 192.168.1.42
```

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
govee screen --no-razer
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
```

**Visualization modes:** `energy`, `frequency`, `beat`, `drop`
**Palettes:** `fire`, `ocean`, `neon`, `rainbow`

## How it works

Govee strips with LAN API enabled listen for UDP commands on your local network:

- **Multicast** (`239.255.255.250:4001`) for device discovery
- **Unicast** (port `4003`) for control commands
- **DreamView/Razer protocol** for per-segment color control (base64-encoded binary over UDP)

No authentication, no cloud dependency, no rate limits.

## License

MIT
