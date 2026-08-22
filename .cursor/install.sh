#!/usr/bin/env bash
# Idempotent Cloud Agent bootstrap for Sqyre.
# Installs the native system dependencies (Tesseract/Leptonica, X11, clang)
# that the Rust workspace links against, then warms the build cache by
# compiling the GUI binary. Safe to re-run; apt is a no-op once satisfied
# and cargo builds incrementally.
set -euo pipefail

SUDO=""
if [ "$(id -u)" -ne 0 ]; then
  SUDO="sudo"
fi

export DEBIAN_FRONTEND=noninteractive

$SUDO apt-get update -qq
$SUDO apt-get install -y --no-install-recommends \
  build-essential pkg-config \
  clang libclang-dev \
  tesseract-ocr tesseract-ocr-eng libtesseract-dev libleptonica-dev \
  libx11-dev libxkbcommon-dev libxkbcommon-x11-dev \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libwayland-dev libegl1-mesa-dev libdbus-1-dev \
  libfontconfig1-dev libasound2-dev \
  mesa-vulkan-drivers libvulkan1 \
  libxtst-dev libxrandr-dev libxinerama-dev libxi-dev

# Warm the workspace build cache and validate the native link chain.
cargo build -p sqyre-app --locked
