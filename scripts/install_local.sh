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
install -Dm644 "$ROOT_DIR/assets/pl.mkarenko.titan_control.svg" "$APP_SHARE_DIR/assets/pl.mkarenko.titan_control.svg"
install -Dm644 "$ROOT_DIR/assets/pl.mkarenko.titan_control.svg" "$ICON_DIR/pl.mkarenko.titan_control.svg"

mkdir -p "$APP_SHARE_DIR/assets/icons"
for icon_file in "$ROOT_DIR"/assets/icons/*.svg; do
    install -Dm644 "$icon_file" "$APP_SHARE_DIR/assets/icons/$(basename "$icon_file")"
done

cat > "$DESKTOP_DIR/pl.mkarenko.titan_control.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Titan Control
Comment=Titan Control
Exec=$BIN_DIR/titan_control
Icon=pl.mkarenko.titan_control
Terminal=false
Categories=Settings;Utility;
StartupNotify=true
StartupWMClass=pl.mkarenko.titan_control
EOF

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
fi

printf '\nGotowe. Uruchamiaj z menu aplikacji albo komendą:\n'
printf '  %s\n' "$BIN_DIR/titan_control"
