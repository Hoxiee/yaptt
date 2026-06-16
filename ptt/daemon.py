"""PTT daemon — reads keyd virtual keyboard events, detects PTT key, toggles mic.

Uses keyd for dynamic key remapping:
  PTT ON:  grave → f13  (daemon detects f13, toggles mic)
  PTT OFF: grave → grave (normal behavior)

No EVIOCGRAB — niri keybinds and all keys work normally.
"""

import evdev
from evdev import ecodes
import os
import select
import signal
import subprocess
import sys

STATE_FILE = "/tmp/ptt-state"
PID_FILE = "/tmp/ptt-daemon.pid"
KEYD_CONFIG = "/etc/keyd/default.conf"


def remap_grave(target):
    try:
        config = f"[ids]\n\n*\n\n[main]\n\ngrave = {target}\n"
        subprocess.run(
            ["sudo", "-n", "tee", KEYD_CONFIG],
            input=config.encode(), capture_output=True, timeout=5)
        subprocess.run(["sudo", "-n", "keyd", "reload"],
                       capture_output=True, timeout=5)
    except Exception as e:
        print(f"keyd remap failed: {e}", file=sys.stderr)


def write_state(active):
    with open(STATE_FILE, "w") as f:
        f.write("1" if active else "0")


def find_keyd_keyboard():
    for path in evdev.list_devices():
        try:
            dev = evdev.InputDevice(path)
            if "keyd virtual keyboard" in dev.name:
                return dev
            dev.close()
        except Exception:
            pass
    return None


class PTT:
    def __init__(self):
        self.device = find_keyd_keyboard()
        if not self.device:
            print("keyd virtual keyboard not found", file=sys.stderr)
            sys.exit(1)

        print(f"Using: {self.device.name} ({self.device.path})", file=sys.stderr)

        self.active = True
        self.toggle_r, self.toggle_w = os.pipe()

        with open(PID_FILE, "w") as f:
            f.write(str(os.getpid()))

        self.grab()

    def grab(self):
        remap_grave("f13")
        subprocess.Popen(["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "1"])
        write_state(True)
        print("PTT active", file=sys.stderr)

    def ungrab(self):
        remap_grave("grave")
        subprocess.Popen(["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "0"])
        write_state(False)
        print("PTT paused", file=sys.stderr)

    def toggle(self):
        if self.active:
            self.ungrab()
            self.active = False
        else:
            self.grab()
            self.active = True

    def cleanup(self):
        if self.active:
            self.ungrab()
        try:
            os.unlink(PID_FILE)
        except Exception:
            pass
        os.close(self.toggle_r)
        os.close(self.toggle_w)
        sys.exit(0)

    def run(self):
        signal.signal(signal.SIGUSR1, lambda s, f: os.write(self.toggle_w, b"x"))
        signal.signal(signal.SIGTERM, lambda s, f: self.cleanup())
        signal.signal(signal.SIGINT, lambda s, f: self.cleanup())

        while True:
            fds = {self.device.fd: self.device, self.toggle_r: "toggle"}
            poll = select.poll()
            for fd in fds:
                poll.register(fd, select.POLLIN)

            while True:
                for fd, _ in poll.poll():
                    if fd == self.toggle_r:
                        os.read(self.toggle_r, 1)
                        goto_outer = True
                        break
                    for event in self.device.read():
                        if event.type != ecodes.EV_KEY:
                            continue
                        if event.code == ecodes.KEY_F13:
                            if event.value == 1:
                                subprocess.Popen(["wpctl", "set-mute",
                                                  "@DEFAULT_AUDIO_SOURCE@", "0"])
                            elif event.value == 0:
                                subprocess.Popen(["wpctl", "set-mute",
                                                  "@DEFAULT_AUDIO_SOURCE@", "1"])
                else:
                    continue
                break

            self.toggle()
