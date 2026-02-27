#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
echo "=== TypeScript SDK conformance ==="
cd "$SCRIPT_DIR/../typescript"
npx tsx "$SCRIPT_DIR/runners/typescript-runner.ts" "$SCRIPT_DIR/fixtures"
