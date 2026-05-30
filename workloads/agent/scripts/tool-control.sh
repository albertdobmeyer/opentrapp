#!/usr/bin/env bash
# OpenCli-Container: Tool Control — Per-Tool Whitelisting/Blacklisting
#
# Manages which tools the OpenClaw agent can use.
# Shell levels (Hard/Split/Soft) are presets on a sliding zero-trust scale.
# The user can also enable/disable individual tools.
#
# Usage:
#   bash scripts/tool-control.sh --preset hard --dry-run      # Preview Hard Shell config
#   bash scripts/tool-control.sh --preset split --dry-run     # Preview Split Shell config
#   bash scripts/tool-control.sh --preset split --enable web_search --dry-run
#   bash scripts/tool-control.sh --status                     # Show current tool status
#   bash scripts/tool-control.sh --preset split --apply       # Apply (Step 3)
#
# Security: All logic runs on the HOST. The container is untrusted.

set -uo pipefail

VAULT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CORE="$VAULT_DIR/scripts/tool-control-core.py"
MANIFEST="$VAULT_DIR/config/tool-manifest.yml"

RUNTIME="podman"
command -v podman &>/dev/null || RUNTIME="docker"
# Container name is configurable via OPENCLAW_CONTAINER for embedders
# (e.g. opentrapp names this container "vault-agent" in its parent compose).
CONTAINER="${OPENCLAW_CONTAINER:-opencli-container}"
# When true, --apply writes config + allowlist but does NOT touch container
# lifecycle (no compose stop/up, no verify). Caller is responsible for
# restarting the container. Required when invoked from a parent orchestrator
# whose compose definition owns the container.
NO_RESTART=false

# Colors
BOLD='\033[1m'
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# --- Prerequisite checks ---
check_prerequisites() {
    if ! command -v python3 &>/dev/null; then
        echo -e "${RED}ERROR: python3 not found. Required for config generation.${NC}" >&2
        exit 1
    fi
    if ! python3 -c "import yaml" 2>/dev/null; then
        echo -e "${RED}ERROR: pyyaml not installed. Run: pip3 install pyyaml${NC}" >&2
        exit 1
    fi
    if [ ! -f "$CORE" ]; then
        echo -e "${RED}ERROR: Core script not found: $CORE${NC}" >&2
        exit 1
    fi
    if [ ! -f "$MANIFEST" ]; then
        echo -e "${RED}ERROR: Tool manifest not found: $MANIFEST${NC}" >&2
        exit 1
    fi
}

# --- Status: show current tool status from running container ---
show_status() {
    echo ""
    echo -e "${BOLD}OpenCli-Container: Tool Status${NC}"
    echo "==========================="
    echo ""

    # Check container
    if ! $RUNTIME inspect "$CONTAINER" --format '{{.State.Status}}' 2>/dev/null | grep -q "running"; then
        echo -e "${RED}Container not running. Start it first: make start${NC}"
        exit 1
    fi

    # Read current config from container
    local config_json
    config_json=$($RUNTIME exec "$CONTAINER" sh -c "cat /home/vault/.openclaw/openclaw.json 2>/dev/null") || {
        echo -e "${RED}Cannot read config from container.${NC}"
        exit 1
    }

    # Parse with core script
    local status_json
    status_json=$(python3 "$CORE" --manifest "$MANIFEST" --output status --status-json "$config_json" 2>&1) || {
        echo -e "${RED}Failed to parse config: $status_json${NC}"
        exit 1
    }

    # Pretty-print the status
    echo "$status_json" | python3 -c "
import sys, json

s = json.loads(sys.stdin.read())
print(f'  Profile:        {s[\"profile\"]}')
print(f'  Exec security:  {s[\"exec_security\"]}')
print(f'  SafeBins:       {s[\"safeBins_count\"]}')
print(f'  Risk score:     {s[\"risk_score\"]}')
print(f'  Enabled tools:  {s[\"enabled_count\"]} of {len(s[\"tools\"])}')
print()

# Risk colors
colors = {'critical': '\033[1;31m', 'high': '\033[0;31m', 'medium': '\033[0;33m', 'low': '\033[0;36m'}
nc = '\033[0m'
green = '\033[0;32m'
red = '\033[0;31m'

print(f'  {\"Tool\":<22} {\"Risk\":<10} {\"Status\":<10} Description')
print(f'  {\"-\"*22} {\"-\"*10} {\"-\"*10} {\"-\"*40}')

for name, t in s['tools'].items():
    risk_color = colors.get(t['risk'], '')
    if t['status'] == 'ENABLED':
        status_str = f'{green}ENABLED{nc}'
    elif t['status'] == 'NEVER':
        status_str = f'{red}NEVER{nc}  '
    else:
        status_str = f'DENIED '
    print(f'  {name:<22} {risk_color}{t[\"risk\"]:<10}{nc} {status_str}  {t[\"description\"]}')
"
    echo ""
    echo "==========================="
    echo ""
}

# --- Dry-run: generate and display config ---
do_dry_run() {
    local preset="$1"
    shift
    local extra_args=("$@")

    echo ""
    echo -e "${BOLD}OpenCli-Container: Tool Control — Dry Run${NC}"
    echo "======================================="
    echo ""

    # Generate config
    local config_json
    config_json=$(python3 "$CORE" --manifest "$MANIFEST" --preset "$preset" "${extra_args[@]}" --output config 2>&1) || {
        echo -e "${RED}$config_json${NC}"
        exit 1
    }

    # Generate risk assessment
    local risk_json
    risk_json=$(python3 "$CORE" --manifest "$MANIFEST" --preset "$preset" "${extra_args[@]}" --output risk 2>&1) || {
        echo -e "${RED}$risk_json${NC}"
        exit 1
    }

    # Generate allowlist
    local allowlist
    allowlist=$(python3 "$CORE" --manifest "$MANIFEST" --preset "$preset" "${extra_args[@]}" --output allowlist 2>&1) || {
        echo -e "${RED}$allowlist${NC}"
        exit 1
    }

    # Display risk assessment
    echo -e "${BOLD}Risk Assessment${NC}"
    echo "$risk_json" | python3 -c "
import sys, json

r = json.loads(sys.stdin.read())
colors = {'critical': '\033[1;31m', 'high': '\033[0;31m', 'medium': '\033[0;33m', 'low': '\033[0;36m'}
nc = '\033[0m'
green = '\033[0;32m'

print(f'  Risk score: {r[\"risk_score\"]}  ({r[\"enabled_count\"]}/{r[\"total_tools\"]} tools enabled)')
print()

enabled = [(n, t) for n, t in r['tools'].items() if t['status'] == 'ENABLED']
if enabled:
    print(f'  Enabled tools:')
    for name, t in enabled:
        c = colors.get(t['risk'], '')
        print(f'    {c}{t[\"risk\"]:<10}{nc} {name:<22} {t[\"description\"]}')
        for iv in t['injection_vectors']:
            print(f'             \033[0;33m⚠ {iv}{nc}')
    print()
"
    echo ""

    # Display allowlist
    echo -e "${BOLD}Proxy Allowlist${NC}"
    echo "$allowlist" | grep -v "^#" | grep -v "^$" | while read -r domain; do
        echo "    $domain"
    done
    echo ""

    # Display generated config
    echo -e "${BOLD}Generated Config (openclaw.json)${NC}"
    echo "$config_json" | python3 -m json.tool | head -60
    local total_lines
    total_lines=$(echo "$config_json" | python3 -m json.tool | wc -l)
    if [ "$total_lines" -gt 60 ]; then
        echo "    ... ($((total_lines - 60)) more lines)"
    fi

    echo ""
    echo "======================================="
    echo -e "  ${YELLOW}This is a dry run — no changes applied.${NC}"
    echo "  To apply: add --apply instead of --dry-run"
    echo ""
}

# --- Apply: generate config, install it, restart, verify ---
do_apply() {
    local preset="$1"
    shift
    local extra_args=("$@")

    echo ""
    echo -e "${BOLD}OpenCli-Container: Tool Control — Apply${NC}"
    echo "====================================="
    echo ""

    # Detect compose command (only needed when we will restart containers)
    local COMPOSE=""
    if [ "$NO_RESTART" != "true" ]; then
        if command -v "${RUNTIME}-compose" &>/dev/null; then
            COMPOSE="${RUNTIME}-compose"
        elif $RUNTIME compose version &>/dev/null 2>&1; then
            COMPOSE="$RUNTIME compose"
        fi
        if [ -z "$COMPOSE" ]; then
            echo -e "${RED}ERROR: No compose command found.${NC}" >&2
            exit 1
        fi
    fi

    # Step 1: Generate config
    echo "[tool-control] Generating config..."
    local config_json
    config_json=$(python3 "$CORE" --manifest "$MANIFEST" --preset "$preset" "${extra_args[@]}" --output config 2>&1) || {
        echo -e "${RED}Config generation failed: $config_json${NC}"
        exit 1
    }

    # Validate generated JSON
    echo "$config_json" | python3 -c "import sys,json; json.loads(sys.stdin.read())" 2>/dev/null || {
        echo -e "${RED}SECURITY: Generated config is not valid JSON. Aborting.${NC}"
        exit 1
    }

    # Store config integrity hash for tamper detection (verify.sh check #24)
    echo "$config_json" | python3 -c "
import sys, json, hashlib
c = json.loads(sys.stdin.read())
critical = json.dumps({
    'profile': c.get('tools',{}).get('profile'),
    'deny': sorted(c.get('tools',{}).get('deny',[])),
    'exec': c.get('tools',{}).get('exec',{}),
    'elevated': c.get('tools',{}).get('elevated',{}),
}, sort_keys=True)
print(hashlib.sha256(critical.encode()).hexdigest())
" > "$VAULT_DIR/.vault-config-hash" 2>/dev/null
    echo "[tool-control] Config integrity hash stored"

    # Step 2: Generate allowlist
    local allowlist
    allowlist=$(python3 "$CORE" --manifest "$MANIFEST" --preset "$preset" "${extra_args[@]}" --output allowlist 2>&1) || {
        echo -e "${RED}Allowlist generation failed: $allowlist${NC}"
        exit 1
    }

    # Step 3: Generate risk assessment for display
    local risk_json
    risk_json=$(python3 "$CORE" --manifest "$MANIFEST" --preset "$preset" "${extra_args[@]}" --output risk 2>&1) || {
        echo -e "${RED}Risk assessment failed: $risk_json${NC}"
        exit 1
    }

    # Step 4: Show what's about to change and ask for confirmation
    local score
    score=$(echo "$risk_json" | python3 -c "import sys,json; print(json.loads(sys.stdin.read())['risk_score'])")
    local enabled_count
    enabled_count=$(echo "$risk_json" | python3 -c "import sys,json; print(json.loads(sys.stdin.read())['enabled_count'])")

    echo -e "${YELLOW}About to apply:${NC}"
    echo "  Preset: ${preset:-custom}"
    echo "  Risk score: $score"
    echo "  Enabled tools: $enabled_count"
    echo ""
    read -rp "Continue? [y/N] " confirm
    if [ "${confirm,,}" != "y" ]; then
        echo "Cancelled."
        exit 0
    fi

    # Step 5: Write config via podman cp (container must exist, can be stopped)
    # Strategy: use podman cp to write into the stopped container's volume.
    # This avoids volume permission issues with Podman rootless.
    echo ""
    echo "[tool-control] Installing config..."
    local config_tmp
    config_tmp=$(mktemp /tmp/openclaw-config-XXXXXX.json)
    echo "$config_json" > "$config_tmp"
    $RUNTIME cp "$config_tmp" "${CONTAINER}":/home/vault/.openclaw/openclaw.json 2>/dev/null && {
        echo "[tool-control] Config installed via container copy"
    } || {
        if [ "$NO_RESTART" = "true" ]; then
            echo -e "${RED}[tool-control] podman cp to ${CONTAINER} failed.${NC}" >&2
            echo -e "${RED}  --no-restart requires the container to already exist and be writable.${NC}" >&2
            echo -e "${RED}  Caller must ensure ${CONTAINER} is running (or paused) before applying config.${NC}" >&2
            rm -f "$config_tmp"
            exit 1
        fi
        # Fallback: update the baked-in source config and rebuild (standalone use only)
        echo "[tool-control] Container copy failed — updating source config and rebuilding..."
        cp "$config_tmp" "$VAULT_DIR/config/openclaw-hardening.json5"
        $RUNTIME stop "$CONTAINER" vault-proxy 2>/dev/null || true
        $RUNTIME build -t opencli-container -f "$VAULT_DIR/Containerfile" "$VAULT_DIR" 2>&1 | tail -3
        $RUNTIME tag opencli-container opencli-container_vault 2>/dev/null || true
    }
    rm -f "$config_tmp"

    # Step 6: Stop the full stack for clean restart (skipped under --no-restart)
    if [ "$NO_RESTART" != "true" ]; then
        echo "[tool-control] Stopping containers for clean restart..."
        cd "$VAULT_DIR"
        $COMPOSE stop 2>/dev/null || $RUNTIME stop "$CONTAINER" vault-proxy 2>/dev/null || true
    fi

    # Step 7: Write allowlist atomically (temp file + move) — always runs
    echo "[tool-control] Updating proxy allowlist..."
    local allowlist_path="$VAULT_DIR/proxy/allowlist.txt"
    local allowlist_tmp="${allowlist_path}.tmp"
    echo "$allowlist" > "$allowlist_tmp"
    mv -f "$allowlist_tmp" "$allowlist_path"
    echo "[tool-control] Allowlist updated"

    # Under --no-restart we are done. Caller restarts and verifies.
    if [ "$NO_RESTART" = "true" ]; then
        echo ""
        echo -e "${GREEN}${BOLD}[tool-control] Config + allowlist written.${NC}"
        echo "  Container: ${CONTAINER}"
        echo "  Preset: ${preset:-custom}"
        echo "  Risk score: $score"
        echo "  Enabled tools: $enabled_count"
        echo ""
        echo -e "${YELLOW}  --no-restart specified. Caller must restart ${CONTAINER} to apply.${NC}"
        echo ""
        return 0
    fi

    # Step 8: Start the containers
    echo "[tool-control] Starting containers..."
    cd "$VAULT_DIR"
    $COMPOSE up -d 2>/dev/null || {
        echo -e "${RED}Failed to start containers${NC}"
        exit 1
    }

    # Step 9: Wait for gateway
    echo "[tool-control] Waiting for gateway (up to 90s)..."
    for i in $(seq 1 90); do
        if $RUNTIME logs "$CONTAINER" 2>&1 | grep -q "listening on ws://\|OpenClaw"; then
            echo "[tool-control] Gateway ready (${i}s)"
            break
        fi
        sleep 1
    done

    # Step 10: Wait for config to be rewritten as JSON by OpenClaw
    echo "[tool-control] Waiting for config processing..."
    sleep 5

    # Step 11: Run verification
    echo ""
    echo -e "${BOLD}Security verification:${NC}"
    bash "$VAULT_DIR/scripts/verify.sh" 2>&1 | grep -E 'PASS|FAIL|Results|ALL CHECKS'
    local verify_exit=$?
    echo ""

    if [ $verify_exit -eq 0 ]; then
        echo -e "${GREEN}${BOLD}Tool control applied successfully.${NC}"
        echo ""
        echo "  Preset: ${preset:-custom}"
        echo "  Risk score: $score"
        echo "  Enabled tools: $enabled_count"
        echo ""
        echo "  Run 'make tools-status' to see current tool status."
    else
        echo -e "${RED}${BOLD}WARNING: Verification failed after applying config.${NC}"
        echo "  Review the output above. Run 'make verify' for full details."
        exit 1
    fi
    echo ""
}

# --- Parse arguments ---
MODE=""
PRESET=""
EXTRA_ARGS=()
FROM_FILE=""

while [ $# -gt 0 ]; do
    case "$1" in
        --status)
            MODE="status"
            shift
            ;;
        --preset)
            PRESET="$2"
            shift 2
            ;;
        --dry-run)
            MODE="dry-run"
            shift
            ;;
        --apply)
            MODE="apply"
            shift
            ;;
        --no-restart)
            NO_RESTART=true
            shift
            ;;
        --enable)
            EXTRA_ARGS+=("--enable" "$2")
            shift 2
            ;;
        --disable)
            EXTRA_ARGS+=("--disable" "$2")
            shift 2
            ;;
        --from-file)
            FROM_FILE="$2"
            EXTRA_ARGS+=("--from-file" "$2")
            shift 2
            ;;
        --help|-h)
            echo "OpenCli-Container: Tool Control"
            echo ""
            echo "Usage:"
            echo "  $0 --status                              Show current tool status"
            echo "  $0 --preset <name> --dry-run             Preview a preset config"
            echo "  $0 --preset <name> --apply               Apply a preset config"
            echo "  $0 --preset <name> --apply --no-restart  Apply config; caller restarts container"
            echo "  $0 --preset <name> --enable <tool> --dry-run"
            echo "  $0 --preset <name> --disable <tool> --dry-run"
            echo ""
            echo "Presets: hard, split"
            echo ""
            echo "Environment:"
            echo "  OPENCLAW_CONTAINER  Container name to target (default: opencli-container)"
            echo ""
            echo "Examples:"
            echo "  $0 --preset hard --dry-run               Preview Hard Shell"
            echo "  $0 --preset split --dry-run              Preview Split Shell"
            echo "  $0 --preset split --enable web_search --dry-run"
            echo "  $0 --status                              What's currently enabled?"
            echo "  OPENCLAW_CONTAINER=vault-agent $0 --preset split --apply --no-restart"
            exit 0
            ;;
        *)
            echo "Unknown argument: $1. Use --help for usage." >&2
            exit 1
            ;;
    esac
done

# --- Execute ---
check_prerequisites

case "$MODE" in
    status)
        show_status
        ;;
    dry-run)
        if [ -z "$PRESET" ] && [ -z "$FROM_FILE" ]; then
            echo "ERROR: --preset or --from-file required for dry-run." >&2
            exit 1
        fi
        if [ -n "$PRESET" ]; then
            do_dry_run "$PRESET" "${EXTRA_ARGS[@]}"
        else
            do_dry_run "" "${EXTRA_ARGS[@]}"
        fi
        ;;
    apply)
        if [ -z "$PRESET" ] && [ -z "$FROM_FILE" ]; then
            echo "ERROR: --preset or --from-file required for apply." >&2
            exit 1
        fi
        if [ -n "$PRESET" ]; then
            do_apply "$PRESET" "${EXTRA_ARGS[@]}"
        else
            do_apply "" "${EXTRA_ARGS[@]}"
        fi
        ;;
    *)
        echo "ERROR: Specify --status, --dry-run, or --apply. Use --help for usage." >&2
        exit 1
        ;;
esac
