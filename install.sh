#!/usr/bin/env bash
set -e

echo "Building cxm in release mode..."
cargo build --release

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

echo "Installing binary to $INSTALL_DIR/cxm..."
cp target/release/cxm "$INSTALL_DIR/cxm"
chmod +x "$INSTALL_DIR/cxm"

echo "✔ Successfully installed cxm to $INSTALL_DIR/cxm"
cxm --version
