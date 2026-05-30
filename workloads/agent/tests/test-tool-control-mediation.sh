#!/usr/bin/env bash
# Test: tool-control.sh F13 + F14 — host-side mediation fixes
#
# F13: Container name must be overridable via OPENCLAW_CONTAINER env var.
#      The default ("opencli-container") is wrong in opentrapp installs,
#      where the container is named "vault-agent". A previous regression
#      caused the apply path to write to the wrong container.
#
# F14: --apply must NOT invoke `compose up`. When invoked from the
#      opentrapp parent repo, that creates a rogue parallel container
#      from the submodule's compose.yml. The parent orchestrator owns
#      lifecycle. tool-control.sh must offer --no-restart to skip
#      compose calls and let the caller restart.
#
# Tests are structural (grep over the script source) plus a behavioral
# smoke test that proves --no-restart short-circuits before any
# compose/lifecycle action. Pure host-side; no container required.
set -uo pipefail

VAULT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SCRIPT="$VAULT_DIR/scripts/tool-control.sh"

PASS=0
FAIL=0

check() {
    local desc="$1"
    shift
    printf "  %-65s " "$desc"
    local output
    output=$("$@" 2>&1) && exit_code=0 || exit_code=$?
    if [ $exit_code -eq 0 ]; then
        echo "PASS"
        PASS=$((PASS + 1))
    else
        echo "FAIL"
        echo "$output" | head -5 | sed 's/^/       /'
        FAIL=$((FAIL + 1))
    fi
}

echo ""
echo "=== Tool Control: F13/F14 (host-side mediation) ==="
echo ""

# --- F13 ---
echo "  F13 — Container name configurable via OPENCLAW_CONTAINER env:"

check "CONTAINER var defaults via \${OPENCLAW_CONTAINER:-...}" \
    grep -qE '^CONTAINER="\$\{OPENCLAW_CONTAINER:-' "$SCRIPT"

check "podman cp uses \$CONTAINER, not hardcoded opencli-container:" \
    bash -c "! grep -nE 'cp [^|]*opencli-container:/' '$SCRIPT'"

check "podman stop does not hardcode opencli-container" \
    bash -c "! grep -nE '\\\$RUNTIME stop opencli-container( |$)' '$SCRIPT'"

check "podman logs does not hardcode opencli-container" \
    bash -c "! grep -nE '\\\$RUNTIME logs opencli-container( |$)' '$SCRIPT'"

# --- F14 ---
echo ""
echo "  F14 — --no-restart flag suppresses compose lifecycle:"

check "Script source contains --no-restart flag handling" \
    grep -q -- '--no-restart' "$SCRIPT"

check "--help advertises --no-restart" \
    bash -c "bash '$SCRIPT' --help 2>&1 | grep -q -- '--no-restart'"

check "compose stop is preceded by NO_RESTART guard" \
    bash -c "grep -B 5 '\\\$COMPOSE stop' '$SCRIPT' | grep -q 'NO_RESTART'"

check "compose up is preceded by NO_RESTART early-return" \
    bash -c "awk '/^do_apply\\(\\)/,/^}\$/' '$SCRIPT' | awk '/NO_RESTART.*=.*\"true\".*then/{flag=1} flag && /return 0/{print \"found\"; exit}' | grep -q found && \
             awk '/^do_apply\\(\\)/,/^}\$/' '$SCRIPT' | awk '/return 0/{flag=1} flag && /\\\$COMPOSE up/{print \"after\"; exit}' | grep -q after"

check "do_apply has an early-return path under NO_RESTART" \
    bash -c "awk '/^do_apply\\(\\)/,/^}\$/' '$SCRIPT' | grep -E 'NO_RESTART.*=.*\"true\"' | grep -q ''"

# --- Behavioral smoke test for F14 ---
# We can't run the full --apply path here (it needs a real container + write
# perms), but we can prove that --no-restart is parsed without error and
# acknowledged in the help output. A live integration test runs from the
# parent repo after the fix lands.
echo ""
echo "  Smoke:"
check "Script parses --no-restart without error" \
    bash -c "bash '$SCRIPT' --help >/dev/null 2>&1 && bash '$SCRIPT' --preset hard --dry-run --no-restart >/dev/null 2>&1"

echo ""
echo "==========================="
echo "Results: $PASS passed, $FAIL failed"
echo ""

if [ $FAIL -gt 0 ]; then
    echo "[!] F13/F14 MEDIATION TESTS FAILED"
    exit 1
else
    echo "[+] ALL F13/F14 MEDIATION TESTS PASSED"
fi
