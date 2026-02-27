#!/usr/bin/env bash
set -euo pipefail

# Build the Tenor WASM bridge for use with the Go SDK via wazero.
# Produces a raw wasm32-unknown-unknown binary with exported C-ABI functions.
# No wasm-bindgen or Node.js required.

REPO_ROOT="$(cd "$(dirname "$0")/../../../" && pwd)"
OUT_DIR="$(cd "$(dirname "$0")/.." && pwd)/internal/wasm"
BRIDGE_DIR="$REPO_ROOT/sdks/go/wasm-bridge"

echo "Building Tenor WASM bridge (wasm32-unknown-unknown)..."
cargo build \
  --manifest-path "$BRIDGE_DIR/Cargo.toml" \
  --target wasm32-unknown-unknown \
  --release

# The bridge crate has its own workspace ([workspace] in Cargo.toml),
# so its output goes to the bridge directory's own target/, not the root target/.
WASM_FILE="$BRIDGE_DIR/target/wasm32-unknown-unknown/release/tenor_wasm_bridge.wasm"

if [ ! -f "$WASM_FILE" ]; then
  echo "ERROR: Expected WASM file not found at $WASM_FILE" >&2
  exit 1
fi

cp "$WASM_FILE" "$OUT_DIR/tenor_eval.wasm"
echo "WASM binary copied to $OUT_DIR/tenor_eval.wasm ($(wc -c < "$OUT_DIR/tenor_eval.wasm") bytes)"
