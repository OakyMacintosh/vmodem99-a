pkgname=vmodem99a
pkgver=1.0
pkgdesc="The modem-like internet tool."
arch=(any)
url="https://github.com/oakymacintosh/vmodem99-a"
license=('MIT')
depends=('bash' 'curl' 'ssh' 'wget' 'argc' 'telnet')
makedepends=('git' 'rust' 'cargo' 'python')
source=("git+https://github.com/oakymacintosh/vmodem99-a")
sha256sums=('SKIP')

build() {
  cd "$srcdir/vmodem99-a"
  python build.py
}

package() {
  cd "$srcdir/vmodem99-a"
  install -Dm755 dist/vmodem99-a "$pkgdir/usr/bin/vmodem99-a"
  install -Dm644 README.md "$pkgdir/usr/share/doc/vmodem99-a/README.md"
#  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/vmodem99-a/LICENSE"
  
  # Create a symlink for easier access
  ln -s /usr/bin/vmodem99-a "$pkgdir/usr/bin/vmodem99"
}
