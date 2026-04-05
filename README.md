# govee-lan

Control Govee LED strip lights over your local network using the LAN API. No cloud, no API keys, just UDP.

## Features

- **Device discovery** via multicast scan
- **On/off, brightness, RGB color, color temperature** control
- **Preset scenes** (movie, chill, party, sunset, ocean, forest, candlelight, aurora)
- **Ambient mode** — syncs strip color to your desktop wallpaper theme (Caelestia/Material You)

## Requirements

- Python 3.10+
- A Govee LED strip with **LAN API enabled** (Govee Home app > Device > Settings > LAN Control)
- For ambient mode: [`inotify-tools`](https://github.com/inotify-tools/inotify-tools) and [Caelestia](https://github.com/caelestia-dots/) (or any tool that writes a scheme.json)

## Usage

### Basic control

```bash
# Discover devices on your network
./govee.py scan

# Turn on/off
./govee.py on
./govee.py off

# Set brightness (1-100)
./govee.py brightness 60

# Set RGB color
./govee.py color 255 100 0

# Set color temperature (2000-9000K)
./govee.py temp 4000

# Query device status
./govee.py status

# Apply a preset scene
./govee.py scene sunset

# Sleep mode (dark but stays responsive — better than off for quick resume)
./govee.py sleep
```

All commands auto-discover the device. Use `--ip` to target a specific one:

```bash
./govee.py on --ip 192.168.1.42
```

### Ambient wallpaper sync

Continuously syncs the strip color to your desktop's dynamic wallpaper theme:

```bash
# Use primary accent color at 80% brightness (default)
./govee-ambient.py

# Use tertiary color, dimmed variant, at 50%
./govee-ambient.py --color tertiary --dim --brightness 50

# Verbose output to see color changes
./govee-ambient.py -v
```

This watches `~/.local/state/caelestia/scheme.json` for changes and pushes the selected color to the strip in real time.

## How it works

Govee strips with LAN API enabled listen for UDP commands on your local network. The protocol uses:

- **Multicast** (`239.255.255.250:4001`) for device discovery
- **Unicast** (port `4003`) for control commands
- **JSON** payloads for all messages

No authentication, no cloud dependency, no rate limits.

## License

MIT
