# ptt

System-wide push-to-talk for Wayland (PipeWire/PulseAudio).

Uses EVIOCGRAB + uinput for keyboard event interception. No external key remapping daemon needed.

## Features

- System-wide PTT — works in any application
- EVIOCGRAB + uinput — grabs physical keyboard, creates virtual keyboard
- niri keybinds work normally via the virtual keyboard
- PipeWire/PulseAudio support via `wpctl`
- Waybar integration with toggle indicator
- SIGUSR1-based toggle (on/off)
- Systemd user service
- Desktop notifications via `notify-send`

## Installation

### From source (Rust)

```bash
cargo build --release
```

Binaries:
- `target/release/yaptt-daemon` — main daemon
- `target/release/yaptt-toggle` — toggle script
- `target/release/yaptt-indicator` — waybar indicator

### Requirements

- User in the `input` group (for `/dev/uinput` access):
  ```bash
  sudo usermod -aG input $USER
  ```
- No keyd dependency required

### Systemd service

```bash
cp systemd/ptt.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now ptt
```

### Waybar

Copy `waybar/ptt.jsonc` to your waybar modules directory and add `"custom/ptt"` to your
bar layout. Copy `waybar/ptt.css` imports to your style.

## Usage

### Toggle PTT

```bash
yaptt-toggle
# or
kill -USR1 $(cat /tmp/ptt-daemon.pid)
```

### State

- **ON**: grave key is remapped to f13 via uinput, mic is muted. Hold Tilde to unmute. Waybar shows green indicator.
- **OFF**: grave works normally. Mic is unmuted.

## How it works

1. Daemon grabs all physical keyboards with EVIOCGRAB
2. Creates a uinput virtual keyboard
3. Forwards all events from physical → virtual (so niri sees them)
4. When PTT active: grave → f13 in forwarding, mic is muted
5. On f13 (grave) press: `wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 0`
6. On f13 (grave) release: `wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 1`
7. SIGUSR1 toggles between active/paused states
8. When paused, grave forwards as-is (no remap)

## Binary sizes

- `yaptt-daemon`: ~2.5MB
- `yaptt-toggle`: ~500KB
- `yaptt-indicator`: ~500KB

## License

MIT
