"""PTT indicator — outputs JSON for waybar custom module."""

import json
import os
import sys

STATE_FILE = "/tmp/ptt-state"


def main():
    try:
        with open(STATE_FILE) as f:
            state = f.read().strip()
    except FileNotFoundError:
        state = "0"

    if state == "1":
        data = {
            "text": "󰍬",
            "class": "ptt-active",
            "tooltip": "Push-to-Talk: ON\nClick to disable",
        }
    else:
        data = {
            "text": "󰍭",
            "class": "ptt-inactive",
            "tooltip": "Push-to-Talk: OFF\nClick to enable",
        }

    print(json.dumps(data))


if __name__ == "__main__":
    main()
