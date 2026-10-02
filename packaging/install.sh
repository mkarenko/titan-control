#!/usr/bin/env sh
# Installs Titan Control from this archive.
#   ./install.sh                    -> ~/.local (as a user) or /usr/local (as root)
#   ./install.sh --prefix /opt/tc   -> a prefix of your choice
# The udev rule and the i2c-dev module list are installed only when the prefix is /usr or /usr/local (system-wide);
# otherwise the script tells you what to do by hand.
set -eu

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ "$(id -u)" -eq 0 ]; then PREFIX=/usr/local; else PREFIX="$HOME/.local"; fi
if [ "${1:-}" = "--prefix" ]; then PREFIX=${2:?missing directory}; fi

printf '==> Installing into %s\n' "$PREFIX"
install -Dm755 "$HERE/bin/titan_control" "$PREFIX/bin/titan_control"
(cd "$HERE/share" && find . -type f) | while read -r file; do
    install -Dm644 "$HERE/share/$file" "$PREFIX/share/$file"
done
# The menu entry must point at the installed binary.
sed -i "s|^Exec=.*|Exec=$PREFIX/bin/titan_control|" "$PREFIX/share/applications/titan_control.desktop"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$PREFIX/share/applications" >/dev/null 2>&1 || true
command -v gtk4-update-icon-cache >/dev/null 2>&1 && gtk4-update-icon-cache -qtf "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true

case "$PREFIX" in
    /usr|/usr/local)
        if [ "$(id -u)" -eq 0 ]; then
            install -Dm644 "$HERE/lib/udev/rules.d/60-titan-control-i2c.rules" /etc/udev/rules.d/60-titan-control-i2c.rules
            install -Dm644 "$HERE/lib/modules-load.d/titan-control.conf" /etc/modules-load.d/titan-control.conf
            modprobe i2c-dev 2>/dev/null || true
            udevadm control --reload 2>/dev/null && udevadm trigger --subsystem-match=i2c-dev 2>/dev/null || true
            printf '==> I2C access set up (udev rule + i2c-dev module).\n'
        else
            printf '==> Run as root to also install the I2C rules, or see the README (I2C access).\n'
        fi
        ;;
    *)
        printf '==> Set up I2C access by hand (see the README, "I2C access"): load i2c-dev and allow your user\n'
        printf '    to open /dev/i2c-*; the rule is in %s/lib/udev/rules.d/.\n' "$HERE"
        ;;
esac
case ":$PATH:" in
    *":$PREFIX/bin:"*) ;;
    *) printf '==> Note: %s/bin is not in your PATH; use the menu entry or the full path.\n' "$PREFIX" ;;
esac
printf '==> Done. Start Titan Control from the menu or with: %s/bin/titan_control\n' "$PREFIX"
