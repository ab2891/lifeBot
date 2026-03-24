#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"

# Clear stale Lifebot dev processes so repeated launches do not fail on port conflicts.
pkill -f "$ROOT_DIR/node_modules/.bin/vite" 2>/dev/null || true
pkill -f "$ROOT_DIR/target/debug/lifebot-desktop" 2>/dev/null || true

cd "$ROOT_DIR/apps/desktop"

# Prevent DMABuf errors on WSL2 (no DRM device). Hardware acceleration is
# disabled in Rust via webkit2gtk's HardwareAccelerationPolicy::Never so the
# webview always paints using the software renderer.
export WEBKIT_DISABLE_DMABUF_RENDERER=1

../../node_modules/.bin/tauri dev
