use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("govee")
        .join("gui.toml")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub last_device_ip: Option<String>,
    pub last_page: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlsConfig {
    pub brightness: u8,
    pub color: [u8; 3],
    pub color_temp: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for AmbientConfig {
    fn default() -> Self {
        Self {
            color: "primary".to_string(),
            brightness: 80,
            dim: false,
        }
    }
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            controls: ControlsConfig::default(),
            screen: ScreenConfig::default(),
            audio: AudioConfig::default(),
            ambient: AmbientConfig::default(),
        }
    }
}

impl GuiConfig {
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

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
