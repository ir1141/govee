//! Persistent GUI configuration stored as TOML at `~/.config/govee/gui.toml`.
//! Each page's settings are preserved across sessions.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Hard cap on segment count. The device firmware stops responding when
/// DreamView segment packets exceed this, so the GUI clamps everywhere.
pub const MAX_SEGMENTS: usize = 10;

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("govee")
        .join("gui.toml")
}

/// Top-level GUI configuration with per-page sections.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuiConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub controls: ControlsConfig,
    #[serde(default)]
    pub screen: ScreenConfig,
    #[serde(default)]
    pub audio: AudioConfig,
    #[serde(default)]
    pub ambient: AmbientConfig,
    #[serde(default)]
    pub sunlight: SunlightConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub last_device_ip: Option<String>,
    pub last_page: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlsConfig {
    pub brightness: u8,
    pub color: [u8; 3],
    pub color_temp: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScreenConfig {
    pub fps: u32,
    pub brightness: u8,
    pub smoothing: f64,
    pub threshold: u32,
    pub segments: usize,
    pub saturation: f64,
    pub output: Option<String>,
    pub gradient: bool,
    pub mirror: bool,
    pub no_dreamview: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    pub mode: String,
    pub palette: String,
    pub brightness: u8,
    pub smoothing: f64,
    pub sensitivity: f64,
    pub segments: usize,
    pub gradient: bool,
    pub mirror: bool,
    pub no_dreamview: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AmbientConfig {
    pub color: String,
    pub brightness: u8,
    pub dim: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            last_device_ip: None,
            last_page: "controls".to_string(),
        }
    }
}

impl Default for ControlsConfig {
    fn default() -> Self {
        Self {
            brightness: 80,
            color: [124, 58, 237],
            color_temp: 4000,
        }
    }
}

impl Default for ScreenConfig {
    fn default() -> Self {
        Self {
            fps: 10,
            brightness: 80,
            smoothing: 0.3,
            threshold: 10,
            segments: 5,
            saturation: 1.0,
            output: None,
            gradient: false,
            mirror: false,
            no_dreamview: false,
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            mode: "energy".to_string(),
            palette: "fire".to_string(),
            brightness: 80,
            smoothing: 0.3,
            sensitivity: 1.0,
            segments: 5,
            gradient: false,
            mirror: false,
            no_dreamview: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SunlightConfig {
    pub preset: String,
    pub brightness: u8,
    pub segments: usize,
    pub transition: u32,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub sunrise: Option<String>,
    pub sunset: Option<String>,
    pub day_temp: u16,
    pub night_temp: u16,
    pub night_brightness: Option<u8>,
    #[serde(default = "default_use_location")]
    pub use_location: bool,
}

fn default_use_location() -> bool { true }

impl Default for SunlightConfig {
    fn default() -> Self {
        Self {
            preset: "coastal".to_string(),
            brightness: 80,
            segments: 10,
            transition: 45,
            lat: None,
            lon: None,
            sunrise: None,
            sunset: None,
            day_temp: 6500,
            night_temp: 3000,
            night_brightness: None,
            use_location: true,
        }
    }
}

impl Default for AmbientConfig {
    fn default() -> Self {
        Self {
            color: "primary".to_string(),
            brightness: 80,
            dim: false,
        }
    }
}

impl SunlightConfig {
    /// All CLI args after the `"sunlight"` subcommand token.
    /// Caller owns the subcommand name, device IP, and global flags (`--mirror`).
    pub fn build_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--preset".into(), self.preset.clone(),
            "--brightness".into(), self.brightness.to_string(),
            "--segments".into(), self.segments.to_string(),
            "--transition".into(), self.transition.to_string(),
        ];
        if self.use_location {
            if let (Some(lat), Some(lon)) = (self.lat, self.lon) {
                args.extend(["--lat".into(), lat.to_string(),
                             "--lon".into(), lon.to_string()]);
            }
        } else if let (Some(rise), Some(set)) = (&self.sunrise, &self.sunset) {
            args.extend(["--sunrise".into(), rise.clone(),
                         "--sunset".into(), set.clone()]);
        }
        if self.preset == "simple" {
            args.extend(["--day-temp".into(), self.day_temp.to_string(),
                         "--night-temp".into(), self.night_temp.to_string()]);
            if let Some(nb) = self.night_brightness {
                args.extend(["--night-brightness".into(), nb.to_string()]);
            }
        }
        args
    }

    /// Whether the current config can legally spawn or restart a sunlight subprocess.
    /// GUI-side gate (stricter than the CLI parser) that prevents spawning with missing data.
    pub fn is_restartable(&self) -> bool {
        if self.use_location {
            self.lat.is_some() && self.lon.is_some()
        } else {
            self.sunrise.is_some() && self.sunset.is_some()
        }
    }
}


impl GuiConfig {
    /// Load config from disk with section-level resilience: a malformed section
    /// only defaults that section, not the whole file. Top-level parse failure
    /// logs once and full-defaults.
    pub fn load() -> Self {
        let path = config_path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let table: toml::Table = match content.parse() {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "govee-gui: failed to parse {}: {e}. Using defaults.",
                    path.display()
                );
                return Self::default();
            }
        };

        fn section<T>(table: &toml::Table, key: &str, path: &std::path::Path) -> T
        where
            T: serde::de::DeserializeOwned + Default,
        {
            match table.get(key).cloned() {
                None => T::default(),
                Some(value) => match value.try_into::<T>() {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        eprintln!(
                            "govee-gui: failed to parse [{key}] in {}: {e}. Using defaults for this section.",
                            path.display()
                        );
                        T::default()
                    }
                },
            }
        }

        let mut cfg = GuiConfig {
            general: section(&table, "general", &path),
            controls: section(&table, "controls", &path),
            screen: section(&table, "screen", &path),
            audio: section(&table, "audio", &path),
            ambient: section(&table, "ambient", &path),
            sunlight: section(&table, "sunlight", &path),
        };
        cfg.screen.segments = cfg.screen.segments.clamp(1, MAX_SEGMENTS);
        cfg.audio.segments = cfg.audio.segments.clamp(1, MAX_SEGMENTS);
        cfg.sunlight.segments = cfg.sunlight.segments.clamp(1, MAX_SEGMENTS);
        cfg
    }

    /// Write config to disk, creating parent directories if needed.
    pub fn save(&self) {
        let path = config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = std::fs::write(&path, content);
        }
    }
}
