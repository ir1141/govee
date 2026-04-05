# Govee CLI — Terminal UI Redesign

## Overview

Add a cohesive visual identity to the govee CLI using ANSI colors and Unicode characters. The "Branded Accent" style uses purple `◆` diamonds as the signature motif, colored accent lines, segment color blocks, and brightness bars. Every command gets styled output while keeping the tool feeling like a proper CLI, not a TUI.

## Dependency

Add the **`colored`** crate (`^2`) for ANSI terminal styling. Lightweight, no significant transitive dependencies. Respects `NO_COLOR` environment variable automatically.

## Design System

### Signature Elements

- `◆` (purple) — info/status line prefix
- `✖` (red) — error prefix
- `░▒▓` (purple gradient) — banner decoration
- `█` blocks — brightness bars (orange filled, dark gray unfilled) and segment color previews
- `·` (dim) — separator in lists and inline metadata

### Color Palette

| Role             | Color     | ANSI Mapping      |
|------------------|-----------|--------------------|
| Brand/accent     | `#6C5CE7` | Purple (bold)      |
| Success/ON       | `#66BB6A` | Green              |
| Error/OFF        | `#EF5350` | Red                |
| Values/highlight | `#FFA500` | Yellow             |
| Secondary info   | `#4ECDC4` | Cyan               |
| Dim/labels       | —         | Dimmed / dark gray |
| Body text        | —         | Default terminal   |

Note: ANSI 256-color or truecolor will be used where the `colored` crate supports it (truecolor for segment blocks specifically, since those reflect actual RGB values). The palette above maps to standard ANSI for the fixed UI elements.

## Output Specifications

### 1. Banner

Shown with `--verbose` or during first auto-discovery.

```
░▒▓ govee v0.1.0
LAN control · no cloud · no keys
─────────────────────────────────
```

- `░▒▓` in ascending purple intensity
- `govee` in purple bold
- Version in dim
- Tagline in dim
- Separator in dim

### 2. Device Discovery

```
◆ Scanning for devices...
◆ Found H6159 at 192.168.1.42
```

- First line: purple diamond, dim text
- Second line: cyan diamond, device name in white/bold, IP in cyan

### 3. Action Confirmations

One-shot commands (on, off, brightness, color) print a single confirmation line:

```
◆ Power ON          (ON in green)
◆ Power OFF         (OFF in red)
◆ Brightness ████████░░ 80%    (bar in orange, percentage in orange)
◆ Color ██ #FF6B6B  (swatch in actual color via truecolor, hex in dim)
```

### 4. Status Display (`govee status`)

```
◆ Power ON
◆ Brightness ████████░░ 80%
◆ Color ██ #FF6B6B (255, 107, 107)
◆ Device 192.168.1.42 H6159
```

- Same formatting as action confirmations
- Color line includes both hex and RGB tuple
- Device line: IP in cyan, SKU in dim

### 5. Theme List (`govee theme --list`)

```
THEMES
│ STATIC
│ movie · chill · party · sunset · forest · ocean
│ NATURE
│ aurora · campfire · thunderstorm · sunrise · lava · tide
│ VIBES
│ cyberpunk · vaporwave · jazz · candlelight · rave · lofi
│ FUNCTIONAL
│ focus · relax · nightlight · reading · energize · warm
│ SEASONAL
│ halloween · christmas · valentine · spring · autumn · snow
```

- Header "THEMES" in purple, small caps style (all caps, letter-spaced feel not possible in terminal — just uppercase purple)
- Each category has a colored left border character (`│`):
  - Static → purple
  - Nature → cyan
  - Vibes → yellow/orange
  - Functional → green
  - Seasonal → red
- Category name in its accent color, uppercase
- Theme names in default color, separated by ` · ` (dim separator)

### 6. Theme Activation

```
◆ Theme aurora [nature]
◆ Brightness ████████░░ 80%
◆ Segments 15
  ██████████████████████████████
  Press Ctrl+C to stop
```

- Theme name in white/bold, category tag in dim brackets
- Segment preview: row of `██` blocks in truecolor showing the theme's palette colors
- "Press Ctrl+C to stop" in dim

### 7. Live Status Bar (Continuous Modes)

For screen capture, audio reactive, and animated theme modes. Initial config printed once (as above), then a single line that updates in place via `\r`:

```
██████████████████████████████ 30fps · smooth: 0.3
```

- Colored blocks in truecolor showing current segment colors
- FPS and mode-specific metadata in dim
- Updated via carriage return (`\r`) — no newline, overwrites same line
- On Ctrl+C: print newline, then cleanup messages

### 8. Error Messages

```
✖ No device found — is the strip powered on and connected to WiFi?
✖ Unknown theme "nonexistent"
  Run govee theme --list to see available themes
```

- `✖` in red
- Error message in red
- Contextual hint on next line, indented, in dim
- Hint references relevant command in default/bright color

## Architecture

### New Module: `src/ui.rs`

Centralizes all output formatting. No module should call `println!` directly for user-facing output (except `--debug` raw protocol dumps which stay plain).

```rust
// Core formatting functions
pub fn banner()                                    // startup banner
pub fn info(label: &str, value: &str)             // ◆ Label Value
pub fn error(msg: &str)                           // ✖ error message
pub fn error_hint(msg: &str, hint: &str)          // ✖ error + indented hint
pub fn brightness_bar(percent: u8) -> String       // ████████░░ 80%
pub fn color_swatch(r: u8, g: u8, b: u8) -> String // ██ #RRGGBB
pub fn segment_blocks(colors: &[(u8,u8,u8)]) -> String  // row of colored ██
pub fn theme_list(themes: &[Theme])               // categorized theme display
pub fn status_line(segments: &[(u8,u8,u8)], fps: u32, extra: &str)  // live updating line
pub fn discovery_scanning()                        // scanning message
pub fn discovery_found(name: &str, ip: &str)      // found device message
```

### Integration Points

All existing `println!`/`eprintln!` calls for user-facing output get replaced:

- **`main.rs`** — on/off/brightness/color confirmations, status display, scan results
- **`themes.rs`** — theme list display, theme activation, animation status
- **`ambient.rs`** — ambient mode startup, color update messages
- **`screen.rs`** — screen mode config display, live status bar
- **`audio_cmd.rs`** — audio mode config display, live status bar
- **`discovery.rs`** — scanning/found messages

### What Doesn't Change

- CLI argument structure (no new subcommands)
- Protocol layer (`protocol.rs`)
- Core logic in any module
- `--debug` raw UDP output stays plain text (diagnostic, not user-facing)
- No new CLI flags except automatic `NO_COLOR` support via `colored` crate

### `--no-color` / Pipe Detection

The `colored` crate automatically disables colors when stdout is not a TTY (piped output). It also respects the `NO_COLOR` environment variable. No manual flag needed.
