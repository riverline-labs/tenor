#!/usr/bin/env bash
# Build the WASM module and copy artifacts into the npm package.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
OUT_DIR="$SCRIPT_DIR/../wasm"

echo "Building WASM module..."
cd "$REPO_ROOT/crates/tenor-eval-wasm"
wasm-pack build --target nodejs --out-dir "$OUT_DIR"

echo "WASM module built successfully at $OUT_DIR"
