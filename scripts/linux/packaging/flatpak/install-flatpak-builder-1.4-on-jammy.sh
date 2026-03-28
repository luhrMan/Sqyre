#!/usr/bin/env bash
# Build and install flatpak-builder 1.4.x into /usr/local on Ubuntu 22.04 LTS (jammy).
# Use this when you cannot move past 22.04: distro flatpak-builder stays at 1.2.x and breaks
# the AppStream finish step against current org.freedesktop.Sdk runtimes.
#
# Run: sudo ./install-flatpak-builder-1.4-on-jammy.sh
set -euo pipefail

FB_VERSION="${FB_VERSION:-1.4.7}"
FB_SHA256="${FB_SHA256:-fd5bc36fe3b974395f782e6c920d8955cee168f513370c32cc800b69acd980d0}"
PREFIX="${PREFIX:-/usr/local}"

if [[ "$(id -u)" -ne 0 ]]; then
  echo "Run as root (sudo)." >&2
  exit 1
fi

. /etc/os-release
if [[ "${VERSION_ID:-}" != "22.04" ]]; then
  echo "Warning: this script targets Ubuntu 22.04; detected ${PRETTY_NAME:-unknown}. Continuing anyway." >&2
fi

apt-get update -qq
# flatpak-builder meson checks `appstreamcli compose`; on Jammy that needs appstream-compose.
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  meson ninja-build pkg-config \
  libglib2.0-dev libjson-glib-dev libcurl4-openssl-dev libxml2-dev \
  libyaml-dev libostree-dev libelf-dev gettext \
  libflatpak-dev \
  appstream appstream-compose \
  debugedit \
  ca-certificates curl xz-utils

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT
cd "$WORKDIR"

ARCHIVE="flatpak-builder-${FB_VERSION}.tar.xz"
curl -fsSL -o "$ARCHIVE" \
  "https://github.com/flatpak/flatpak-builder/releases/download/${FB_VERSION}/${ARCHIVE}"
echo "${FB_SHA256}  ${ARCHIVE}" | sha256sum -c -
tar -xf "$ARCHIVE"
cd "flatpak-builder-${FB_VERSION}"

meson setup build --prefix="$PREFIX" -Dtests=false
meson compile -C build
meson install -C build

echo ""
echo "Installed flatpak-builder ${FB_VERSION} to ${PREFIX}/bin"
echo "Ensure ${PREFIX}/bin is before /usr/bin in PATH, e.g.:"
echo "  export PATH=${PREFIX}/bin:\$PATH"
echo "Verify: ${PREFIX}/bin/flatpak-builder --version"
