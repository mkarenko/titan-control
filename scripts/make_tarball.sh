#!/usr/bin/env sh
# Builds the release archive: dist/titan-control-<version>-<arch>-linux.tar.gz
# The archive has a bin/ + share/ layout, so the program finds its assets next to itself, and install.sh puts it
# into a prefix (default: ~/.local for a user, /usr/local for root).
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT_DIR"

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
ARCH=$(uname -m)
NAME="titan-control-$VERSION-$ARCH-linux"
STAGE="dist/$NAME"

printf '==> Building %s\n' "$NAME"
cargo build --release --locked

rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/share/titan_control" "$STAGE/share/applications" \
    "$STAGE/share/icons/hicolor/scalable/apps" "$STAGE/share/doc/titan-control" \
    "$STAGE/lib/udev/rules.d" "$STAGE/lib/modules-load.d"

install -m755 target/release/titan_control "$STAGE/bin/titan_control"
strip "$STAGE/bin/titan_control" 2>/dev/null || true
cp -r assets "$STAGE/share/titan_control/assets"
install -m644 assets/titan_control.svg "$STAGE/share/icons/hicolor/scalable/apps/titan_control.svg"
install -m644 packaging/titan_control.desktop "$STAGE/share/applications/titan_control.desktop"
install -m644 packaging/60-titan-control-i2c.rules "$STAGE/lib/udev/rules.d/60-titan-control-i2c.rules"
install -m644 packaging/titan-control-modules.conf "$STAGE/lib/modules-load.d/titan-control.conf"
install -m644 README.md FAQ.md "$STAGE/share/doc/titan-control/"
cp -r docs "$STAGE/share/doc/titan-control/docs"
rm -rf "$STAGE/share/doc/titan-control/docs/__pycache__"
install -m644 LICENSE "$STAGE/LICENSE"
install -m755 packaging/install.sh "$STAGE/install.sh"
install -m755 packaging/uninstall.sh "$STAGE/uninstall.sh"

tar -C dist -czf "dist/$NAME.tar.gz" "$NAME"
(cd dist && sha256sum "$NAME.tar.gz" > "$NAME.tar.gz.sha256")
rm -rf "$STAGE"
printf '==> dist/%s.tar.gz\n' "$NAME"
