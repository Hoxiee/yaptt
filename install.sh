#!/usr/bin/env bash
set -euo pipefail

REPO="Hoxiee/yaptt"
VERSION="${1:-latest}"

if [[ "$VERSION" == "latest" ]]; then
    VERSION=$(curl -sL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | sed 's/.*"v\(.*\)".*/\1/')
fi

echo "Installing yappt v$VERSION..."
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

curl -sL "https://github.com/$REPO/releases/download/v$VERSION/artifacts.tar.gz" | tar xz -C "$tmp"

sudo cp "$tmp/yaptt-daemon" "$tmp/yaptt-toggle" "$tmp/yaptt-indicator" /usr/bin/
sudo curl -sL "https://raw.githubusercontent.com/$REPO/master/systemd/ptt.service" -o /usr/lib/systemd/user/ptt.service
sudo systemctl --user daemon-reload

echo "Installed v$VERSION"
echo "Run: systemctl --user enable --now ptt"
