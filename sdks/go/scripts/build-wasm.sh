#!/usr/bin/env bash
set -euo pipefail

# Build the Tenor WASM bridge for use with the Go SDK via wazero.
# Produces a wasm32-wasip1 binary with exported C-ABI functions.
# WASI imports are satisfied by wazero's built-in wasi_snapshot_preview1 module.

REPO_ROOT="$(cd "$(dirname "$0")/../../../" && pwd)"
OUT_DIR="$(cd "$(dirname "$0")/.." && pwd)/internal/wasm"
BRIDGE_DIR="$REPO_ROOT/sdks/go/wasm-bridge"

echo "Building Tenor WASM bridge (wasm32-wasip1)..."
cargo build \
  --manifest-path "$BRIDGE_DIR/Cargo.toml" \
  --target wasm32-wasip1 \
  --release

# The bridge crate has its own workspace ([workspace] in Cargo.toml),
# so its output goes to the bridge directory's own target/, not the root target/.
WASM_FILE="$BRIDGE_DIR/target/wasm32-wasip1/release/tenor_wasm_bridge.wasm"

if [ ! -f "$WASM_FILE" ]; then
  echo "ERROR: Expected WASM file not found at $WASM_FILE" >&2
  exit 1
fi

cp "$WASM_FILE" "$OUT_DIR/tenor_eval.wasm"
echo "WASM binary copied to $OUT_DIR/tenor_eval.wasm ($(wc -c < "$OUT_DIR/tenor_eval.wasm") bytes)"
