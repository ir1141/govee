//! Device discovery via Govee's multicast protocol.
//!
//! Sends a scan packet to `239.255.255.250:4001` and listens on port 4002
//! for JSON responses containing device IP and model SKU.

use anyhow::Result;
use crate::protocol::*;
use serde::Deserialize;
use std::net::{Ipv4Addr, UdpSocket};
use std::time::{Duration, Instant};

/// A discovered Govee device with its IP address and model information.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceInfo {
    pub ip: String,
    #[serde(default)]
    pub sku: String,
    #[serde(default)]
    pub device: String,
    #[serde(default, rename = "wifiVersionSoft")]
    pub wifi_version: String,
    #[serde(default, rename = "bleVersionSoft")]
    pub ble_version: String,
}

/// Scan the LAN for Govee devices, returning all found within the timeout.
pub fn scan_devices(timeout: Duration) -> Vec<DeviceInfo> {
    let mut devices = Vec::new();

    let recv_sock = match (|| -> std::io::Result<UdpSocket> {
        let sock = socket2::Socket::new(socket2::Domain::IPV4, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))?;
        sock.set_reuse_address(true)?;
        sock.bind(&socket2::SockAddr::from(std::net::SocketAddrV4::new(
            std::net::Ipv4Addr::UNSPECIFIED,
            RESPONSE_PORT,
        )))?;
        Ok(sock.into())
    })() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("govee: failed to bind discovery port {RESPONSE_PORT}: {e}");
            return devices;
        }
    };

    let multicast: Ipv4Addr = MULTICAST_GROUP.parse().unwrap();
    // Best-effort join — discovery still works via broadcast if multicast join fails.
    recv_sock.join_multicast_v4(&multicast, &Ipv4Addr::UNSPECIFIED).ok();
    recv_sock.set_read_timeout(Some(timeout)).ok();

    // Send scan on a separate socket
    let scan_msg = make_msg("scan", serde_json::json!({"account_topic": "reserve"}));
    if let Ok(send_sock) = UdpSocket::bind("0.0.0.0:0") {
        // TTL of 2 allows the scan to reach devices one router hop away.
        send_sock.set_multicast_ttl_v4(2).ok();
        send_sock.send_to(&scan_msg, (MULTICAST_GROUP, SCAN_PORT)).ok();
    }

    let deadline = Instant::now() + timeout;
    let mut buf = [0u8; 4096];
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        recv_sock.set_read_timeout(Some(remaining)).ok();
        match recv_sock.recv_from(&mut buf) {
            Ok((n, _)) => {
                if let Ok(text) = std::str::from_utf8(&buf[..n]) {
                    if let Ok(resp) = serde_json::from_str::<GoveeMsg>(text) {
                        if let Ok(info) = serde_json::from_value::<DeviceInfo>(resp.msg.data) {
                            if !info.ip.is_empty()
                                && info.ip.parse::<Ipv4Addr>().is_ok()
                                && !devices.iter().any(|d| d.ip == info.ip)
                            {
                                devices.push(info);
                            }
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }

    devices
}

/// Discover a single device, returning the first responder's IP.
pub fn discover_device(timeout: Duration) -> Option<String> {
    scan_devices(timeout).into_iter().next().map(|d| d.ip)
}

/// Returns the given IP if provided, otherwise auto-discovers a device.
pub fn resolve_ip(ip: Option<&str>, timeout: Duration) -> Result<String> {
    if let Some(ip) = ip {
        return Ok(ip.to_string());
    }
    match discover_device(timeout) {
        Some(ip) => Ok(ip),
        None => anyhow::bail!("No Govee devices found"),
    }
}
