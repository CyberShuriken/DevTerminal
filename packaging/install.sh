#!/bin/sh
# Install devterm as a desktop app for the current user (no root needed).
#   sh packaging/install.sh            install / update
#   sh packaging/install.sh --uninstall
set -eu
cd "$(dirname "$0")/.."
BIN="$HOME/.local/bin"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
ICONS="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/scalable/apps"

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "$BIN/devterm-launch" "$APPS/devterm.desktop" "$ICONS/devterm.svg"
    cargo uninstall devterm 2>/dev/null || true
    update-desktop-database "$APPS" 2>/dev/null || true
    echo "devterm uninstalled."
    exit 0
fi

cargo install --path . --locked
mkdir -p "$BIN" "$APPS" "$ICONS"
install -m 755 packaging/devterm-launch "$BIN/devterm-launch"
install -m 644 packaging/devterm.svg "$ICONS/devterm.svg"
# Absolute paths: desktop sessions often don't have ~/.local/bin or ~/.cargo/bin on PATH.
sed "s|@LAUNCH@|env DEVTERM_BIN=$HOME/.cargo/bin/devterm $BIN/devterm-launch|" \
    packaging/devterm.desktop > "$APPS/devterm.desktop"
update-desktop-database "$APPS" 2>/dev/null || true
gtk-update-icon-cache -q "${ICONS%/scalable/apps}" 2>/dev/null || true
echo "devterm installed. Find it in your app menu, then right-click → Pin to Dash."
