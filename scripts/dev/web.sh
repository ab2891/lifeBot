#!/usr/bin/env bash
# Development launcher for the Axum web-server mode.
# Builds the Svelte frontend, then starts the Rust server.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

echo "[web] Building frontend..."
cd "$REPO_ROOT/apps/desktop"
npx vite build

echo "[web] Starting Axum server..."
cd "$REPO_ROOT"
LIFEBOT_STATIC_DIR="$REPO_ROOT/apps/desktop/dist" \
LIFEBOT_DATA_DIR="$REPO_ROOT/data" \
  cargo run -p lifebot-web-server
