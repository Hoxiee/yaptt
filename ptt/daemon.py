"""PTT daemon — grabs keyboards, intercepts PTT key, toggles mic via wpctl."""

import evdev
from evdev import UInput, ecodes
import glob
import os
import select
import signal
import subprocess
import sys

PTT_KEY = ecodes.KEY_GRAVE
DEVICE_ID_PATH = "/dev/input/by-id"
STATE_FILE = "/tmp/ptt-state"
PID_FILE = "/tmp/ptt-daemon.pid"


def find_keyboards():
    devices = []
    for path in sorted(glob.glob(f"{DEVICE_ID_PATH}/*-event-kbd")):
        try:
            dev = evdev.InputDevice(path)
            caps = dev.capabilities(verbose=False)
            if ecodes.EV_KEY in caps and ecodes.KEY_Q in caps[ecodes.EV_KEY]:
                devices.append(dev)
            else:
                dev.close()
        except Exception:
            pass
    return devices


def make_virtual(device):
    caps = device.capabilities(verbose=False)
    key_caps = caps.get(ecodes.EV_KEY, [])
    return UInput({ecodes.EV_KEY: key_caps}, name=f"PTT-{device.name}")


def write_state(active):
    with open(STATE_FILE, "w") as f:
        f.write("1" if active else "0")


class PTT:
    def __init__(self):
        self.devices = find_keyboards()
        if not self.devices:
            print("No keyboards found", file=sys.stderr)
            sys.exit(1)

        self.pairs = []
        self.active = True
        self.toggle_r, self.toggle_w = os.pipe()

        with open(PID_FILE, "w") as f:
            f.write(str(os.getpid()))

        self.grab()

    def grab(self):
        for dev in self.devices:
            dev.grab()
            virt = make_virtual(dev)
            self.pairs.append((dev, virt))
            print(f"Grabbed: {dev.name} ({dev.path})", file=sys.stderr)
        subprocess.Popen(["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "1"])
        write_state(True)

    def ungrab(self):
        for dev, virt in self.pairs:
            try:
                dev.ungrab()
            except Exception:
                pass
            virt.close()
        self.pairs.clear()
        subprocess.Popen(["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "0"])
        write_state(False)

    def toggle(self):
        if self.active:
            self.ungrab()
            self.active = False
            print("PTT paused", file=sys.stderr)
        else:
            self.grab()
            self.active = True
            print("PTT active", file=sys.stderr)

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
            fds = {dev.fd: (dev, virt) for dev, virt in self.pairs}
            poll = select.poll()
            poll.register(self.toggle_r, select.POLLIN)
            for fd in fds:
                poll.register(fd, select.POLLIN)

            while True:
                for fd, _ in poll.poll():
                    if fd == self.toggle_r:
                        os.read(self.toggle_r, 1)
                        goto_outer = True
                        break
                    dev, virt = fds[fd]
                    for event in dev.read():
                        if event.type == ecodes.EV_KEY and event.code == PTT_KEY:
                            if event.value == 1:
                                subprocess.Popen(["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "0"])
                            elif event.value == 0:
                                subprocess.Popen(["wpctl", "set-mute", "@DEFAULT_AUDIO_SOURCE@", "1"])
                            continue
                        virt.write(event.type, event.code, event.value)
                else:
                    continue
                break

            self.toggle()
