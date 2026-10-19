#!/usr/bin/env bash
# Install Snake for the current user (no root required).
# Usage: ./install-linux.sh
set -euo pipefail

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
APP_DIR="$PREFIX/share/applications"
ICON_DIR="$PREFIX/share/icons/hicolor/512x512/apps"

install -Dm755 snake "$BIN_DIR/snake"
install -Dm644 snake.desktop "$APP_DIR/snake.desktop"
install -Dm644 snake.png "$ICON_DIR/snake.png"

update-desktop-database "$APP_DIR" 2>/dev/null || true
gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true

echo "Snake installed to $BIN_DIR/snake"
echo "Launch it from your app menu or run: snake"
