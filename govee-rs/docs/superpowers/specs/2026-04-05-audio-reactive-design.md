# Audio Reactive Mode — Design Spec

## Overview

Add an `audio` subcommand to the Govee CLI that reacts to system audio output (via PulseAudio monitor source) and drives the LED strip in real time. Supports three visualization modes (energy, frequency, beat) and four color palettes (fire, ocean, neon, rainbow). Uses DreamView multi-segment mode by default with single-color fallback.

## CLI Interface

```
govee audio [OPTIONS]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--ip` | `Option<String>` | auto-discover | Device IP |
| `--mode` | `enum` | `energy` | `energy`, `frequency`, `beat` |
| `--palette` | `enum` | `fire` | `fire`, `ocean`, `neon`, `rainbow` |
| `--brightness` | `u8` | `80` | Strip brightness 1-100 |
| `--segments` | `usize` | `5` | Number of DreamView segments |
| `--smoothing` | `f64` | `0.3` | Color transition speed 0.0-1.0 |
| `--sensitivity` | `f64` | `1.0` | Audio gain multiplier |
| `--no-razer` | `bool` | `false` | Single-color mode instead of DreamView |
| `--gradient` | `bool` | `false` | Interpolate between segment colors |
| `--mirror` | (global) | `false` | Mirror segments for U-shaped strip |
| `--verbose` | `bool` | `false` | Print debug info |

## Audio Capture (`src/audio.rs`)

### AudioCapture

Runs PulseAudio monitor source capture in a background thread.

- Connects to PulseAudio, finds the default sink's monitor source (e.g. `alsa_output.pci-0000_00_1f.3.analog-stereo.monitor`)
- Opens a recording stream on that monitor source: 44100 Hz, mono, f32le
- Continuously reads PCM samples into a ring buffer (2048 samples = ~46ms window at 44.1kHz)
- On each full buffer, runs `rustfft` (1024-point FFT with Hanning window) and stores the result

### AudioAnalyzer

Wraps the capture thread and exposes analysis results behind an `Arc<Mutex<AudioState>>`:

```rust
pub struct AudioState {
    pub energy: f64,       // 0.0-1.0 normalized RMS energy
    pub bands: [f64; 6],   // bass, low-mid, mid, upper-mid, presence, brilliance (0.0-1.0 each)
    pub beat: bool,        // true on detected beat onset
    pub peak: f64,         // recent peak for auto-gain
}
```

### Beat Detection

Energy-based: compare current energy to a short rolling average. If current exceeds average by a threshold, it's a beat. Cooldown timer prevents double-triggers.

### Frequency Band Split

Divide FFT bins into 6 frequency ranges:

| Band | Range |
|------|-------|
| Bass | 20-150 Hz |
| Low-mid | 150-400 Hz |
| Mid | 400-1000 Hz |
| Upper-mid | 1-2.5 kHz |
| Presence | 2.5-6 kHz |
| Brilliance | 6-20 kHz |

Each band's magnitude is normalized against its own rolling peak for auto-leveling.

## Visualization Modes

### Palettes

Each palette is an array of 4-6 anchor colors. A `palette_color(palette, intensity: f64) -> (u8, u8, u8)` function interpolates between anchors.

| Palette | Anchor colors |
|---------|--------------|
| `fire` | black -> dark red -> orange -> yellow -> white |
| `ocean` | black -> deep blue -> teal -> cyan -> white |
| `neon` | dark purple -> magenta -> hot pink -> electric blue -> cyan |
| `rainbow` | red -> orange -> yellow -> green -> blue -> violet (wrapping) |

### Energy Mode

- All segments get the same color, driven by RMS energy mapped through the palette
- In DreamView mode, slight per-segment variation using a time-offset sine wave so the strip "breathes" rather than being flat

### Frequency Mode

- 6 bands mapped across N segments (segments distributed proportionally — more segments for bass since that's visually dominant)
- Each segment's intensity comes from its corresponding band
- Each band's intensity mapped through the palette independently

### Beat Mode

- On beat: all segments flash to the bright end of the palette, then exponential decay back to dim
- Color hue rotates on each beat (advance position along the palette by a fixed step)
- Between beats: segments show a dim base color with gentle pulse from residual energy

### Single-Color Mode (`--no-razer`)

All modes collapse to a single averaged color sent via `send_color()`.

## Main Loop (`run_audio()`)

Follows the same structure as `run_screen()`:

1. Resolve device IP, set brightness
2. Activate DreamView if using razer mode
3. Create `AudioAnalyzer::new()` — starts background capture thread
4. Install Ctrl+C handler (reuse existing `ctrlc_setup()` / `RUNNING` atomic)
5. Main loop at ~60 ticks/sec:
   - Read latest `AudioState` from the analyzer (lock, clone, unlock)
   - Apply `--sensitivity` multiplier to energy/bands
   - Map to colors based on `--mode` and `--palette`
   - Apply `--smoothing` (reuse existing `smooth()` function)
   - Apply `--mirror` if set
   - Send via `send_segments()` or `send_color()` depending on `--no-razer`
   - Keepalive every 2 seconds (same as screen mode)
   - Verbose logging of current energy/bands/colors
6. On exit: deactivate DreamView, drop analyzer (joins capture thread)

## Dependencies

New crates in `Cargo.toml`:

- `libpulse-binding = "2"` — PulseAudio client API
- `rustfft = "6"` — FFT computation

## File Changes

| File | Change |
|------|--------|
| `src/audio.rs` | New — `AudioCapture`, `AudioAnalyzer`, `AudioState`, palette/visualization logic |
| `src/main.rs` | Add `Audio` subcommand, `AudioArgs`, `run_audio()` |
| `src/lib.rs` | Add `pub mod audio;` |
| `Cargo.toml` | Add `libpulse-binding`, `rustfft` |
