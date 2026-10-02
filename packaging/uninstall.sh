#!/usr/bin/env sh
# Removes what install.sh installed (same --prefix rule). Your settings stay: ~/.config/titan_control and
# ~/.local/state/titan_control (delete them by hand if you want).
set -eu

if [ "$(id -u)" -eq 0 ]; then PREFIX=/usr/local; else PREFIX="$HOME/.local"; fi
if [ "${1:-}" = "--prefix" ]; then PREFIX=${2:?missing directory}; fi

printf '==> Removing from %s\n' "$PREFIX"
rm -f "$PREFIX/bin/titan_control" \
    "$PREFIX/share/applications/titan_control.desktop" \
    "$PREFIX/share/icons/hicolor/scalable/apps/titan_control.svg"
rm -rf "$PREFIX/share/titan_control" "$PREFIX/share/doc/titan-control"
rm -f "$HOME/.config/autostart/titan_control.desktop"
if [ "$(id -u)" -eq 0 ]; then
    rm -f /etc/udev/rules.d/60-titan-control-i2c.rules /etc/modules-load.d/titan-control.conf
fi
printf '==> Done.\n'
