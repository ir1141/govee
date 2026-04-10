//! Govee LAN protocol implementation.
//!
//! Standard commands use JSON over UDP unicast to port 4003. The DreamView
//! (Razer) sub-protocol sends binary packets — base64-encoded and wrapped in
//! JSON — for per-segment color control.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::net::UdpSocket;

/// Multicast group for device discovery.
pub const MULTICAST_GROUP: &str = "239.255.255.250";
/// Port for sending scan requests.
pub const SCAN_PORT: u16 = 4001;
/// Port for receiving scan responses.
pub const RESPONSE_PORT: u16 = 4002;
/// Port for sending commands to devices.
pub const CONTROL_PORT: u16 = 4003;

/// Top-level JSON envelope for all Govee LAN messages.
#[derive(Debug, Serialize, Deserialize)]
pub struct GoveeMsg {
    pub msg: MsgInner,
}

/// Inner message containing the command name and its JSON payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct MsgInner {
    pub cmd: String,
    pub data: serde_json::Value,
}

/// Build a JSON command packet ready to send over UDP.
pub fn make_msg(cmd: &str, data: serde_json::Value) -> Vec<u8> {
    let msg = GoveeMsg {
        msg: MsgInner {
            cmd: cmd.to_string(),
            data,
        },
    };
    serde_json::to_vec(&msg).expect("JSON serialization failed")
}

/// Send a raw message to a device via a one-shot UDP socket.
pub fn udp_send(ip: &str, msg: &[u8]) -> std::io::Result<()> {
    let addr = parse_device_addr(ip)?;
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.send_to(msg, addr)?;
    Ok(())
}

fn parse_device_addr(ip: &str) -> std::io::Result<std::net::SocketAddr> {
    let addr: std::net::Ipv4Addr = ip.parse().map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid IP: {e}"))
    })?;
    if addr.is_broadcast() || addr.is_multicast() || addr.is_unspecified() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "IP must be a unicast address",
        ));
    }
    Ok(std::net::SocketAddr::from((addr, CONTROL_PORT)))
}

/// A reusable UDP sender that keeps a socket open across calls.
/// Use this in hot loops (screen capture, audio, themes) to avoid
/// creating a new socket per frame.
pub struct UdpSender {
    sock: UdpSocket,
    addr: std::net::SocketAddr,
}

impl UdpSender {
    /// Create a sender bound to the given device IP.
    pub fn new(ip: &str) -> std::io::Result<Self> {
        let addr = parse_device_addr(ip)?;
        let sock = UdpSocket::bind("0.0.0.0:0")?;
        Ok(Self { sock, addr })
    }

    /// Send a raw message to the device.
    pub fn send(&self, msg: &[u8]) -> std::io::Result<()> {
        self.sock.send_to(msg, self.addr)?;
        Ok(())
    }

    /// Send per-segment colors via the DreamView binary protocol.
    pub fn send_segments(&self, colors: &[(u8, u8, u8)], gradient: bool) -> std::io::Result<()> {
        if colors.len() > 255 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "too many segments (max 255)"));
        }
        let mut color_data: Vec<u8> = vec![if gradient { 1 } else { 0 }, colors.len() as u8];
        for &(r, g, b) in colors {
            color_data.extend_from_slice(&[r, g, b]);
        }
        let data_len = color_data.len();
        // DreamView packet: 0xBB = start marker, 2-byte big-endian payload length,
        // 0xB0 = segment-color command, then color data, then XOR checksum.
        let mut packet = vec![0xBB, (data_len >> 8) as u8, data_len as u8, 0xB0];
        packet.extend_from_slice(&color_data);
        packet.push(xor_checksum(&packet));
        self.send(&razer_msg(&packet))
    }

    /// Send a single color to the entire strip (non-DreamView).
    pub fn send_color(&self, r: u8, g: u8, b: u8) -> std::io::Result<()> {
        let msg = make_msg(
            "colorwc",
            serde_json::json!({
                "color": {"r": r, "g": g, "b": b},
                "colorTemInKelvin": 0,
            }),
        );
        self.send(&msg)
    }
}

/// Turn the device on or off.
pub fn send_turn(ip: &str, on: bool) -> std::io::Result<()> {
    let msg = make_msg("turn", serde_json::json!({"value": if on { 1 } else { 0 }}));
    udp_send(ip, &msg)
}

/// Set strip brightness (1-100).
pub fn send_brightness(ip: &str, value: u8) -> std::io::Result<()> {
    let msg = make_msg("brightness", serde_json::json!({"value": value}));
    udp_send(ip, &msg)
}

/// Set the strip to a single RGB color.
pub fn send_color(ip: &str, r: u8, g: u8, b: u8) -> std::io::Result<()> {
    let msg = make_msg(
        "colorwc",
        serde_json::json!({
            "color": {"r": r, "g": g, "b": b},
            "colorTemInKelvin": 0,
        }),
    );
    udp_send(ip, &msg)
}

/// Set the strip to a white color temperature in Kelvin.
pub fn send_color_temp(ip: &str, kelvin: u16) -> std::io::Result<()> {
    let msg = make_msg(
        "colorwc",
        serde_json::json!({
            "color": {"r": 0, "g": 0, "b": 0},
            "colorTemInKelvin": kelvin,
        }),
    );
    udp_send(ip, &msg)
}

/// Send an arbitrary command and optionally read a response (only `devStatus` returns data).
pub fn send_command(ip: &str, cmd: &str, data: serde_json::Value, debug: bool) -> Option<serde_json::Value> {
    match send_command_inner(ip, cmd, data, debug) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Command failed: {e}");
            None
        }
    }
}

fn send_command_inner(ip: &str, cmd: &str, data: serde_json::Value, debug: bool) -> std::io::Result<Option<serde_json::Value>> {
    let msg = make_msg(cmd, data);
    if debug {
        eprintln!("  >> {}", String::from_utf8_lossy(&msg));
    }

    let sock = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))?;
    sock.set_reuse_address(true)?;
    sock.bind(&socket2::SockAddr::from(std::net::SocketAddrV4::new(std::net::Ipv4Addr::UNSPECIFIED, RESPONSE_PORT)))?;
    sock.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    let sock: UdpSocket = sock.into();
    sock.send_to(&msg, (ip, CONTROL_PORT))?;

    if cmd == "devStatus" {
        let mut buf = [0u8; 4096];
        match sock.recv_from(&mut buf) {
            Ok((n, _)) => {
                let resp_str = std::str::from_utf8(&buf[..n])
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                if debug {
                    eprintln!("  << {}", resp_str);
                }
                let resp: GoveeMsg = serde_json::from_str(resp_str)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(Some(resp.msg.data))
            }
            Err(e) => Err(e),
        }
    } else {
        Ok(None)
    }
}

fn xor_checksum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc ^ b)
}

fn razer_msg(payload: &[u8]) -> Vec<u8> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(payload);
    make_msg("razer", serde_json::json!({"pt": b64}))
}

/// Enable DreamView mode on the device. Must be called before `send_segments`.
///
/// Sends a mode-switch command: `0xB1` = mode control, `0x01` = enable.
pub fn razer_activate(ip: &str) -> std::io::Result<()> {
    let mut packet = vec![0xBB, 0x00, 0x01, 0xB1, 0x01];
    packet.push(xor_checksum(&packet));
    udp_send(ip, &razer_msg(&packet))
}

/// Disable DreamView mode, returning the device to normal operation.
pub fn razer_deactivate(ip: &str) -> std::io::Result<()> {
    let mut packet = vec![0xBB, 0x00, 0x01, 0xB1, 0x00];
    packet.push(xor_checksum(&packet));
    udp_send(ip, &razer_msg(&packet))
}

/// Send per-segment colors via DreamView, creating a fresh socket per call.
///
/// Use [`UdpSender::send_segments`] in hot loops to avoid per-call socket overhead.
pub fn send_segments(ip: &str, colors: &[(u8, u8, u8)], gradient: bool) -> std::io::Result<()> {
    if colors.len() > 255 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "too many segments (max 255)"));
    }
    let mut color_data: Vec<u8> = vec![if gradient { 1 } else { 0 }, colors.len() as u8];
    for &(r, g, b) in colors {
        color_data.extend_from_slice(&[r, g, b]);
    }
    let data_len = color_data.len();
    let mut packet = vec![0xBB, (data_len >> 8) as u8, data_len as u8, 0xB0];
    packet.extend_from_slice(&color_data);
    packet.push(xor_checksum(&packet));
    udp_send(ip, &razer_msg(&packet))
}


