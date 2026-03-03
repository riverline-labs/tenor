#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Generating conformance fixtures from Rust evaluator..."
cd "$REPO_ROOT"
cargo run --manifest-path sdks/conformance/fixture-gen/Cargo.toml -- "$SCRIPT_DIR/fixtures"
echo "Done."
