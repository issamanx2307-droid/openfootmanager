#!/usr/bin/env bash
set -euo pipefail

REPO_DIR="${1:-/srv/openfootballmanager/source}"
INSTALL_DIR="${2:-/srv/openfootballmanager/bin}"

cd "$REPO_DIR/src-tauri"
cargo build --release -p albion_server
install -D -m 755 target/release/albion_server "$INSTALL_DIR/albion_server"
echo "Installed $INSTALL_DIR/albion_server"
