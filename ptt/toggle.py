"""PTT toggle — sends SIGUSR1 to the daemon to switch modes."""

import os
import signal
import subprocess
import sys

PID_FILE = "/tmp/ptt-daemon.pid"
STATE_FILE = "/tmp/ptt-state"


def notify(title, body, icon):
    subprocess.Popen([
        "notify-send", "-a", "ptt", "-i", icon, "-t", "2000", title, body
    ])


def main():
    try:
        with open(PID_FILE) as f:
            pid = int(f.read().strip())
    except FileNotFoundError:
        print("PTT daemon is not running", file=sys.stderr)
        sys.exit(1)
    except ProcessLookupError:
        print("PTT daemon PID is stale, removing", file=sys.stderr)
        os.unlink(PID_FILE)
        sys.exit(1)
    except ValueError:
        print("Invalid PID file", file=sys.stderr)
        sys.exit(1)

    try:
        with open(STATE_FILE) as f:
            state = f.read().strip()
    except FileNotFoundError:
        state = "0"

    os.kill(pid, signal.SIGUSR1)

    if state == "0":
        notify("PTT ON", "Hold Tilde to talk", "microphone-sensitivity-high")
    else:
        notify("PTT OFF", "Tilde works normally", "microphone-sensitivity-muted")


if __name__ == "__main__":
    main()
