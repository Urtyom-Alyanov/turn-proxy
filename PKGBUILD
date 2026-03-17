# Maintainer: Urtyom Alyanov <urtyomalynov@gmail.com>
pkgname=turn-proxy-server-rs
pkgver=1.0.6
pkgrel=1
pkgdesc="DTLS proxy server for TURN-based video call traffic relay"
arch=('x86_64' 'aarch64')
url="https://github.com/Urtyom-Alyanov/turn-proxy-server"
license=('AGPL3')
depends=('gcc-libs')
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz::https://github.com/Urtyom-Alyanov/turn-proxy-server/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
    cd "turn-proxy-$pkgver"
    cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
    cd "turn-proxy-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    cargo build --frozen --release --all-features
}

check() {
    cd "turn-proxy-$pkgver"
    cargo test --frozen
}

package() {
    cd "turn-proxy-$pkgver"

    # Установка бинарника
    install -Dm755 "target/release/turn-proxy-server" "$pkgdir/usr/bin/turn-proxy-server"

    # Установка конфига по умолчанию
    install -Dm644 "example.toml" "$pkgdir/etc/turn-proxy/server/config.toml"

    if [ -f "services/systemd/turn-proxy.service" ]; then
        install -Dm644 "services/systemd/turn-proxy.service" "$pkgdir/usr/lib/systemd/system/turn-proxy.service"
    fi

    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}