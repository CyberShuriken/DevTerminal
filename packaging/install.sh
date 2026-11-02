#!/bin/sh
# Install devterm as a desktop app for the current user (no root needed).
#   sh packaging/install.sh            install / update
#   sh packaging/install.sh --uninstall
set -eu
cd "$(dirname "$0")/.."
BIN="$HOME/.local/bin"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
HICOLOR="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor"
SIZES="32 48 64 128 256 512"

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "$BIN/devterm-launch" "$APPS/devterm.desktop" "$HICOLOR/scalable/apps/devterm.svg"
    for n in $SIZES; do rm -f "$HICOLOR/${n}x${n}/apps/devterm.png"; done
    cargo uninstall devterm 2>/dev/null || true
    update-desktop-database "$APPS" 2>/dev/null || true
    echo "devterm uninstalled."
    exit 0
fi

cargo install --path . --locked
mkdir -p "$BIN" "$APPS"
install -m 755 packaging/devterm-launch "$BIN/devterm-launch"
install -Dm 644 packaging/devterm.svg "$HICOLOR/scalable/apps/devterm.svg"
for n in $SIZES; do
    out="$HICOLOR/${n}x${n}/apps/devterm.png"
    mkdir -p "$(dirname "$out")"
    if command -v magick >/dev/null 2>&1; then
        magick -background none -density 600 packaging/devterm.svg -resize "${n}x${n}" "$out"
    elif command -v rsvg-convert >/dev/null 2>&1; then
        rsvg-convert -w "$n" -h "$n" packaging/devterm.svg -o "$out"
    else
        install -m 644 packaging/devterm.png "$out"  # prebuilt 256px; scaled by the desktop
    fi
done
# Absolute paths: desktop sessions often don't have ~/.local/bin or ~/.cargo/bin on PATH.
# Absolute icon path too, so a stale icon cache can't show a broken icon.
sed -e "s|@LAUNCH@|env DEVTERM_BIN=$HOME/.cargo/bin/devterm $BIN/devterm-launch|" \
    -e "s|^Icon=devterm|Icon=$HICOLOR/256x256/apps/devterm.png|" \
    packaging/devterm.desktop > "$APPS/devterm.desktop"
update-desktop-database "$APPS" 2>/dev/null || true
# -f -t: rebuild even without an index.theme in the user icon dir.
gtk-update-icon-cache -f -t -q "$HICOLOR" 2>/dev/null || true
echo "devterm installed. Find it in your app menu, then right-click → Pin to Dash."
