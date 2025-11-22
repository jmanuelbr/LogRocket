#!/bin/bash

# Build script for Log Viewer 2
# This script attempts to work around dependency issues

echo "Cleaning cargo cache for problematic packages..."
rm -rf ~/.cargo/registry/src/index.crates.io-*/mime_guess2-* 2>/dev/null

echo "Building Log Viewer 2..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "Build successful!"
    echo "Run with: cargo run --release"
else
    echo "Build failed. Try updating Rust:"
    echo "  brew upgrade rust"
    echo "Or install rustup:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi

