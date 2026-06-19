# Maintainer: Hoxiee
pkgname=yaptt-bin
pkgver=0.1.0
pkgrel=1
pkgdesc="System-wide push-to-talk for Wayland"
arch=('x86_64')
url="https://github.com/Hoxiee/yaptt"
license=('MIT')
depends=('evdev' 'libinput' 'pipewire' 'lib32-glibc')
makedepends=('cargo')
source=("$url/releases/download/v$pkgver/artifacts.tar.gz")
sha256sums=('SKIP')

package() {
    install -Dm755 "$srcdir/yaptt-daemon" "$pkgdir/usr/bin/yaptt-daemon"
    install -Dm755 "$srcdir/yaptt-toggle" "$pkgdir/usr/bin/yaptt-toggle"
    install -Dm755 "$srcdir/yaptt-indicator" "$pkgdir/usr/bin/yaptt-indicator"
    install -Dm644 "$srcdir/../../systemd/ptt.service" "$pkgdir/usr/lib/systemd/user/ptt.service"
}

check() {
    cd "$srcdir"
    echo "Binaries:"
    ./yaptt-daemon --version 2>/dev/null || true
    ./yaptt-toggle --version 2>/dev/null || true
    ./yaptt-indicator --version 2>/dev/null || true
}
