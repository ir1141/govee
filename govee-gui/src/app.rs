//! Application state, message handling, and view composition.
//!
//! The [`App`] struct owns device state, configuration, subprocess management,
//! and all page routing for the iced application.

use govee_lan::DeviceInfo;
use govee_themes::{ThemeDef, ThemeKind, load_all_themes};
use iced::widget::{column, container, row};
use iced::{Element, Length, Task};
use std::time::Duration;
use crate::config::{GuiConfig, MAX_SEGMENTS};
use crate::pages;
use crate::widgets::{sidebar, status_bar};

/// Transient text buffers for the sunlight page's four text inputs.
/// Never persisted — the parsed, validated values live on `config.sunlight`.
#[derive(Debug, Clone, Default)]
pub struct SunlightInputs {
    pub lat: String,
    pub lon: String,
    pub sunrise: String,
    pub sunset: String,
}

/// Inline error messages for the sunlight text inputs.
/// `None` means either Valid or Incomplete — only hard-Invalid renders.
#[derive(Debug, Clone, Default)]
pub struct SunlightInputErrors {
    pub lat: Option<String>,
    pub lon: Option<String>,
    pub sunrise: Option<String>,
    pub sunset: Option<String>,
}

/// Tri-state result for per-keystroke validation of a text input.
enum TriState<T> {
    /// Parses cleanly AND inside range — save and clear the error.
    Valid(T),
    /// Prefix of something that could still become valid — clear error, don't save.
    Incomplete,
    /// Cannot become valid with more characters — render the error, don't save.
    Invalid(String),
}

/// Commit a tri-state validation result into `slot`/`err`.
/// Returns true iff the value was `Valid` (caller should then persist).
fn apply_tristate<T>(
    result: TriState<T>,
    slot: &mut Option<T>,
    err: &mut Option<String>,
) -> bool {
    match result {
        TriState::Valid(v) => { *slot = Some(v); *err = None; true }
        TriState::Incomplete => { *err = None; false }
        TriState::Invalid(e) => { *err = Some(e); false }
    }
}

/// Validate a latitude or longitude text input.
/// `bound` is the absolute limit (90.0 for lat, 180.0 for lon).
fn validate_latlon(s: &str, bound: f64, label: &str) -> TriState<f64> {
    let t = s.trim();
    if t.is_empty() || t == "-" || t == "." || t == "-." {
        return TriState::Incomplete;
    }
    match t.parse::<f64>() {
        Ok(v) if v.is_finite() && v.abs() <= bound => TriState::Valid(v),
        Ok(_) => TriState::Invalid(format!("{label} must be in [-{bound}, {bound}]")),
        Err(_) => TriState::Invalid(format!("{label} is not a number")),
    }
}

/// Validate an HH:MM time input.
/// Incomplete = strict prefix of some valid "HH:MM" (00:00 .. 23:59);
/// Valid = full "HH:MM" matching that range; anything else = Invalid.
fn validate_time(s: &str) -> TriState<String> {
    let t = s;
    let bytes = t.as_bytes();

    fn hh_first_ok(c: u8) -> bool { matches!(c, b'0'..=b'2') }
    fn digit(c: u8) -> bool { c.is_ascii_digit() }

    match bytes.len() {
        0 => TriState::Incomplete,
        1 => {
            if hh_first_ok(bytes[0]) {
                TriState::Incomplete
            } else {
                TriState::Invalid("expected HH:MM".into())
            }
        }
        2 => {
            if digit(bytes[0]) && digit(bytes[1]) {
                let h = (bytes[0] - b'0') * 10 + (bytes[1] - b'0');
                if h <= 23 { TriState::Incomplete } else { TriState::Invalid("hour 00-23".into()) }
            } else {
                TriState::Invalid("expected HH:MM".into())
            }
        }
        3 => {
            if digit(bytes[0]) && digit(bytes[1]) && bytes[2] == b':' {
                let h = (bytes[0] - b'0') * 10 + (bytes[1] - b'0');
                if h <= 23 { TriState::Incomplete } else { TriState::Invalid("hour 00-23".into()) }
            } else {
                TriState::Invalid("expected HH:MM".into())
            }
        }
        4 => {
            if digit(bytes[0]) && digit(bytes[1]) && bytes[2] == b':' && digit(bytes[3]) {
                let h = (bytes[0] - b'0') * 10 + (bytes[1] - b'0');
                let m_hi = bytes[3] - b'0';
                if h <= 23 && m_hi <= 5 { TriState::Incomplete } else { TriState::Invalid("hour 00-23, minute 00-59".into()) }
            } else {
                TriState::Invalid("expected HH:MM".into())
            }
        }
        5 => {
            if digit(bytes[0]) && digit(bytes[1]) && bytes[2] == b':' && digit(bytes[3]) && digit(bytes[4]) {
                let h = (bytes[0] - b'0') * 10 + (bytes[1] - b'0');
                let m = (bytes[3] - b'0') * 10 + (bytes[4] - b'0');
                if h <= 23 && m <= 59 {
                    TriState::Valid(t.to_string())
                } else {
                    TriState::Invalid("hour 00-23, minute 00-59".into())
                }
            } else {
                TriState::Invalid("expected HH:MM".into())
            }
        }
        _ => TriState::Invalid("expected HH:MM".into()),
    }
}

/// Navigation pages in the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Controls,
    Themes,
    Screen,
    Audio,
    Ambient,
    Sunlight,
}

/// All messages the application can process.
#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Page),
    TogglePower,
    SetBrightness(u8),
    SetColor(u8, u8, u8),
    SetColorTemp(u16),
    DeviceCommandDone,
    DiscoveryTick,
    DevicesDiscovered(Vec<DeviceInfo>),
    SelectDevice(usize),
    ApplyTheme(String),
    StopMode,
    ThemeFilterChanged(String),
    Tick,

    // Screen settings
    SetScreenFps(u32),
    SetScreenBrightness(u8),
    SetScreenSegments(usize),
    StartScreen,

    // Audio settings
    SetAudioMode(String),
    SetAudioPalette(String),
    SetAudioBrightness(u8),
    SetAudioSensitivity(u8),
    SetAudioSegments(usize),
    ToggleAudioGradient(bool),
    StartAudio,

    // Ambient settings
    SetAmbientBrightness(u8),
    ToggleAmbientDim(bool),
    StartAmbient,

    // Sunlight settings
    SetSunlightPreset(String),
    SetSunlightBrightness(u8),
    SetSunlightSegments(usize),
    SetSunlightTransition(u32),
    StartSunlight,
    SetSunlightUseLocation(bool),
    EditSunlightLat(String),
    EditSunlightLon(String),
    EditSunlightSunrise(String),
    EditSunlightSunset(String),
    SubmitSunlightCoords,
    SetSunlightDayTemp(u16),
    SetSunlightNightTemp(u16),
    ToggleSunlightNightBrightnessOverride(bool),
    SetSunlightNightBrightness(u8),

    ToggleMirror(bool),
    SaveConfig,

    // Apply settings and restart subprocess if mode is active
    ApplyScreenSettings,
    ApplyAudioSettings,
    ApplyAmbientSettings,
    ApplySunlightSettings,
}

/// Main application state.
pub struct App {
    pub page: Page,
    pub device: Option<DeviceInfo>,
    pub devices: Vec<DeviceInfo>,
    pub config: GuiConfig,
    pub power: bool,
    pub brightness: u8,
    pub color: (u8, u8, u8),
    pub color_temp: u16,
    pub themes: Vec<ThemeDef>,
    pub active_theme: Option<String>,
    pub subprocess: Option<std::process::Child>,
    pub theme_filter: String,
    pub active_mode: Option<String>,
    pub elapsed_secs: u64,
    pub subprocess_start: Option<std::time::Instant>,
    pub mirror: bool,
    pub sunlight_inputs: SunlightInputs,
    pub sunlight_errors: SunlightInputErrors,
}

impl App {
    /// Populate `sunlight_errors` for the currently active mode (location or manual)
    /// from the current input buffers, flagging any missing/invalid pair.
    fn populate_sunlight_errors_from_inputs(&mut self) {
        if self.config.sunlight.use_location {
            if self.config.sunlight.lat.is_none() {
                if let TriState::Invalid(e) = validate_latlon(&self.sunlight_inputs.lat, 90.0, "latitude") {
                    self.sunlight_errors.lat = Some(e);
                } else {
                    self.sunlight_errors.lat = Some("set latitude".into());
                }
            }
            if self.config.sunlight.lon.is_none() {
                if let TriState::Invalid(e) = validate_latlon(&self.sunlight_inputs.lon, 180.0, "longitude") {
                    self.sunlight_errors.lon = Some(e);
                } else {
                    self.sunlight_errors.lon = Some("set longitude".into());
                }
            }
        } else {
            if self.config.sunlight.sunrise.is_none() {
                if let TriState::Invalid(e) = validate_time(&self.sunlight_inputs.sunrise) {
                    self.sunlight_errors.sunrise = Some(e);
                } else {
                    self.sunlight_errors.sunrise = Some("set sunrise HH:MM".into());
                }
            }
            if self.config.sunlight.sunset.is_none() {
                if let TriState::Invalid(e) = validate_time(&self.sunlight_inputs.sunset) {
                    self.sunlight_errors.sunset = Some(e);
                } else {
                    self.sunlight_errors.sunset = Some("set sunset HH:MM".into());
                }
            }
        }
    }

    /// Kill the current subprocess and clear associated state.
    fn stop_subprocess(&mut self) {
        if let Some(ref mut child) = self.subprocess {
            crate::subprocess::kill(child);
        }
        self.subprocess = None;
        self.active_theme = None;
        self.active_mode = None;
        self.subprocess_start = None;
        self.elapsed_secs = 0;
    }

    /// If the given mode is currently running, restart its subprocess with current config.
    fn restart_if_active(&mut self, mode: &str) {
        if self.active_mode.as_deref() != Some(mode) {
            return;
        }
        match mode {
            "screen" => {
                let s = &self.config.screen;
                self.start_subprocess("screen", vec![
                    "screen".into(),
                    "--fps".into(), s.fps.to_string(),
                    "--brightness".into(), s.brightness.to_string(),
                    "--segments".into(), s.segments.to_string(),
                ]);
            }
            "audio" => {
                let a = &self.config.audio;
                let mut args = vec![
                    "audio".into(),
                    "--mode".into(), a.mode.clone(),
                    "--palette".into(), a.palette.clone(),
                    "--brightness".into(), a.brightness.to_string(),
                    "--segments".into(), a.segments.to_string(),
                    "--sensitivity".into(), a.sensitivity.to_string(),
                ];
                if a.gradient { args.push("--gradient".into()); }
                self.start_subprocess("audio", args);
            }
            "ambient" => {
                let amb = &self.config.ambient;
                let mut args = vec![
                    "ambient".into(),
                    "--brightness".into(), amb.brightness.to_string(),
                ];
                if amb.dim { args.push("--dim".into()); }
                self.start_subprocess("ambient", args);
            }
            "sunlight" => {
                if !self.config.sunlight.is_restartable() {
                    return;
                }
                let mut args = vec!["sunlight".into()];
                args.extend(self.config.sunlight.build_cli_args());
                self.start_subprocess("sunlight", args);
            }
            _ => {}
        }
    }

    /// Spawn a govee CLI subprocess for a continuous mode.
    fn start_subprocess(&mut self, mode: &str, args: Vec<String>) {
        self.stop_subprocess();
        if let Some(ref dev) = self.device {
            let mut args = args;
            if self.mirror { args.push("--mirror".into()); }
            let ip = dev.ip.clone();
            let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            if let Ok(child) = crate::subprocess::spawn_govee(&arg_refs, Some(&ip)) {
                self.subprocess = Some(child);
                self.active_mode = Some(mode.into());
                self.subprocess_start = Some(std::time::Instant::now());
                self.elapsed_secs = 0;
            }
        }
    }

    pub fn new() -> (Self, Task<Message>) {
        let config = GuiConfig::load();
        let page = match config.general.last_page.as_str() {
            "themes" => Page::Themes,
            "screen" => Page::Screen,
            "audio" => Page::Audio,
            "ambient" => Page::Ambient,
            "sunlight" => Page::Sunlight,
            _ => Page::Controls,
        };
        let brightness = config.controls.brightness;
        let color = (
            config.controls.color[0],
            config.controls.color[1],
            config.controls.color[2],
        );
        let color_temp = config.controls.color_temp;
        let mirror = config.screen.mirror;
        let sunlight_inputs = SunlightInputs {
            lat: config.sunlight.lat.map(|v| v.to_string()).unwrap_or_default(),
            lon: config.sunlight.lon.map(|v| v.to_string()).unwrap_or_default(),
            sunrise: config.sunlight.sunrise.clone().unwrap_or_default(),
            sunset: config.sunlight.sunset.clone().unwrap_or_default(),
        };
        let app = Self {
            page,
            device: None,
            devices: vec![],
            config,
            power: true,
            brightness,
            color,
            color_temp,
            themes: load_all_themes(),
            active_theme: None,
            subprocess: None,
            theme_filter: "all".into(),
            active_mode: None,
            elapsed_secs: 0,
            mirror,
            subprocess_start: None,
            sunlight_inputs,
            sunlight_errors: SunlightInputErrors::default(),
        };
        let init_task = Task::perform(
            async {
                tokio::task::spawn_blocking(|| govee_lan::scan_devices(Duration::from_secs(2)))
                    .await
                    .unwrap_or_default()
            },
            Message::DevicesDiscovered,
        );
        (app, init_task)
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let mut subs = vec![
            iced::time::every(Duration::from_secs(10)).map(|_| Message::DiscoveryTick),
        ];
        if self.subprocess.is_some() {
            subs.push(iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick));
        }
        iced::Subscription::batch(subs)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(page) => {
                self.page = page;
                self.config.general.last_page = match page {
                    Page::Controls => "controls",
                    Page::Themes => "themes",
                    Page::Screen => "screen",
                    Page::Audio => "audio",
                    Page::Ambient => "ambient",
                    Page::Sunlight => "sunlight",
                }
                .into();
                self.config.save();
            }
            Message::TogglePower => {
                self.power = !self.power;
                if let Some(ref dev) = self.device {
                    let ip = dev.ip.clone();
                    let on = self.power;
                    return Task::perform(
                        async move {
                            govee_lan::send_turn(&ip, on).map_err(|e| e.to_string())
                        },
                        |_| Message::DeviceCommandDone,
                    );
                }
            }
            Message::SetBrightness(value) => {
                self.brightness = value;
                self.config.controls.brightness = value;
                if let Some(ref dev) = self.device {
                    let ip = dev.ip.clone();
                    return Task::perform(
                        async move {
                            govee_lan::send_brightness(&ip, value).map_err(|e| e.to_string())
                        },
                        |_| Message::DeviceCommandDone,
                    );
                }
            }
            Message::SetColor(r, g, b) => {
                self.color = (r, g, b);
                self.config.controls.color = [r, g, b];
                if let Some(ref dev) = self.device {
                    let ip = dev.ip.clone();
                    return Task::perform(
                        async move {
                            govee_lan::send_color(&ip, r, g, b).map_err(|e| e.to_string())
                        },
                        |_| Message::DeviceCommandDone,
                    );
                }
            }
            Message::SetColorTemp(kelvin) => {
                self.color_temp = kelvin;
                self.config.controls.color_temp = kelvin;
                if let Some(ref dev) = self.device {
                    let ip = dev.ip.clone();
                    return Task::perform(
                        async move {
                            govee_lan::send_color_temp(&ip, kelvin).map_err(|e| e.to_string())
                        },
                        |_| Message::DeviceCommandDone,
                    );
                }
            }
            Message::DeviceCommandDone => {}
            Message::DiscoveryTick => {
                return Task::perform(
                    async {
                        tokio::task::spawn_blocking(|| govee_lan::scan_devices(Duration::from_secs(2)))
                            .await
                            .unwrap_or_default()
                    },
                    Message::DevicesDiscovered,
                );
            }
            Message::DevicesDiscovered(devices) => {
                self.devices = devices;
                if self.device.is_none() {
                    let target_ip = self.config.general.last_device_ip.as_deref();
                    let pick = self
                        .devices
                        .iter()
                        .find(|d| target_ip.is_some_and(|ip| ip == d.ip))
                        .or_else(|| self.devices.first());
                    if let Some(dev) = pick {
                        self.device = Some(dev.clone());
                        self.config.general.last_device_ip = Some(dev.ip.clone());
                        self.config.save();
                    }
                }
            }
            Message::SelectDevice(idx) => {
                if let Some(dev) = self.devices.get(idx) {
                    self.device = Some(dev.clone());
                    self.config.general.last_device_ip = Some(dev.ip.clone());
                    self.config.save();
                }
            }
            Message::ApplyTheme(name) => {
                self.stop_subprocess();
                let theme = self.themes.iter().find(|t| t.name == name).cloned();
                if let (Some(theme), Some(ref dev)) = (theme, &self.device) {
                    match &theme.kind {
                        ThemeKind::Solid { color } => {
                            let ip = dev.ip.clone();
                            let (r, g, b) = *color;
                            self.active_theme = Some(name);
                            return Task::perform(
                                async move {
                                    govee_lan::send_color(&ip, r, g, b).map_err(|e| e.to_string())
                                },
                                |_| Message::DeviceCommandDone,
                            );
                        }
                        ThemeKind::Animated { .. } => {
                            let ip = dev.ip.clone();
                            let mut args = vec!["theme".to_string(), name.clone()];
                            if self.mirror { args.push("--mirror".into()); }
                            let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                            if let Ok(child) = crate::subprocess::spawn_govee(&arg_refs, Some(&ip)) {
                                self.subprocess = Some(child);
                                self.active_theme = Some(name);
                                self.subprocess_start = Some(std::time::Instant::now());
                                self.elapsed_secs = 0;
                            }
                        }
                    }
                }
            }
            Message::StopMode => {
                self.stop_subprocess();
            }
            Message::ThemeFilterChanged(filter) => {
                self.theme_filter = filter;
            }
            Message::Tick => {
                if let Some(start) = self.subprocess_start {
                    self.elapsed_secs = start.elapsed().as_secs();
                }
            }
            Message::SetScreenFps(v) => { self.config.screen.fps = v; }
            Message::SetScreenBrightness(v) => { self.config.screen.brightness = v; }
            Message::SetScreenSegments(v) => { self.config.screen.segments = v.min(MAX_SEGMENTS); }
            Message::SetAudioMode(v) => { self.config.audio.mode = v; self.config.save(); self.restart_if_active("audio"); }
            Message::SetAudioPalette(v) => { self.config.audio.palette = v; self.config.save(); self.restart_if_active("audio"); }
            Message::SetAudioBrightness(v) => { self.config.audio.brightness = v; }
            Message::SetAudioSensitivity(v) => { self.config.audio.sensitivity = v as f64 / 10.0; }
            Message::SetAudioSegments(v) => { self.config.audio.segments = v.min(MAX_SEGMENTS); }
            Message::ToggleAudioGradient(v) => { self.config.audio.gradient = v; self.config.save(); self.restart_if_active("audio"); }
            Message::SetAmbientBrightness(v) => { self.config.ambient.brightness = v; }
            Message::SaveConfig => { self.config.save(); }
            Message::ToggleAmbientDim(v) => { self.config.ambient.dim = v; self.config.save(); self.restart_if_active("ambient"); }
            Message::ToggleMirror(v) => {
                self.mirror = v;
                self.config.screen.mirror = v;
                self.config.audio.mirror = v;
                self.config.save();
                if let Some(mode) = self.active_mode.clone() {
                    self.restart_if_active(&mode);
                }
            }
            Message::StartScreen => {
                let s = &self.config.screen;
                self.start_subprocess("screen", vec![
                    "screen".into(),
                    "--fps".into(), s.fps.to_string(),
                    "--brightness".into(), s.brightness.to_string(),
                    "--segments".into(), s.segments.to_string(),
                ]);
            }
            Message::StartAudio => {
                let a = &self.config.audio;
                let mut args = vec![
                    "audio".into(),
                    "--mode".into(), a.mode.clone(),
                    "--palette".into(), a.palette.clone(),
                    "--brightness".into(), a.brightness.to_string(),
                    "--segments".into(), a.segments.to_string(),
                    "--sensitivity".into(), a.sensitivity.to_string(),
                ];
                if a.gradient { args.push("--gradient".into()); }
                self.start_subprocess("audio", args);
            }
            Message::SetSunlightPreset(v) => { self.config.sunlight.preset = v; self.config.save(); self.restart_if_active("sunlight"); }
            Message::SetSunlightBrightness(v) => { self.config.sunlight.brightness = v; }
            Message::SetSunlightSegments(v) => { self.config.sunlight.segments = v.min(MAX_SEGMENTS); }
            Message::SetSunlightTransition(v) => { self.config.sunlight.transition = v; }
            Message::StartSunlight => {
                if !self.config.sunlight.is_restartable() {
                    self.populate_sunlight_errors_from_inputs();
                    return Task::none();
                }
                let mut args = vec!["sunlight".into()];
                args.extend(self.config.sunlight.build_cli_args());
                self.start_subprocess("sunlight", args);
            }
            Message::SetSunlightUseLocation(v) => {
                self.config.sunlight.use_location = v;
                self.config.save();
                self.restart_if_active("sunlight");
            }
            Message::EditSunlightLat(s) => {
                self.sunlight_inputs.lat = s;
                let r = validate_latlon(&self.sunlight_inputs.lat, 90.0, "latitude");
                if apply_tristate(r, &mut self.config.sunlight.lat, &mut self.sunlight_errors.lat) {
                    self.config.save();
                }
            }
            Message::EditSunlightLon(s) => {
                self.sunlight_inputs.lon = s;
                let r = validate_latlon(&self.sunlight_inputs.lon, 180.0, "longitude");
                if apply_tristate(r, &mut self.config.sunlight.lon, &mut self.sunlight_errors.lon) {
                    self.config.save();
                }
            }
            Message::EditSunlightSunrise(s) => {
                self.sunlight_inputs.sunrise = s;
                let r = validate_time(&self.sunlight_inputs.sunrise);
                if apply_tristate(r, &mut self.config.sunlight.sunrise, &mut self.sunlight_errors.sunrise) {
                    self.config.save();
                }
            }
            Message::EditSunlightSunset(s) => {
                self.sunlight_inputs.sunset = s;
                let r = validate_time(&self.sunlight_inputs.sunset);
                if apply_tristate(r, &mut self.config.sunlight.sunset, &mut self.sunlight_errors.sunset) {
                    self.config.save();
                }
            }
            Message::SubmitSunlightCoords => {
                if self.config.sunlight.is_restartable() {
                    self.config.save();
                    self.restart_if_active("sunlight");
                } else {
                    self.populate_sunlight_errors_from_inputs();
                }
            }
            Message::SetSunlightDayTemp(v) => { self.config.sunlight.day_temp = v; }
            Message::SetSunlightNightTemp(v) => { self.config.sunlight.night_temp = v; }
            Message::ToggleSunlightNightBrightnessOverride(on) => {
                self.config.sunlight.night_brightness = if on {
                    Some(self.config.sunlight.night_brightness.unwrap_or(30))
                } else {
                    None
                };
                self.config.save();
                self.restart_if_active("sunlight");
            }
            Message::SetSunlightNightBrightness(v) => {
                self.config.sunlight.night_brightness = Some(v);
            }
            Message::ApplyScreenSettings => { self.config.save(); self.restart_if_active("screen"); }
            Message::ApplyAudioSettings => { self.config.save(); self.restart_if_active("audio"); }
            Message::ApplyAmbientSettings => { self.config.save(); self.restart_if_active("ambient"); }
            Message::ApplySunlightSettings => { self.config.save(); self.restart_if_active("sunlight"); }
            Message::StartAmbient => {
                let amb = &self.config.ambient;
                let mut args = vec![
                    "ambient".into(),
                    "--brightness".into(), amb.brightness.to_string(),
                ];
                if amb.dim { args.push("--dim".into()); }
                self.start_subprocess("ambient", args);
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let device_label = self
            .device
            .as_ref()
            .map(|d| format!("{} • {}", d.sku, d.ip))
            .unwrap_or_else(|| "No device".into());

        let sidebar = sidebar::view(
            self.page,
            &device_label,
            &self.devices,
            self.device.as_ref().map(|d| d.ip.as_str()),
        );

        let page_content: Element<Message> = match self.page {
            Page::Controls => pages::controls::view(self),
            Page::Themes => pages::themes::view(self),
            Page::Screen => pages::screen::view(self),
            Page::Audio => pages::audio::view(self),
            Page::Ambient => pages::ambient::view(self),
            Page::Sunlight => pages::sunlight::view(self),
        };

        let content = container(page_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(24);

        let mode_label = if let Some(ref mode) = self.active_mode {
            match mode.as_str() {
                "screen" => format!("Screen Capture · {}s", self.elapsed_secs),
                "audio" => format!("Audio Reactive · {}s", self.elapsed_secs),
                "ambient" => format!("Ambient Sync · {}s", self.elapsed_secs),
                "sunlight" => format!("Sunlight · {}s", self.elapsed_secs),
                _ => format!("Mode: {mode}"),
            }
        } else if let Some(ref theme) = self.active_theme {
            if self.subprocess.is_some() {
                format!("Theme: {} · {}s", theme, self.elapsed_secs)
            } else {
                format!("Theme: {theme}")
            }
        } else {
            "Idle".to_string()
        };
        let main = column![
            row![sidebar, content].height(Length::Fill),
            status_bar::view(self.device.is_some(), &mode_label, self.mirror),
        ];

        container(main)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.subprocess {
            crate::subprocess::kill(child);
            let _ = child.wait();
        }
    }
}
