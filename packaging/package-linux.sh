#!/usr/bin/env bash
# Build and package a release for the current Linux x86_64 machine.
# Output: dist/snake-<version>-linux-x86_64.tar.gz
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
PKG="snake-${VERSION}-linux-x86_64"

cargo build --release --locked

rm -rf "dist/$PKG" "dist/$PKG.tar.gz"
mkdir -p "dist/$PKG"
cp target/release/snake "dist/$PKG/"
cp README.md packaging/snake.desktop packaging/snake.png packaging/install-linux.sh "dist/$PKG/"
chmod +x "dist/$PKG/snake" "dist/$PKG/install-linux.sh"
tar -C dist -czf "dist/$PKG.tar.gz" "$PKG"

echo "Created dist/$PKG.tar.gz"
