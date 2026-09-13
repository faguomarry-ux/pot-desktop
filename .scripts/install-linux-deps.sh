#!/usr/bin/env bash
set -euo pipefail

# Native Ubuntu/Debian build dependencies for Tauri 2 (WebKitGTK 4.1 / libsoup 3).
# Run as the regular desktop user; sudo is used only for apt.
sudo apt-get update
sudo apt-get install -y \
    build-essential pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev librsvg2-dev patchelf libssl-dev \
    libxdo-dev libxcb1-dev libxrandr-dev libdbus-1-dev libclang-dev \
    tesseract-ocr rpm
pkg-config --modversion gtk+-3.0 webkit2gtk-4.1 libsoup-3.0
