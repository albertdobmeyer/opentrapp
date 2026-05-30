#!/usr/bin/env bash
# Test: Kill switch functions correctly
# NOTE: This test is destructive — it will stop the vault stack.
# Re-run setup.sh after this test.
set -euo pipefail

VAULT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RUNTIME="${RUNTIME:-podman}"

echo "=== Kill Switch Tests ==="
echo "WARNING: This test will stop your vault stack."
echo ""

# Test 1: Soft kill stops containers
echo -n "  Soft kill: "
bash "$VAULT_DIR/scripts/kill.sh" --soft >/dev/null 2>&1
running=$($RUNTIME ps --filter name=opencli-container --format "{{.Names}}" 2>/dev/null || true)
if [ -z "$running" ]; then
    echo "PASS (containers stopped)"
else
    echo "FAIL — container still running after soft kill"
    exit 1
fi

# Restart for hard kill test
cd "$VAULT_DIR"
$RUNTIME compose up -d >/dev/null 2>&1
sleep 3

# Test 2: Hard kill removes everything
echo -n "  Hard kill: "
bash "$VAULT_DIR/scripts/kill.sh" --hard >/dev/null 2>&1
remaining=$($RUNTIME ps -a --filter name=opencli-container --format "{{.Names}}" 2>/dev/null || true)
if [ -z "$remaining" ]; then
    echo "PASS (containers removed)"
else
    echo "FAIL — container still exists after hard kill"
    exit 1
fi

echo ""
echo "=== All kill switch tests passed ==="
echo "NOTE: Run setup.sh to restart the vault stack."
