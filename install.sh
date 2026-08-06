#!/usr/bin/env bash
set -e

echo "Building cxm in release mode..."
cargo build --release

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

echo "Installing binary to $INSTALL_DIR/cxm..."
cp target/release/cxm "$INSTALL_DIR/cxm"
chmod +x "$INSTALL_DIR/cxm"

echo "Installing shell autocompletions..."

# Bash
BASH_DIR="$HOME/.local/share/bash-completion/completions"
mkdir -p "$BASH_DIR"
"$INSTALL_DIR/cxm" completions bash > "$BASH_DIR/cxm" 2>/dev/null || true

# Zsh
ZSH_DIR="$HOME/.zsh/completion"
mkdir -p "$ZSH_DIR"
"$INSTALL_DIR/cxm" completions zsh > "$ZSH_DIR/_cxm" 2>/dev/null || true

# Fish
FISH_DIR="$HOME/.config/fish/completions"
mkdir -p "$FISH_DIR"
"$INSTALL_DIR/cxm" completions fish > "$FISH_DIR/cxm.fish" 2>/dev/null || true

echo "✔ Successfully installed cxm & shell completions!"
cxm --version
