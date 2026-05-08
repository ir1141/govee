//! Core library for controlling Govee LED strips over LAN via UDP.
//!
//! Provides device discovery via multicast, the standard JSON command protocol,
//! the DreamView binary segment protocol for per-LED control, color utilities,
//! and optional Wayland screen-capture and PulseAudio audio-analysis modules.

/// RGB color manipulation utilities.
pub mod colors;
/// Multicast-based device discovery on the LAN.
pub mod discovery;
/// JSON and DreamView binary protocol for device communication.
pub mod protocol;

/// Real-time PulseAudio audio analysis with FFT.
#[cfg(feature = "audio")]
pub mod audio;
/// Wayland screen capture via `wlr-screencopy-unstable-v1`.
#[cfg(feature = "screen")]
pub mod wayland;

pub use colors::{color_distance, hex_to_rgb, lerp_color_chain, saturate_color, smooth};
pub use discovery::{discover_device, resolve_ip, scan_devices, DeviceInfo};
pub use protocol::{
    razer_activate, razer_deactivate, send_brightness, send_color, send_color_temp, send_command,
    send_segments, send_turn, UdpSender, CONTROL_PORT, MULTICAST_GROUP, RESPONSE_PORT, SCAN_PORT,
};
