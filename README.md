# yaptt

System-wide push-to-talk for Wayland.

Grabs your microphone at the OS level — hold a key to talk, release to mute. Works in every application (Discord, browser, Telegram, etc.) without per-app configuration.

## How it works

```
Physical keyboard ──EVIOCGRAB──▸ PTT daemon ──uinput──▸ Your compositor (niri/hypr/…)
                                       │
                                       ├─ grave pressed  → wpctl unmute mic
                                       └─ grave released → fade-out → wpctl mute mic
```

1. Daemon grabs all physical keyboards via `EVIOCGRAB`
2. Creates a uinput virtual keyboard and forwards all events
3. When PTT is **active**: the PTT key (grave) is remapped to F13, mic is muted
4. **Hold grave** → mic unmutes, F13 is forwarded to apps
5. **Release grave** → smooth volume fade-out, then mic mutes
6. SIGUSR1 toggles PTT on/off

## Install

```bash
cargo build --release
```

Binaries in `target/release/`:
| Binary | Purpose |
|---|---|
| `yaptt-daemon` | Main daemon |
| `yaptt-toggle` | Toggle PTT on/off |
| `yaptt-indicator` | Waybar status widget |

### System requirements

- Linux with PipeWire or PulseAudio
- User in the `input` group: `sudo usermod -aG input $USER`
- `wpctl` (PipeWire) or `pactl` (PulseAudio) in PATH

## Usage

### Start the daemon

```bash
yaptt-daemon
# or with debug logging:
RUST_LOG=debug yaptt-daemon
```

### Toggle PTT

```bash
yaptt-toggle
# or directly:
kill -USR1 $(cat /tmp/ptt-daemon.pid)
```

### States

| State | Mic | Key behavior | Waybar |
|---|---|---|---|
| **OFF** | Unmuted | grave works normally | Grey icon |
| **ON** | Muted | Hold grave to talk | Yellow icon |
| **TALKING** | Unmuted | F13 forwarded to apps | Green icon |

## Configuration

Edit `~/.config/yaptt/config.json`:

```json
{
  "ptt_key": "grave",
  "remap_key": "f13",
  "fade_duration_ms": 350
}
```

| Key | Default | Description |
|---|---|---|
| `ptt_key` | `"grave"` | Key to hold for push-to-talk |
| `remap_key` | `"f13"` | Virtual key forwarded while holding PTT key |
| `fade_duration_ms` | `35` | Volume fade-out time in milliseconds |

### Supported key names

Letters: `a`–`z`, Numbers: `0`–`9`, Function: `f1`–`f24`, Modifiers: `leftctrl`, `rightctrl`, `leftshift`, `rightshift`, `leftalt`, `rightalt`, `leftmeta`, `rightmeta`, Special: `grave`, `esc`, `tab`, `capslock`, `space`, `enter`, `backspace`

## Systemd

```bash
cp systemd/ptt.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now ptt
```

Edit the `ExecStart` path in the service file to match your build location.

## Waybar

Add to your waybar config:

```jsonc
// In your bar's "modules-left" or "modules-right":
"custom/ptt"

// Module config (waybar/ptt.jsonc):
"custom/ptt": {
    "format": "{}",
    "exec": "/path/to/yaptt-indicator",
    "return-type": "json",
    "on-click": "/path/to/yaptt-toggle",
    "interval": 1,
    "tooltip": true
}
```

Add the CSS from `waybar/ptt.css` to your style:

```css
#custom-ptt { margin: 4px 5px; padding: 6px 15px 6px 10px; border-radius: 20px; }
#custom-ptt.ptt-off { color: #9399b2; }
#custom-ptt.ptt-on { color: #f9e2af; }
#custom-ptt.ptt-talking { color: #a6e3a1; }
```

## Project structure

```
daemon/
  src/
    lib.rs             — Config, state, key mapping, wpctl helpers, device discovery
    main.rs            — Daemon: EVIOCGRAB, uinput, fade loop, signal handling
    bin/
      ptt-toggle.rs    — SIGUSR1 toggle with desktop notification
      ptt-indicator.rs — Waybar JSON output (OFF/ON/TALKING)
  tests/               — 78 tests covering config, keys, state, devices, events
systemd/
  ptt.service          — Systemd user service
waybar/
  ptt.jsonc            — Waybar module config
```

## License

MIT
