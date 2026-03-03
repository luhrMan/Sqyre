#!/usr/bin/env bash
# Export the Flatpak build to a single-file bundle (.flatpak) that you can copy
# to your host and install there (where you have a display).
# Run from repo root after a successful flatpak-builder build.
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$SCRIPT_DIR/repo"
BUNDLE="$SCRIPT_DIR/com.sqyre.app.flatpak"
BRANCH="${1:-stable}"
BUILD_DIR="${2:-build-dir}"

if [ ! -d "$BUILD_DIR" ]; then
  echo "Build dir '$BUILD_DIR' not found. Build first with:" >&2
  echo "  flatpak-builder --user --force-clean build-dir $SCRIPT_DIR/com.sqyre.app.yml" >&2
  exit 1
fi

rm -rf "$REPO_DIR"
mkdir -p "$REPO_DIR"

echo "Exporting build to repo..."
flatpak build-export "$REPO_DIR" "$BUILD_DIR" "$BRANCH"

echo "Creating bundle $BUNDLE..."
flatpak build-bundle "$REPO_DIR" "$BUNDLE" com.sqyre.app "$BRANCH"

echo "Done. Copy to your host and run:"
echo "  flatpak install --user com.sqyre.app.flatpak"
echo "  flatpak run com.sqyre.app"
echo ""
echo "On the host, ensure the Freedesktop runtime is installed (one-time):"
echo "  flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo"
echo "  flatpak install flathub org.freedesktop.Platform//25.08"
