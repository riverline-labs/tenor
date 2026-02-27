#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "========================================"
echo "  Tenor SDK Cross-Conformance Suite"
echo "========================================"
echo ""

TOTAL_PASS=0
TOTAL_FAIL=0
SDK_RESULTS=()

run_sdk() {
    local name="$1"
    local script="$2"
    echo "--- $name ---"
    if bash "$script"; then
        SDK_RESULTS+=("$name: PASS")
        TOTAL_PASS=$((TOTAL_PASS + 1))
    else
        SDK_RESULTS+=("$name: FAIL")
        TOTAL_FAIL=$((TOTAL_FAIL + 1))
    fi
    echo ""
}

run_sdk "TypeScript" "$SCRIPT_DIR/run-typescript.sh"
run_sdk "Python"     "$SCRIPT_DIR/run-python.sh"
run_sdk "Go"         "$SCRIPT_DIR/run-go.sh"

echo "========================================"
echo "  Summary"
echo "========================================"
for result in "${SDK_RESULTS[@]}"; do
    echo "  $result"
done
echo ""
echo "Total: $TOTAL_PASS passed, $TOTAL_FAIL failed"
echo "========================================"

if [ "$TOTAL_FAIL" -gt 0 ]; then
    echo "CONFORMANCE FAILED"
    exit 1
else
    echo "ALL SDKs CONFORM"
    exit 0
fi
