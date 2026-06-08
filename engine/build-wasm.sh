#!/usr/bin/env bash
# Build the wargame WASM and install it into web/ (committed artifact, so the
# served site stays buildless). Run from anywhere.
#
#   engine/build-wasm.sh
#
# One-time setup: rustup target add wasm32-unknown-unknown
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

cargo build -p opengine-wasm --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/opengine_wasm.wasm ../web/wargame.wasm

size=$(du -h ../web/wargame.wasm | cut -f1)
echo "installed web/wargame.wasm ($size)"
