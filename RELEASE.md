# Release process

## Quick release

```bash
# 1. Update version in daemon/Cargo.toml
# 2. Push tag:
git tag vX.Y.Z
git push origin master --tags
```

Everything else is automated:

- **GitHub Actions** builds binaries, creates Release, updates PKGBUILD version
- **AUR** requires manual push (see below)

## What happens automatically

1. Tag push triggers `.github/workflows/release.yml`
2. Tests run, binaries built, `artifacts.tar.gz` packaged
3. PKGBUILD `pkgver` updated and committed back to repo
4. GitHub Release created with artifacts

## AUR push (manual)

After the GitHub Release is created:

```bash
# Clone AUR repo (first time only)
git clone ssh://aur@aur.archlinux.org/yaptt-bin.git

# Update and push
cd yaptt-bin
cp /path/to/yaptt/PKGBUILD .
updpkgsums          # or set sha256sums=('SKIP')
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "update to X.Y.Z"
git push
```
