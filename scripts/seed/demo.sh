#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."
cargo run -p lifebot-core --bin lifebot-admin -- seed-demo
