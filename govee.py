#!/usr/bin/env python3
"""Control Govee LED strip lights over the local network (LAN API).

Usage:
    govee.py scan                        Discover devices on the network
    govee.py on    [--ip IP]             Turn on
    govee.py off   [--ip IP]             Turn off
    govee.py brightness N [--ip IP]      Set brightness (1-100)
    govee.py color R G B [--ip IP]       Set color (0-255 each)
    govee.py temp K [--ip IP]            Set color temperature (2000-9000K)
    govee.py status [--ip IP]            Query device status
    govee.py scene NAME [--ip IP]        Apply a preset scene

Scenes: movie, chill, party, sunset, ocean, forest, candlelight, aurora
"""

import argparse
import json
import socket
import struct
import sys
import time

MULTICAST_GROUP = "239.255.255.250"
SCAN_PORT = 4001
RESPONSE_PORT = 4002
CONTROL_PORT = 4003
SCAN_TIMEOUT = 2.0
COMMAND_TIMEOUT = 2.0

SCENES = {
    "movie":       {"r": 20,  "g": 10,  "b": 40,  "temp": 0},
    "chill":       {"r": 80,  "g": 40,  "b": 120, "temp": 0},
    "party":       {"r": 255, "g": 0,   "b": 200, "temp": 0},
    "sunset":      {"r": 255, "g": 100, "b": 20,  "temp": 0},
    "ocean":       {"r": 0,   "g": 80,  "b": 200, "temp": 0},
    "forest":      {"r": 10,  "g": 120, "b": 30,  "temp": 0},
    "candlelight": {"r": 0,   "g": 0,   "b": 0,   "temp": 3000},
    "aurora":      {"r": 0,   "g": 200, "b": 150, "temp": 0},
}


def make_msg(cmd: str, data: dict) -> bytes:
    return json.dumps({"msg": {"cmd": cmd, "data": data}}).encode()


def scan_devices(timeout: float = SCAN_TIMEOUT) -> list[dict]:
    """Send multicast scan and collect device responses."""
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
    try:
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind(("", RESPONSE_PORT))

        # Join multicast group on all interfaces
        mreq = struct.pack("4s4s", socket.inet_aton(MULTICAST_GROUP), socket.inet_aton("0.0.0.0"))
        sock.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)
        sock.settimeout(timeout)

        # Send scan request
        scan_msg = make_msg("scan", {"account_topic": "reserve"})
        send_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
        try:
            send_sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 2)
            send_sock.sendto(scan_msg, (MULTICAST_GROUP, SCAN_PORT))
        finally:
            send_sock.close()

        devices = []
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                break
            sock.settimeout(remaining)
            try:
                data, _ = sock.recvfrom(4096)
                resp = json.loads(data.decode())
                device_data = resp.get("msg", {}).get("data", {})
                if device_data.get("ip"):
                    devices.append(device_data)
            except socket.timeout:
                break
            except (json.JSONDecodeError, UnicodeDecodeError):
                continue

        return devices
    finally:
        sock.close()


def send_command(ip: str, cmd: str, data: dict, debug: bool = False) -> dict | None:
    """Send a control command to a device and optionally receive a response."""
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind(("", RESPONSE_PORT))
        sock.settimeout(COMMAND_TIMEOUT)
        msg = make_msg(cmd, data)
        if debug:
            print(f"  >> {msg.decode()}", file=sys.stderr)
        sock.sendto(msg, (ip, CONTROL_PORT))

        if cmd == "devStatus":
            try:
                resp_data, _ = sock.recvfrom(4096)
                if debug:
                    print(f"  << {resp_data.decode()}", file=sys.stderr)
                return json.loads(resp_data.decode()).get("msg", {}).get("data", {})
            except (socket.timeout, json.JSONDecodeError, UnicodeDecodeError):
                return None
        return None
    finally:
        sock.close()


def resolve_ip(args_ip: str | None) -> str:
    """Get device IP from argument or auto-discover."""
    if args_ip:
        return args_ip
    print("Scanning for devices...")
    devices = scan_devices()
    if not devices:
        print("No Govee devices found. Make sure LAN API is enabled in the Govee app.", file=sys.stderr)
        sys.exit(1)
    ip = devices[0]["ip"]
    sku = devices[0].get("sku", "unknown")
    print(f"Found {sku} at {ip}")
    return ip


def cmd_scan(args):
    devices = scan_devices()
    if not devices:
        print("No devices found. Ensure LAN API is enabled in the Govee Home app.")
        return
    print(f"Found {len(devices)} device(s):\n")
    for d in devices:
        print(f"  IP:     {d.get('ip')}")
        print(f"  SKU:    {d.get('sku', 'unknown')}")
        print(f"  Device: {d.get('device', 'unknown')}")
        wifi_v = d.get("wifiVersionSoft", "?")
        ble_v = d.get("bleVersionSoft", "?")
        print(f"  WiFi:   {wifi_v}  BLE: {ble_v}")
        print()


def cmd_on(args):
    ip = resolve_ip(args.ip)
    send_command(ip, "turn", {"value": 1}, debug=args.debug)
    print(f"Turned ON ({ip})")


def cmd_off(args):
    ip = resolve_ip(args.ip)
    send_command(ip, "turn", {"value": 0}, debug=args.debug)
    print(f"Turned OFF ({ip})")


def cmd_brightness(args):
    val = args.value
    if not 1 <= val <= 100:
        print("Brightness must be 1-100", file=sys.stderr)
        sys.exit(1)
    ip = resolve_ip(args.ip)
    send_command(ip, "brightness", {"value": val}, debug=args.debug)
    print(f"Brightness set to {val}% ({ip})")


def cmd_color(args):
    r, g, b = args.r, args.g, args.b
    for name, val in [("R", r), ("G", g), ("B", b)]:
        if not 0 <= val <= 255:
            print(f"{name} must be 0-255", file=sys.stderr)
            sys.exit(1)
    ip = resolve_ip(args.ip)
    send_command(ip, "colorwc", {
        "color": {"r": r, "g": g, "b": b},
        "colorTemInKelvin": 0,
    }, debug=args.debug)
    print(f"Color set to ({r}, {g}, {b}) ({ip})")


def cmd_temp(args):
    k = args.kelvin
    if not 2000 <= k <= 9000:
        print("Color temperature must be 2000-9000K", file=sys.stderr)
        sys.exit(1)
    ip = resolve_ip(args.ip)
    send_command(ip, "colorwc", {
        "color": {"r": 0, "g": 0, "b": 0},
        "colorTemInKelvin": k,
    }, debug=args.debug)
    print(f"Color temperature set to {k}K ({ip})")


def cmd_status(args):
    ip = resolve_ip(args.ip)
    status = send_command(ip, "devStatus", {}, debug=args.debug)
    if status is None:
        print(f"No response from {ip}", file=sys.stderr)
        sys.exit(1)

    on_off = "ON" if status.get("onOff") == 1 else "OFF"
    brightness = status.get("brightness", "?")
    color = status.get("color", {})
    temp = status.get("colorTemInKelvin", 0)

    print(f"  Power:       {on_off}")
    print(f"  Brightness:  {brightness}%")
    if temp and temp > 0:
        print(f"  Color Temp:  {temp}K")
    else:
        r, g, b = color.get("r", 0), color.get("g", 0), color.get("b", 0)
        print(f"  Color:       ({r}, {g}, {b})")


def cmd_sleep(args):
    ip = resolve_ip(args.ip)
    send_command(ip, "colorwc", {
        "color": {"r": 0, "g": 0, "b": 0},
        "colorTemInKelvin": 0,
    }, debug=args.debug)
    send_command(ip, "brightness", {"value": 1}, debug=args.debug)
    print(f"Sleep mode (dark but responsive) ({ip})")


def cmd_scene(args):
    name = args.name.lower()
    if name not in SCENES:
        print(f"Unknown scene '{name}'. Available: {', '.join(sorted(SCENES))}", file=sys.stderr)
        sys.exit(1)
    scene = SCENES[name]
    ip = resolve_ip(args.ip)

    # Turn on first
    send_command(ip, "turn", {"value": 1}, debug=args.debug)

    if scene["temp"] > 0:
        send_command(ip, "colorwc", {
            "color": {"r": 0, "g": 0, "b": 0},
            "colorTemInKelvin": scene["temp"],
        }, debug=args.debug)
    else:
        send_command(ip, "colorwc", {
            "color": {"r": scene["r"], "g": scene["g"], "b": scene["b"]},
            "colorTemInKelvin": 0,
        }, debug=args.debug)
    print(f"Scene '{name}' applied ({ip})")


def main():
    parser = argparse.ArgumentParser(
        description="Control Govee LED strip lights over LAN",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="Scenes: " + ", ".join(sorted(SCENES)),
    )
    parser.add_argument("--debug", action="store_true", help="Show raw UDP messages")
    sub = parser.add_subparsers(dest="command", required=True)

    # scan
    sub.add_parser("scan", help="Discover Govee devices on the network")

    # on / off
    p_on = sub.add_parser("on", help="Turn on")
    p_on.add_argument("--ip", help="Device IP (auto-discovers if omitted)")
    p_off = sub.add_parser("off", help="Turn off")
    p_off.add_argument("--ip", help="Device IP (auto-discovers if omitted)")

    # brightness
    p_br = sub.add_parser("brightness", help="Set brightness (1-100)")
    p_br.add_argument("value", type=int, help="Brightness percentage")
    p_br.add_argument("--ip", help="Device IP")

    # color
    p_col = sub.add_parser("color", help="Set RGB color")
    p_col.add_argument("r", type=int, help="Red (0-255)")
    p_col.add_argument("g", type=int, help="Green (0-255)")
    p_col.add_argument("b", type=int, help="Blue (0-255)")
    p_col.add_argument("--ip", help="Device IP")

    # temp
    p_temp = sub.add_parser("temp", help="Set color temperature (2000-9000K)")
    p_temp.add_argument("kelvin", type=int, help="Temperature in Kelvin")
    p_temp.add_argument("--ip", help="Device IP")

    # sleep
    p_sleep = sub.add_parser("sleep", help="Dark but stays responsive (use instead of off)")
    p_sleep.add_argument("--ip", help="Device IP")

    # status
    p_stat = sub.add_parser("status", help="Query device status")
    p_stat.add_argument("--ip", help="Device IP")

    # scene
    p_scene = sub.add_parser("scene", help="Apply a preset scene")
    p_scene.add_argument("name", help="Scene name")
    p_scene.add_argument("--ip", help="Device IP")

    args = parser.parse_args()

    commands = {
        "scan": cmd_scan,
        "on": cmd_on,
        "off": cmd_off,
        "sleep": cmd_sleep,
        "brightness": cmd_brightness,
        "color": cmd_color,
        "temp": cmd_temp,
        "status": cmd_status,
        "scene": cmd_scene,
    }
    commands[args.command](args)


if __name__ == "__main__":
    main()
