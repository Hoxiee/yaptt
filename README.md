# ptt

System-wide push-to-talk for Wayland (PipeWire/PulseAudio).

Grabs the keyboard at the evdev level, intercepts a configurable key (default: Tilde/Grave),
and toggles microphone mute via `wpctl`. All other keys are forwarded transparently via
uinput virtual keyboards.

## Features

- System-wide PTT — works in any application
- Grab-based key blocking — PTT key produces no output in applications
- PipeWire/PulseAudio support via `wpctl`
- Waybar integration with toggle indicator
- SIGUSR1-based toggle (on/off)
- Systemd user service

## Installation

```bash
pip install .
```

Or from source:

```bash
git clone https://github.com/user/ptt
cd ptt
pip install .
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

Toggle is also available via waybar click on the PTT module.

### State

- **ON**: Mic is muted by default. Hold Tilde to unmute. Waybar shows green indicator.
- **OFF**: Tilde works normally. Mic is unmuted.

### Limitations

When PTT is active, the keyboard is grabbed at the evdev level (`EVIOCGRAB`).
This means **all** key events are blocked from reaching the compositor — including
window manager keybinds like Mod+M. Toggle is only available via waybar click.

## How it works

1. Scans `/dev/input/by-id/*-event-kbd` for keyboard devices
2. Grabs each keyboard exclusively via `EVIOCGRAB`
3. Creates uinput virtual keyboards for each physical device
4. Forwards all events except the PTT key to virtual keyboards
5. On PTT key press: `wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 0`
6. On PTT key release: `wpctl set-mute @DEFAULT_AUDIO_SOURCE@ 1`
7. SIGUSR1 toggles between active/paused states

## Requirements

- Linux with evdev support
- PipeWire or PulseAudio
- Python 3.10+
- `evdev` Python package
- User must be in `input` group

## License

MIT
