#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SDK_DIR="$SCRIPT_DIR/../python"
echo "=== Python SDK conformance ==="

# Use the .venv if it exists (created by maturin develop), otherwise fall back
# to PYTHONPATH pointing at the pre-built .so in python/tenor/_native.abi3.so.
if [ -f "$SDK_DIR/.venv/bin/python" ]; then
    "$SDK_DIR/.venv/bin/python" "$SCRIPT_DIR/runners/python_runner.py" "$SCRIPT_DIR/fixtures"
else
    PYTHONPATH="$SDK_DIR/python" python3 "$SCRIPT_DIR/runners/python_runner.py" "$SCRIPT_DIR/fixtures"
fi
