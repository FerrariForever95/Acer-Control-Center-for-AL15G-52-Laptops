#!/usr/bin/env bash

set -e

if [[ $EUID -ne 0 ]]; then
    echo "Please run this script with sudo."
    exit 1
fi

echo "=========================================="
echo " Acer Control Center Uninstaller"
echo "=========================================="
echo

echo "==> Stopping daemon..."

systemctl stop acer-control-daemon.service 2>/dev/null || true
systemctl disable acer-control-daemon.service 2>/dev/null || true

echo "==> Removing systemd service..."

rm -f /usr/lib/systemd/system/acer-control-daemon.service
rm -f /etc/systemd/system/acer-control-daemon.service

systemctl daemon-reload
systemctl reset-failed 2>/dev/null || true

echo "==> Removing D-Bus policy..."

rm -f /usr/share/dbus-1/system.d/io.github.stanlysilas.AcerControl.conf

echo "==> Removing udev rules..."

rm -f /etc/udev/rules.d/99-acercontrol.rules

udevadm control --reload-rules
udevadm trigger

echo "==> Removing executables..."

rm -f /usr/bin/acer-control-center
rm -f /usr/bin/acer-control-daemon

echo "==> Removing desktop entry..."

rm -f /usr/share/applications/acer-control-center.desktop
rm -f /usr/share/applications/io.github.stanlysilas.AcerControlCenter.desktop

echo "==> Removing icons..."

rm -f /usr/share/icons/hicolor/256x256/apps/acer-control-center.png
rm -f /usr/share/icons/hicolor/scalable/apps/io.github.stanlysilas.AcerControlCenter.svg
rm -f /usr/share/pixmaps/acer-control-center.png

echo "==> Removing shared library..."

rm -f /usr/lib/libacercontrol.so
rm -f /usr/lib64/libacercontrol.so

ldconfig

echo "==> Removing DKMS module..."

if command -v dkms >/dev/null 2>&1; then
    if dkms status | grep -q "^acer_keyboard"; then
        VERSION=$(dkms status | awk -F'[:,]' '/^acer_keyboard/ {print $2; exit}' | xargs)

        if [[ -n "$VERSION" ]]; then
            dkms remove acer_keyboard/"$VERSION" --all || true
        fi
    fi
fi

rm -rf /usr/src/acer_keyboard-*

echo "==> Removing loaded kernel module..."

modprobe -r acer_keyboard 2>/dev/null || true

depmod -a

echo
echo "=========================================="
echo " Acer Control Center has been removed."
echo "=========================================="
echo
echo "A reboot is recommended."
