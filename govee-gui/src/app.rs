use govee_lan::DeviceInfo;
use iced::widget::{column, container, row};
use iced::{Element, Length, Task};
use std::time::Duration;
use crate::config::GuiConfig;
use crate::pages;
use crate::widgets::{sidebar, status_bar};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Controls,
    Themes,
    Screen,
    Audio,
    Ambient,
}

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
    ToggleScreenMirror(bool),
    StartScreen,

    // Audio settings
    SetAudioMode(String),
    SetAudioBrightness(u8),
    SetAudioSegments(usize),
    ToggleAudioMirror(bool),
    StartAudio,

    // Ambient settings
    SetAmbientBrightness(u8),
    ToggleAmbientDim(bool),
    StartAmbient,

    ToggleMirror(bool),
}

pub struct App {
    pub page: Page,
    pub device: Option<DeviceInfo>,
    pub devices: Vec<DeviceInfo>,
    pub config: GuiConfig,
    pub power: bool,
    pub brightness: u8,
    pub color: (u8, u8, u8),
    pub color_temp: u16,
    pub themes: Vec<govee_lan::ThemeDef>,
    pub active_theme: Option<String>,
    pub subprocess: Option<std::process::Child>,
    pub theme_filter: String,
    pub active_mode: Option<String>,
    pub elapsed_secs: u64,
    pub subprocess_start: Option<std::time::Instant>,
    pub mirror: bool,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let config = GuiConfig::load();
        let page = match config.general.last_page.as_str() {
            "themes" => Page::Themes,
            "screen" => Page::Screen,
            "audio" => Page::Audio,
            "ambient" => Page::Ambient,
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
        let app = Self {
            page,
            device: None,
            devices: vec![],
            config,
            power: true,
            brightness,
            color,
            color_temp,
            themes: govee_lan::load_all_themes(),
            active_theme: None,
            subprocess: None,
            theme_filter: "all".into(),
            active_mode: None,
            elapsed_secs: 0,
            mirror,
            subprocess_start: None,
        };
        let init_task = Task::perform(
            async { govee_lan::scan_devices(Duration::from_secs(2)) },
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
                self.config.save();
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
                self.config.save();
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
                self.config.save();
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
                    async { govee_lan::scan_devices(Duration::from_secs(2)) },
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
                if let Some(ref mut child) = self.subprocess {
                    crate::subprocess::kill(child);
                    self.subprocess = None;
                }
                let theme = self.themes.iter().find(|t| t.name == name).cloned();
                if let (Some(theme), Some(ref dev)) = (theme, &self.device) {
                    match &theme.kind {
                        govee_lan::ThemeKind::Solid { color } => {
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
                        govee_lan::ThemeKind::Animated { .. } => {
                            let ip = dev.ip.clone();
                            let mut args = vec!["theme", &name];
                            if self.mirror { args.push("--mirror"); }
                            match crate::subprocess::spawn_govee(&args, Some(&ip)) {
                                Ok(child) => {
                                    self.subprocess = Some(child);
                                    self.active_theme = Some(name);
                                    self.subprocess_start = Some(std::time::Instant::now());
                                    self.elapsed_secs = 0;
                                }
                                Err(_) => {}
                            }
                        }
                    }
                }
            }
            Message::StopMode => {
                if let Some(ref mut child) = self.subprocess {
                    crate::subprocess::kill(child);
                }
                self.subprocess = None;
                self.active_theme = None;
                self.active_mode = None;
                self.subprocess_start = None;
                self.elapsed_secs = 0;
            }
            Message::ThemeFilterChanged(filter) => {
                self.theme_filter = filter;
            }
            Message::Tick => {
                if let Some(start) = self.subprocess_start {
                    self.elapsed_secs = start.elapsed().as_secs();
                }
            }
            Message::SetScreenFps(v) => { self.config.screen.fps = v; self.config.save(); }
            Message::SetScreenBrightness(v) => { self.config.screen.brightness = v; self.config.save(); }
            Message::SetScreenSegments(v) => { self.config.screen.segments = v; self.config.save(); }
            Message::ToggleScreenMirror(v) => { self.config.screen.mirror = v; self.config.save(); }
            Message::SetAudioMode(v) => { self.config.audio.mode = v; self.config.save(); }
            Message::SetAudioBrightness(v) => { self.config.audio.brightness = v; self.config.save(); }
            Message::SetAudioSegments(v) => { self.config.audio.segments = v; self.config.save(); }
            Message::ToggleAudioMirror(v) => { self.config.audio.mirror = v; self.config.save(); }
            Message::SetAmbientBrightness(v) => { self.config.ambient.brightness = v; self.config.save(); }
            Message::ToggleAmbientDim(v) => { self.config.ambient.dim = v; self.config.save(); }
            Message::ToggleMirror(v) => {
                self.mirror = v;
                self.config.screen.mirror = v;
                self.config.audio.mirror = v;
                self.config.save();
            }
            Message::StartScreen => {
                if let Some(ref mut child) = self.subprocess {
                    crate::subprocess::kill(child);
                }
                self.subprocess = None;
                self.active_theme = None;
                if let Some(ref dev) = self.device {
                    let s = &self.config.screen;
                    let mut args: Vec<String> = vec![
                        "screen".into(),
                        "--fps".into(), s.fps.to_string(),
                        "--brightness".into(), s.brightness.to_string(),
                        "--segments".into(), s.segments.to_string(),
                    ];
                    if self.mirror { args.push("--mirror".into()); }
                    let ip = dev.ip.clone();
                    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                    match crate::subprocess::spawn_govee(&arg_refs, Some(&ip)) {
                        Ok(child) => {
                            self.subprocess = Some(child);
                            self.active_mode = Some("screen".into());
                            self.subprocess_start = Some(std::time::Instant::now());
                            self.elapsed_secs = 0;
                        }
                        Err(_) => {}
                    }
                }
            }
            Message::StartAudio => {
                if let Some(ref mut child) = self.subprocess {
                    crate::subprocess::kill(child);
                }
                self.subprocess = None;
                self.active_theme = None;
                if let Some(ref dev) = self.device {
                    let a = &self.config.audio;
                    let mut args: Vec<String> = vec![
                        "audio".into(),
                        "--mode".into(), a.mode.clone(),
                        "--brightness".into(), a.brightness.to_string(),
                        "--segments".into(), a.segments.to_string(),
                    ];
                    if self.mirror { args.push("--mirror".into()); }
                    let ip = dev.ip.clone();
                    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                    match crate::subprocess::spawn_govee(&arg_refs, Some(&ip)) {
                        Ok(child) => {
                            self.subprocess = Some(child);
                            self.active_mode = Some("audio".into());
                            self.subprocess_start = Some(std::time::Instant::now());
                            self.elapsed_secs = 0;
                        }
                        Err(_) => {}
                    }
                }
            }
            Message::StartAmbient => {
                if let Some(ref mut child) = self.subprocess {
                    crate::subprocess::kill(child);
                }
                self.subprocess = None;
                self.active_theme = None;
                if let Some(ref dev) = self.device {
                    let amb = &self.config.ambient;
                    let mut args: Vec<String> = vec![
                        "ambient".into(),
                        "--brightness".into(), amb.brightness.to_string(),
                    ];
                    if amb.dim { args.push("--dim".into()); }
                    let ip = dev.ip.clone();
                    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                    match crate::subprocess::spawn_govee(&arg_refs, Some(&ip)) {
                        Ok(child) => {
                            self.subprocess = Some(child);
                            self.active_mode = Some("ambient".into());
                            self.subprocess_start = Some(std::time::Instant::now());
                            self.elapsed_secs = 0;
                        }
                        Err(_) => {}
                    }
                }
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
        };

        let content = container(page_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20);

        let mode_label = if let Some(ref mode) = self.active_mode {
            match mode.as_str() {
                "screen" => format!("Screen Capture · {}s", self.elapsed_secs),
                "audio" => format!("Audio Reactive · {}s", self.elapsed_secs),
                "ambient" => format!("Ambient Sync · {}s", self.elapsed_secs),
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
