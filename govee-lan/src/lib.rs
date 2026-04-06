pub mod protocol;
pub mod discovery;
pub mod wayland;
pub mod audio;
pub mod colors;
pub mod themes;
pub mod theme_defs;
pub mod theme_loader;

// Public API re-exports
pub use protocol::{
    send_turn, send_brightness, send_color, send_color_temp, send_command,
    razer_activate, razer_deactivate, send_segments,
    MULTICAST_GROUP, SCAN_PORT, RESPONSE_PORT, CONTROL_PORT,
};
pub use colors::{hex_to_rgb, color_distance, smooth, saturate_color};
pub use discovery::{DeviceInfo, scan_devices, discover_device, resolve_ip};
pub use themes::{ThemeDef, ThemeKind, Behavior, Delay, PA, WaveParam, Rgb};
pub use theme_defs::builtin_themes;
pub use theme_loader::load_all_themes;
