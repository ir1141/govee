pub mod protocol;
pub mod discovery;
pub mod wayland;
pub mod audio;

// Public API re-exports
pub use protocol::{
    send_turn, send_brightness, send_color, send_color_temp, send_command,
    razer_activate, razer_deactivate, send_segments,
    hex_to_rgb, color_distance, smooth, saturate_color,
    MULTICAST_GROUP, SCAN_PORT, RESPONSE_PORT, CONTROL_PORT,
};
pub use discovery::{DeviceInfo, scan_devices, discover_device, resolve_ip};
