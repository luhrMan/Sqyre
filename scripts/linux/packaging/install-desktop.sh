#!/usr/bin/env bash
# Install com.sqyre.app.desktop + hicolor icon for GNOME/Wayland dock integration.
#
# Required when running ./bin/sqyre from a dev build: Wayland compositors resolve
# window icons from the .desktop file whose name matches the app_id.
#
# Usage: scripts/linux/packaging/install-desktop.sh [/path/to/sqyre/binary]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/lib/repo-root.sh
. "$SCRIPT_DIR/../../lib/repo-root.sh"

SQYRE_BIN="${1:-$REPO_ROOT/bin/sqyre}"
DESKTOP_SRC="$SCRIPT_DIR/appimage/com.sqyre.app.desktop"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
DESKTOP_DIR="$DATA_HOME/applications"
ICON_DIR="$DATA_HOME/icons/hicolor/256x256/apps"
DESKTOP_DST="$DESKTOP_DIR/com.sqyre.app.desktop"
ICON_DST="$ICON_DIR/com.sqyre.app.png"

"$SCRIPT_DIR/generate-app-icon.sh" 256

mkdir -p "$DESKTOP_DIR" "$ICON_DIR"
cp -f "$REPO_ROOT/crates/sqyre-app/assets/icons/sqyre.png" "$ICON_DST"

# Point Exec at the actual binary (absolute path survives PATH changes).
SQYRE_BIN="$(readlink -f "$SQYRE_BIN")"
if [ ! -x "$SQYRE_BIN" ]; then
  echo "WARNING: $SQYRE_BIN is not executable; desktop entry may not launch." >&2
fi

sed "s|^Exec=.*|Exec=$SQYRE_BIN|" "$DESKTOP_SRC" > "$DESKTOP_DST"
chmod 644 "$DESKTOP_DST" "$ICON_DST"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
fi

echo "Installed:"
echo "  $DESKTOP_DST"
echo "  $ICON_DST"
echo ""
echo "Log out/in or run: gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell --method org.gnome.Shell.Eval \"global.reexec_self()\""
