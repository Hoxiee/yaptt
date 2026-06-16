# ptt

System-wide push-to-talk for Wayland (PipeWire/PulseAudio).

Uses keyd for dynamic key remapping and evdev for event detection.
No EVIOCGRAB — niri keybinds and all keys work normally when PTT is active.

## Features

- System-wide PTT — works in any application
- keyd-based remapping — no keyboard grab, niri keybinds work
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
- `target/release/ptt-daemon` — main daemon
- `target/release/ptt-toggle` — toggle script
- `target/release/ptt-indicator` — waybar indicator

### Requirements

- keyd system service running
- User in `keyd` group
- Sudoers rule for keyd config updates:
  ```
  hoshi ALL=(root) NOPASSWD: /usr/bin/keyd reload, /usr/bin/tee /etc/keyd/default.conf
  ```

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
ptt-toggle
# or
kill -USR1 $(cat /tmp/ptt-daemon.pid)
```

### State

- **ON**: keyd remaps grave → f13, mic is muted. Hold Tilde to unmute. Waybar shows green indicator.
- **OFF**: grave works normally. Mic is unmuted.

## How it works

1. keyd remaps `grave → f13` when PTT is active
2. Daemon reads events from keyd virtual keyboard
3. On f13 press: `wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 0`
4. On f13 release: `wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 1`
5. SIGUSR1 toggles between active/paused states
6. When paused, keyd restores `grave → grave`

## Binary sizes

- `ptt-daemon`: ~2.2MB
- `ptt-toggle`: ~500KB
- `ptt-indicator`: ~500KB

## License

MIT
