#!/usr/bin/env sh
set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PREFIX=${PREFIX:-"$HOME/.local"}
BIN_DIR="$PREFIX/bin"
APP_SHARE_DIR="$PREFIX/share/titan_control"
DESKTOP_DIR="$PREFIX/share/applications"
ICON_DIR="$PREFIX/share/icons/hicolor/scalable/apps"

printf '==> Building release binary\n'
cargo build --release --manifest-path "$ROOT_DIR/Cargo.toml"

printf '==> Installing binary and assets into %s\n' "$PREFIX"
install -Dm755 "$ROOT_DIR/target/release/titan_control" "$BIN_DIR/titan_control"
install -Dm644 "$ROOT_DIR/assets/titan_control.svg" "$APP_SHARE_DIR/assets/titan_control.svg"
install -Dm644 "$ROOT_DIR/assets/titan_control.svg" "$ICON_DIR/titan_control.svg"
# Names an older version used (application ID); they would show up twice.
rm -f "$ICON_DIR/pl.mkarenko.titan_control.svg"

# Every asset: icons, crosshair shapes, the monitor picture.
(cd "$ROOT_DIR/assets" && find . -type f) | while read -r asset; do
    install -Dm644 "$ROOT_DIR/assets/$asset" "$APP_SHARE_DIR/assets/$asset"
done

# An entry an older version created under the application ID would show up twice.
rm -f "$DESKTOP_DIR/pl.mkarenko.titan_control.desktop"
mkdir -p "$DESKTOP_DIR"
cat > "$DESKTOP_DIR/titan_control.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Titan Control
Comment=Control the Titan Army P275MV PLUS monitor
Exec=$BIN_DIR/titan_control
Icon=titan_control
Terminal=false
Categories=Settings;Utility;
StartupNotify=true
StartupWMClass=io.github.mkarenko.titan_control
EOF

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
fi

printf '\nDone. Start it from the application menu or with:\n'
printf '  %s\n' "$BIN_DIR/titan_control"
