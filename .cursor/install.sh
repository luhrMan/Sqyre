#!/usr/bin/env bash
# Cloud Agent install for the Sqyre Rust workspace (egui / PureCV / Tesseract).
# Idempotent: installs native build deps, refreshes the cargo cache, ensures
# cargo-nextest, and builds the debug binary into ./bin/sqyre.
set -euo pipefail

# System libraries required to build & run Sqyre on Linux/X11 + Wayland portals.
# Mirrors the build-relevant subset of .devcontainer/Dockerfile.
PACKAGES=(
  build-essential pkg-config clang libclang-dev
  # leptess / Tesseract OCR
  tesseract-ocr tesseract-ocr-eng libtesseract-dev libleptonica-dev
  # egui / eframe / winit (X11 + xkb)
  libx11-dev libxkbcommon-dev libxkbcommon-x11-dev
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
  # Wayland portal capture + EGL + D-Bus + PipeWire
  libwayland-dev libegl1-mesa-dev libdbus-1-dev libpipewire-0.3-dev libspa-0.2-dev
  # Fonts + audio
  libfontconfig1-dev libasound2-dev
  # Software Vulkan (lavapipe) for headless wgpu rendering / egui screenshot tests
  mesa-vulkan-drivers libvulkan1
  # rustautogui X11 capture & input
  libxtst-dev libxrandr-dev libxinerama-dev libxi-dev libxfixes-dev
  # rdev unstable_grab (Wayland evdev hotkeys)
  libevdev-dev
  # Headless display for running the GUI without a physical screen
  xvfb
)

if command -v apt-get >/dev/null 2>&1; then
  sudo apt-get update
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends "${PACKAGES[@]}"
fi

# cargo-nextest powers `make test`; install only when missing.
if ! cargo nextest --version >/dev/null 2>&1; then
  cargo install cargo-nextest --locked
fi

# Warm the dependency cache, then build the default debug binary -> ./bin/sqyre.
cargo fetch --locked
make sqyre

echo "install.sh complete: $(./bin/sqyre --version 2>/dev/null || echo 'sqyre build present')"
