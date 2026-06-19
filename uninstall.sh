#!/usr/bin/env bash
set -euo pipefail

sudo systemctl --user disable --now ptt 2>/dev/null || true
sudo rm -f /usr/bin/yaptt-daemon /usr/bin/yaptt-toggle /usr/bin/yaptt-indicator
sudo rm -f /usr/lib/systemd/user/ptt.service
sudo systemctl --user daemon-reload

echo "Uninstalled"
