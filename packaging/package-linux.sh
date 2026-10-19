#!/usr/bin/env bash
# Build and package a release for the current Linux x86_64 machine.
# Output: dist/snake-<version>-linux-x86_64.tar.gz
#         dist/snake-<version>-linux-x86_64.AppImage
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
PKG="snake-${VERSION}-linux-x86_64"

cargo build --release --locked

rm -rf "dist/$PKG" "dist/$PKG.tar.gz" "dist/$PKG.AppImage"
mkdir -p "dist/$PKG"
cp target/release/snake "dist/$PKG/"
cp README.md packaging/snake.desktop packaging/snake.png packaging/install-linux.sh "dist/$PKG/"
chmod +x "dist/$PKG/snake" "dist/$PKG/install-linux.sh"
tar -C dist -czf "dist/$PKG.tar.gz" "$PKG"

# Build a single-file AppImage.
APPDIR="build/AppDir"
TOOL="build/appimagetool"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/512x512/apps"
cp target/release/snake "$APPDIR/usr/bin/snake"
cp packaging/snake.desktop "$APPDIR/snake.desktop"
cp packaging/snake.desktop "$APPDIR/usr/share/applications/snake.desktop"
cp packaging/snake.png "$APPDIR/snake.png"
cp packaging/snake.png "$APPDIR/usr/share/icons/hicolor/512x512/apps/snake.png"
printf '%s\n' \
  '#!/bin/sh' \
  'HERE="$(dirname "$(readlink -f "$0")")"' \
  'exec "$HERE/usr/bin/snake" "$@"' > "$APPDIR/AppRun"
chmod +x "$APPDIR/AppRun"

if [[ ! -x "$TOOL" ]]; then
  mkdir -p build
  curl -sSL -o "$TOOL" \
    https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x "$TOOL"
fi
ARCH=x86_64 "$TOOL" --appimage-extract-and-run "$APPDIR" "dist/$PKG.AppImage"

echo "Created dist/$PKG.tar.gz"
echo "Created dist/$PKG.AppImage"
