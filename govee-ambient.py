#!/usr/bin/env python3
"""Sync Govee strip light color with Caelestia's dynamic wallpaper theme.

Watches ~/.local/state/caelestia/scheme.json for changes and pushes
the primary accent color to the Govee strip over LAN.

Usage:
    govee-ambient.py [--ip IP] [--color primary|tertiary|secondary]
                     [--brightness N] [--dim] [--verbose]
"""

import argparse
import json
import os
import signal
import socket
import struct
import subprocess
import sys
import time

SCHEME_PATH = os.path.expanduser("~/.local/state/caelestia/scheme.json")
MULTICAST_GROUP = "239.255.255.250"
SCAN_PORT = 4001
RESPONSE_PORT = 4002
CONTROL_PORT = 4003


def hex_to_rgb(h: str) -> tuple[int, int, int]:
    h = h.lstrip("#")
    return int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)


def make_msg(cmd: str, data: dict) -> bytes:
    return json.dumps({"msg": {"cmd": cmd, "data": data}}).encode()


def discover_device(timeout: float = 2.0) -> str | None:
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
    try:
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind(("", RESPONSE_PORT))
        mreq = struct.pack("4s4s", socket.inet_aton(MULTICAST_GROUP), socket.inet_aton("0.0.0.0"))
        sock.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)
        sock.settimeout(timeout)

        scan_msg = make_msg("scan", {"account_topic": "reserve"})
        send_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
        try:
            send_sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 2)
            send_sock.sendto(scan_msg, (MULTICAST_GROUP, SCAN_PORT))
        finally:
            send_sock.close()

        try:
            data, _ = sock.recvfrom(4096)
            resp = json.loads(data.decode())
            return resp.get("msg", {}).get("data", {}).get("ip")
        except (socket.timeout, json.JSONDecodeError, UnicodeDecodeError):
            return None
    finally:
        sock.close()


def send_color(ip: str, r: int, g: int, b: int):
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        msg = make_msg("colorwc", {
            "color": {"r": r, "g": g, "b": b},
            "colorTemInKelvin": 0,
        })
        sock.sendto(msg, (ip, CONTROL_PORT))
    finally:
        sock.close()


def send_brightness(ip: str, value: int):
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        msg = make_msg("brightness", {"value": value})
        sock.sendto(msg, (ip, CONTROL_PORT))
    finally:
        sock.close()


def read_scheme_color(color_key: str) -> tuple[int, int, int] | None:
    try:
        with open(SCHEME_PATH) as f:
            scheme = json.load(f)
        hex_color = scheme.get("colours", {}).get(color_key)
        if hex_color:
            return hex_to_rgb(hex_color)
    except (json.JSONDecodeError, FileNotFoundError, ValueError):
        pass
    return None


def main():
    parser = argparse.ArgumentParser(
        description="Sync Govee strip with Caelestia wallpaper theme",
    )
    parser.add_argument("--ip", help="Device IP (auto-discovers if omitted)")
    parser.add_argument("--color", default="primary",
                        choices=["primary", "secondary", "tertiary",
                                 "primaryContainer", "tertiaryContainer",
                                 "surfaceTint"],
                        help="Which theme color to use (default: primary)")
    parser.add_argument("--brightness", type=int, default=80,
                        help="Strip brightness 1-100 (default: 80)")
    parser.add_argument("--dim", action="store_true",
                        help="Use the Dim variant of the color (e.g. primaryDim)")
    parser.add_argument("--verbose", "-v", action="store_true")
    args = parser.parse_args()

    # Resolve device IP
    ip = args.ip
    if not ip:
        print("Scanning for Govee device...")
        ip = discover_device()
        if not ip:
            print("No device found. Use --ip or enable LAN API.", file=sys.stderr)
            sys.exit(1)
    print(f"Using device at {ip}")

    # Determine color key
    color_key = args.color
    if args.dim:
        color_key = args.color + "Dim"

    # Set initial brightness
    send_brightness(ip, args.brightness)

    # Apply current color immediately
    rgb = read_scheme_color(color_key)
    if rgb:
        r, g, b = rgb
        send_color(ip, r, g, b)
        if args.verbose:
            print(f"Initial color: ({r}, {g}, {b}) from {color_key}")

    # Watch for scheme changes
    print(f"Watching {SCHEME_PATH} for theme changes (Ctrl+C to stop)")
    print(f"Color key: {color_key} | Brightness: {args.brightness}%")

    last_rgb = rgb

    # Use inotifywait to watch for file changes
    try:
        while True:
            proc = subprocess.run(
                ["inotifywait", "-qq", "-e", "modify,close_write,moved_to",
                 SCHEME_PATH],
                timeout=None,
            )

            # Small delay for file to settle after write
            time.sleep(0.1)

            rgb = read_scheme_color(color_key)
            if rgb and rgb != last_rgb:
                r, g, b = rgb
                send_color(ip, r, g, b)
                last_rgb = rgb
                if args.verbose:
                    print(f"Updated: ({r}, {g}, {b})")

    except KeyboardInterrupt:
        print("\nStopped.")


if __name__ == "__main__":
    main()
