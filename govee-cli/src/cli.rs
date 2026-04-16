//! Command-line argument definitions using clap derive.

use clap::{Args, Parser, Subcommand};

/// CLI-side visualization mode (maps to [`govee_lan::audio::VisMode`]).
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CliVisMode {
    Energy, Frequency, Beat, Drop,
}

/// CLI-side palette selection (maps to [`govee_lan::audio::Palette`]).
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CliPalette {
    Fire, Ocean, Forest, Neon, Ice, Sunset, Rainbow,
}

impl From<CliVisMode> for govee_lan::audio::VisMode {
    fn from(m: CliVisMode) -> Self {
        match m {
            CliVisMode::Energy => Self::Energy,
            CliVisMode::Frequency => Self::Frequency,
            CliVisMode::Beat => Self::Beat,
            CliVisMode::Drop => Self::Drop,
        }
    }
}

impl From<CliPalette> for govee_lan::audio::Palette {
    fn from(p: CliPalette) -> Self {
        match p {
            CliPalette::Fire => Self::Fire,
            CliPalette::Ocean => Self::Ocean,
            CliPalette::Forest => Self::Forest,
            CliPalette::Neon => Self::Neon,
            CliPalette::Ice => Self::Ice,
            CliPalette::Sunset => Self::Sunset,
            CliPalette::Rainbow => Self::Rainbow,
        }
    }
}

/// CLI-side sunlight preset selection.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CliSunlightPreset {
    Coastal, Arctic, Ember, Simple,
}

use crate::themes;

/// Validate segment count is within the DreamView range (1-127).
fn parse_segments_127(s: &str) -> Result<usize, String> {
    let v: usize = s.parse().map_err(|_| format!("invalid number '{s}'"))?;
    if !(1..=127).contains(&v) {
        return Err(format!("segments must be 1-127, got {v}"));
    }
    Ok(v)
}

#[derive(Parser)]
#[command(name = "govee", about = "Control Govee LED strip lights over LAN")]
pub struct Cli {
    #[arg(long, global = true, help = "Show raw UDP messages")]
    pub debug: bool,

    #[arg(long, global = true, help = "Mirror segments for U-shaped strip layout")]
    pub mirror: bool,

    #[arg(long, short, global = true, help = "Suppress informational output")]
    pub quiet: bool,

    #[arg(long, global = true, help = "Device IP (auto-discovers if omitted)")]
    pub ip: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Discover Govee devices on the network
    Scan,
    /// Turn on
    On,
    /// Turn off
    Off,
    /// Set brightness (1-100)
    Brightness {
        #[arg(help = "Brightness level (1-100)")]
        value: u8,
    },
    /// Set RGB color
    Color {
        #[arg(help = "Red (0-255)")]
        r: u8,
        #[arg(help = "Green (0-255)")]
        g: u8,
        #[arg(help = "Blue (0-255)")]
        b: u8,
    },
    /// Set color temperature (2000-9000K)
    Temp {
        #[arg(help = "Color temperature in Kelvin (2000-9000)")]
        kelvin: u16,
    },
    /// Query device status
    Status,
    /// Dim to near-black while keeping device responsive to commands (avoids rediscovery delay of full off)
    Sleep,
    /// Reset device to a known good state (deactivates DreamView, turns on, full brightness, warm white)
    Reset,
    /// Apply a theme (static or animated). Static themes set a color once. Animated themes loop until Ctrl+C.
    #[command(alias = "scene", after_help = format!("Themes:\n{}", themes::theme_list_display()))]
    Theme {
        #[arg(help = "Theme name (see list below)")]
        name: String,
        #[arg(long, default_value_t = 60, help = "Strip brightness 1-100")]
        brightness: u8,
        #[arg(long, default_value_t = 5, value_parser = |s: &str| -> Result<usize, String> {
            let v: usize = s.parse().map_err(|_| format!("invalid number '{s}'"))?;
            if v < 1 || v > 75 { return Err(format!("segments must be 1-75, got {v}")); }
            Ok(v)
        }, help = "Number of segments for animated themes (max 75, doubled with --mirror)")]
        segments: usize,
    },
    /// Sync Govee strip with Caelestia wallpaper theme
    Ambient(AmbientArgs),
    /// Sync Govee strip with screen content (Ambilight-style)
    Screen(ScreenArgs),
    /// React to system audio with LED visualizations
    Audio(AudioArgs),
    /// Adjust LED mood based on time of day (ocean by day, fireplace by night)
    Sunlight(SunlightArgs),
}

#[derive(Args)]
pub struct AmbientArgs {
    #[arg(
        long,
        default_value = "primary",
        help = "Which theme color to use"
    )]
    pub color: String,

    #[arg(long, default_value_t = 80, help = "Strip brightness 1-100")]
    pub brightness: u8,

    #[arg(long, help = "Use the Dim variant of the color")]
    pub dim: bool,

    #[arg(short, long, help = "Verbose output")]
    pub verbose: bool,
}

#[derive(Args)]
pub struct SunlightArgs {
    #[arg(long, default_value = "coastal", help = "Preset: coastal, arctic, ember, simple")]
    pub preset: CliSunlightPreset,

    #[arg(long, allow_negative_numbers = true, help = "Latitude for solar calculation")]
    pub lat: Option<f64>,

    #[arg(long, allow_negative_numbers = true, help = "Longitude for solar calculation")]
    pub lon: Option<f64>,

    #[arg(long, help = "Manual sunrise time (HH:MM)")]
    pub sunrise: Option<String>,

    #[arg(long, help = "Manual sunset time (HH:MM)")]
    pub sunset: Option<String>,

    #[arg(long, default_value_t = 45, help = "Transition duration in minutes")]
    pub transition: u32,

    #[arg(long, default_value_t = 80, help = "Strip brightness 1-100")]
    pub brightness: u8,

    #[arg(long, help = "Night brightness 1-100 (omit to keep constant)")]
    pub night_brightness: Option<u8>,

    #[arg(long, default_value_t = 15, value_parser = parse_segments_127, help = "Number of DreamView segments")]
    pub segments: usize,

    #[arg(long, default_value_t = 6500, help = "Day color temperature (simple preset only)")]
    pub day_temp: u16,

    #[arg(long, default_value_t = 3000, help = "Night color temperature (simple preset only)")]
    pub night_temp: u16,

    #[arg(short, long, help = "Verbose output")]
    pub verbose: bool,
}

#[derive(Args)]
pub struct ScreenArgs {
    #[arg(long, default_value_t = 10, help = "Screen capture rate")]
    pub fps: u32,

    #[arg(long, default_value_t = 80, help = "Strip brightness 1-100")]
    pub brightness: u8,

    #[arg(
        long,
        default_value_t = 0.3,
        help = "Color transition speed 0.0-1.0 (higher=faster)"
    )]
    pub smoothing: f64,

    #[arg(long, default_value_t = 10, help = "Min color change to send update")]
    pub threshold: u32,

    #[arg(long, help = "Wayland output/monitor name (e.g. DP-1, HDMI-A-1)")]
    pub output: Option<String>,

    #[arg(
        long,
        default_value_t = 5,
        value_parser = parse_segments_127,
        help = "Number of color zones across top edge (max 127 for mirror support)"
    )]
    pub segments: usize,

    #[arg(long, help = "Interpolate between segment colors")]
    pub gradient: bool,

    #[arg(
        long,
        default_value_t = 1.0,
        help = "Boost color saturation (1.0=normal, 1.5=vivid)"
    )]
    pub saturate: f64,

    #[arg(
        long,
        help = "Use single colorwc instead of DreamView segments"
    )]
    pub no_dreamview: bool,

    #[arg(short, long, help = "Verbose output")]
    pub verbose: bool,
}

#[derive(Args)]
pub struct AudioArgs {
    #[arg(long, default_value = "energy", help = "Visualization mode")]
    pub mode: CliVisMode,

    #[arg(long, default_value = "fire", help = "Color palette")]
    pub palette: CliPalette,

    #[arg(long, default_value_t = 80, help = "Strip brightness 1-100")]
    pub brightness: u8,

    #[arg(long, default_value_t = 5, value_parser = parse_segments_127, help = "Number of DreamView segments (max 127 for mirror support)")]
    pub segments: usize,

    #[arg(long, default_value_t = 0.3, help = "Color transition speed 0.0-1.0")]
    pub smoothing: f64,

    #[arg(long, default_value_t = 1.0, help = "Audio gain multiplier")]
    pub sensitivity: f64,

    #[arg(long, help = "Use single colorwc instead of DreamView segments")]
    pub no_dreamview: bool,

    #[arg(long, help = "Interpolate between segment colors")]
    pub gradient: bool,

    #[arg(short, long, help = "Verbose output")]
    pub verbose: bool,
}
