#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
echo "=== Go SDK conformance ==="
cd "$SCRIPT_DIR/runners/go-runner"
go run . "$SCRIPT_DIR/fixtures"
