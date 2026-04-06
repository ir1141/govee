use clap::{Args, Parser, Subcommand};
use govee_lan::audio::{Palette, VisMode};

use crate::themes;

#[derive(Parser)]
#[command(name = "govee", about = "Control Govee LED strip lights over LAN")]
#[command(after_help = format!("Themes:\n{}", themes::theme_list_display()))]
pub struct Cli {
    #[arg(long, global = true, help = "Show raw UDP messages")]
    pub debug: bool,

    #[arg(long, global = true, help = "Mirror segments for U-shaped strip layout")]
    pub mirror: bool,

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
    #[command(alias = "scene")]
    Theme {
        #[arg(help = "Theme name (see 'govee theme --help' for list)")]
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
        value_parser = |s: &str| -> Result<usize, String> {
            let v: usize = s.parse().map_err(|_| format!("invalid number '{s}'"))?;
            if v < 1 || v > 127 { return Err(format!("segments must be 1-127, got {v}")); }
            Ok(v)
        },
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
    pub mode: VisMode,

    #[arg(long, default_value = "fire", help = "Color palette")]
    pub palette: Palette,

    #[arg(long, default_value_t = 80, help = "Strip brightness 1-100")]
    pub brightness: u8,

    #[arg(long, default_value_t = 5, value_parser = |s: &str| -> Result<usize, String> {
        let v: usize = s.parse().map_err(|_| format!("invalid number '{s}'"))?;
        if v < 1 || v > 127 { return Err(format!("segments must be 1-127, got {v}")); }
        Ok(v)
    }, help = "Number of DreamView segments (max 127 for mirror support)")]
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
