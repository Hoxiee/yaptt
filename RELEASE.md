# Release process

## 1. Update version

Edit `daemon/Cargo.toml`:
```toml
version = "X.Y.Z"
```

## 2. Commit and tag

```bash
git add -A
git commit -m "release: vX.Y.Z"
git tag vX.Y.Z
git push origin master --tags
```

## 3. GitHub Actions

The push triggers `.github/workflows/release.yml` which:
- Runs all tests
- Builds release binaries for x86_64-unknown-linux-gnu
- Creates a GitHub Release with `artifacts.tar.gz`

## 4. AUR (optional)

After the GitHub Release is created:

```bash
# Update PKGBUILD
# - pkgver=X.Y.Z
# - sha256sums=('SKIP') or generate with updpkgsums

# Publish to AUR
cd /path/to/yaptt-aur
cp /path/to/yaptt/PKGBUILD .
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "update to X.Y.Z"
git push
```

## Artifacts

Each release produces `artifacts.tar.gz` containing:
- `yaptt-daemon` — main daemon
- `yaptt-toggle` — SIGUSR1 toggle
- `yaptt-indicator` — waybar widget
