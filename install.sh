#!/usr/bin/env bash

set -euo pipefail

if [[ $EUID -ne 0 ]]; then
    echo "Please run this script with sudo."
    echo
    echo "Usage:"
    echo "sudo ./install.sh"
    exit 1
fi

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=========================================="
echo " Acer Control Center Installer"
echo "=========================================="
echo

install_packages() {
    if [[ -f /etc/arch-release ]]; then
        echo "Detected Arch Linux"

        pacman -Sy --noconfirm

        pacman -S --needed --noconfirm \
            git \
            gcc \
            make \
            dkms \
            linux-headers \
            base-devel \
            meson \
            ninja \
            rust \
            cargo \
            pkgconf \
            gtk4 \
            libadwaita

    elif [[ -f /etc/debian_version ]]; then
        echo "Detected Debian / Ubuntu"

        apt update

        apt install -y \
            git \
            gcc \
            g++ \
            make \
            dkms \
            build-essential \
            linux-headers-$(uname -r) \
            meson \
            ninja-build \
            cargo \
            rustc \
            pkg-config \
            libgtk-4-dev \
            libadwaita-1-dev

    elif [[ -f /etc/redhat-release ]]; then
        echo "Detected Fedora"

        dnf install -y \
            git \
            gcc \
            gcc-c++ \
            make \
            dkms \
            kernel-devel \
            kernel-headers \
            meson \
            ninja-build \
            cargo \
            rust \
            pkgconf-pkg-config \
            gtk4-devel \
            libadwaita-devel

    elif command -v zypper >/dev/null 2>&1; then
        echo "Detected openSUSE"

        zypper install -y \
            git \
            gcc \
            gcc-c++ \
            make \
            dkms \
            kernel-devel \
            kernel-headers \
            meson \
            ninja \
            cargo \
            rust \
            pkg-config \
            gtk4-devel \
            libadwaita-devel

    else
        echo "Unsupported Linux distribution."
        exit 1
    fi
}

install_packages

echo
echo "==> Cleaning previous driver build..."

make -C "$ROOT_DIR/driver" clean || true

echo
echo "==> Installing DKMS driver..."

make -C "$ROOT_DIR/driver" dkmsinstall

echo
echo "==> Building libacercontrol..."

if [[ -d "$ROOT_DIR/lib/build" ]]; then
    meson setup "$ROOT_DIR/lib/build" "$ROOT_DIR/lib" --reconfigure
else
    meson setup "$ROOT_DIR/lib/build" "$ROOT_DIR/lib"
fi

meson compile -C "$ROOT_DIR/lib/build"

echo
echo "==> Installing libacercontrol..."

meson install -C "$ROOT_DIR/lib/build"

ldconfig

echo
echo "==> Building daemon..."

cargo build \
    --release \
    --manifest-path "$ROOT_DIR/daemon/Cargo.toml"

echo
echo "==> Installing daemon..."

install -Dm755 \
    "$ROOT_DIR/daemon/target/release/acer-control-daemon" \
    /usr/bin/acer-control-daemon

echo
echo "==> Building GUI..."

cargo build \
    --release \
    --manifest-path "$ROOT_DIR/gui/Cargo.toml"

echo
echo "==> Installing GUI..."

install -Dm755 \
    "$ROOT_DIR/gui/target/release/acer-control-center" \
    /usr/bin/acer-control-center

echo
echo "==> Installing launcher and icon..."

install -Dm644 \
    "$ROOT_DIR/packaging/desktop/io.github.stanlysilas.AcerControlCenter.desktop" \
    /usr/share/applications/io.github.stanlysilas.AcerControlCenter.desktop

install -Dm644 \
    "$ROOT_DIR/packaging/icons/io.github.stanlysilas.AcerControlCenter.svg" \
    /usr/share/icons/hicolor/scalable/apps/io.github.stanlysilas.AcerControlCenter.svg

gtk-update-icon-cache -q -t /usr/share/icons/hicolor 2>/dev/null || true
update-desktop-database -q /usr/share/applications 2>/dev/null || true

echo
echo "==> Installing udev rules..."

# The udev rule hands the driver device to this group.
getent group acercontrol >/dev/null || groupadd --system acercontrol

install -Dm644 \
    "$ROOT_DIR/packaging/udev/99-acercontrol.rules" \
    /etc/udev/rules.d/99-acercontrol.rules

udevadm control --reload-rules
udevadm trigger

echo
echo "==> Installing D-Bus policy..."

install -Dm644 \
    "$ROOT_DIR/packaging/dbus/io.github.stanlysilas.AcerControl.conf" \
    /usr/share/dbus-1/system.d/io.github.stanlysilas.AcerControl.conf

echo
echo "==> Reloading D-Bus..."

busctl call \
    --system \
    org.freedesktop.DBus \
    /org/freedesktop/DBus \
    org.freedesktop.DBus \
    ReloadConfig >/dev/null || true

echo
echo "==> Installing systemd service..."

install -Dm644 \
    "$ROOT_DIR/packaging/systemd/acer-control-daemon.service" \
    /usr/lib/systemd/system/acer-control-daemon.service

systemctl daemon-reload

echo
echo "==> Enabling daemon..."

systemctl enable acer-control-daemon.service

echo
echo "==> Starting daemon..."

systemctl restart acer-control-daemon.service

echo
echo "=========================================="
echo " Acer Control Center installed successfully"
echo "=========================================="
echo
echo "Please reboot your laptop before first use, then open"
echo "\"Acer Control Center\" from your application menu"
echo "(or run: acer-control-center)."
echo
if ! command -v envycontrol >/dev/null 2>&1; then
    echo "Optional: install EnvyControl to enable GPU mode switching"
    echo "(Integrated / Hybrid / NVIDIA) from the GPU page."
    echo
fi
