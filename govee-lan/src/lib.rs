//! Core library for controlling Govee LED strips over LAN via UDP.
//!
//! Provides device discovery via multicast, the standard JSON command protocol,
//! the DreamView binary segment protocol for per-LED control, color utilities,
//! and optional Wayland screen-capture and PulseAudio audio-analysis modules.

/// JSON and DreamView binary protocol for device communication.
pub mod protocol;
/// Multicast-based device discovery on the LAN.
pub mod discovery;
/// RGB color manipulation utilities.
pub mod colors;

/// Wayland screen capture via `wlr-screencopy-unstable-v1`.
#[cfg(feature = "screen")]
pub mod wayland;
/// Real-time PulseAudio audio analysis with FFT.
#[cfg(feature = "audio")]
pub mod audio;

pub use protocol::{
    send_turn, send_brightness, send_color, send_color_temp, send_command,
    razer_activate, razer_deactivate, send_segments, UdpSender,
    MULTICAST_GROUP, SCAN_PORT, RESPONSE_PORT, CONTROL_PORT,
};
pub use colors::{hex_to_rgb, color_distance, smooth, saturate_color, lerp_color_chain};
pub use discovery::{DeviceInfo, scan_devices, discover_device, resolve_ip};
