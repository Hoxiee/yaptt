# yappt - System-wide push-to-talk for Wayland

# Show available recipes
default:
    @just --list

# Show current version from all sources
version:
    #!/usr/bin/env bash
    set -euo pipefail
    daemon=$(grep '^version' daemon/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    pkg=$(grep '^pkgver=' PKGBUILD | sed 's/pkgver=//')
    gui=$(grep '^version' gui/src-tauri/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    echo "daemon/Cargo.toml:      $daemon"
    echo "PKGBUILD:               $pkg"
    echo "gui/src-tauri/Cargo.toml: $gui"

# Update version in all sources
newver:
    #!/usr/bin/env bash
    set -euo pipefail
    current=$(grep '^version' daemon/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    echo "Current version: $current"
    read -rp "New version: " new
    [[ -z "$new" ]] && { echo "Aborted."; exit 1; }
    first=$(printf '%s\n%s\n' "$current" "$new" | sort -V | head -1)
    if [[ "$first" == "$new" && "$new" != "$current" ]]; then
        read -rp "WARNING: $new is LOWER than $current. Proceed? [y/N] " confirm
        [[ "$confirm" =~ ^[Yy]$ ]] || { echo "Aborted."; exit 1; }
    elif [[ "$new" == "$current" ]]; then
        echo "Version unchanged ($current)"
    fi
    sed -i "0,/^version = .*/s//version = \"$new\"/" daemon/Cargo.toml
    sed -i "0,/^version = .*/s//version = \"$new\"/" gui/src-tauri/Cargo.toml
    sed -i "s/^pkgver=.*/pkgver=$new/" PKGBUILD
    sed -i "s/^pkgrel=.*/pkgrel=1/" PKGBUILD
    echo "Updated all -> $new"

# Run tests
test:
    cargo test --release --manifest-path daemon/Cargo.toml

# Build release binaries
build:
    cargo build --release --manifest-path daemon/Cargo.toml

# Full release: test, build, tag, push
release: test build
    #!/usr/bin/env bash
    set -euo pipefail
    ver=$(grep '^version' daemon/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    tag="v$ver"
    echo "Release $tag"
    read -rp "Proceed? [y/N] " confirm
    [[ "$confirm" =~ ^[Yy]$ ]] || { echo "Aborted."; exit 1; }
    git add -A
    git diff --cached --quiet || git commit -m "release: v$ver"
    git tag "$tag"
    git push origin master --tags
    echo "Pushed $tag — CI will build and create GitHub Release"
