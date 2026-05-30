#!/usr/bin/env bash
# OpenCli-Container: One-command setup (Linux/macOS)
# Usage: bash scripts/setup.sh

set -euo pipefail

VAULT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ENV_FILE="$VAULT_DIR/.env"

echo "╔══════════════════════════════════════════════════════╗"
echo "║         OpenCli-Container — Secure Containment          ║"
echo "║    Defense-in-depth sandbox for OpenClaw research    ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""

# --- Detect container runtime ---
RUNTIME=""
if command -v podman &>/dev/null; then
    RUNTIME="podman"
    echo "[+] Detected: Podman (rootless — recommended)"
elif command -v docker &>/dev/null; then
    RUNTIME="docker"
    echo "[+] Detected: Docker"
    echo "    Note: Podman is preferred for rootless operation."
    echo "    Install: https://podman.io/docs/installation"
else
    echo "[!] ERROR: Neither podman nor docker found."
    echo "    Install Podman: https://podman.io/docs/installation"
    echo "    Or Docker:      https://docs.docker.com/get-docker/"
    exit 1
fi

# Check for compose
COMPOSE=""
if command -v "${RUNTIME}-compose" &>/dev/null; then
    COMPOSE="${RUNTIME}-compose"
elif $RUNTIME compose version &>/dev/null 2>&1; then
    COMPOSE="$RUNTIME compose"
else
    echo "[!] ERROR: ${RUNTIME} compose not found."
    echo "    Install: https://docs.docker.com/compose/install/"
    exit 1
fi
echo "[+] Compose: $COMPOSE"

# --- Prompt for API keys ---
echo ""
echo "API keys are stored in $ENV_FILE (gitignored)."
echo "Keys are injected by the proxy sidecar — they NEVER enter the OpenClaw container."
echo ""

if [ -f "$ENV_FILE" ]; then
    echo "[+] Existing .env file found. Using existing keys."
    echo "    Edit $ENV_FILE to change keys."
else
    echo "Enter your Anthropic API key (required):"
    read -rsp "  ANTHROPIC_API_KEY: " ANTHROPIC_KEY
    echo ""

    # Basic key format validation
    if [ -z "$ANTHROPIC_KEY" ]; then
        echo -e "${RED:-}ERROR: API key cannot be empty.${NC:-}"
        exit 1
    fi
    # Strip any accidental whitespace or quotes
    ANTHROPIC_KEY=$(echo "$ANTHROPIC_KEY" | tr -d ' "'"'")

    echo "Enter your OpenAI API key (optional, press Enter to skip):"
    read -rsp "  OPENAI_API_KEY: " OPENAI_KEY
    echo ""

    {
        echo "# OpenCli-Container API keys — NEVER committed to git"
        echo "# These are injected by the mitmproxy sidecar, not the OpenClaw container."
        echo "#"
        echo "# Best practices:"
        echo "#   - Create scoped/restricted API keys for sandbox use only"
        echo "#   - Set hard spending limits on your API provider dashboard"
        echo "#   - Rotate keys regularly"
        printf 'ANTHROPIC_API_KEY=%s\n' "$ANTHROPIC_KEY"
        printf 'OPENAI_API_KEY=%s\n' "$OPENAI_KEY"
    } > "$ENV_FILE"
    chmod 600 "$ENV_FILE"
    echo "[+] API keys saved to $ENV_FILE (mode 600)"
fi

# --- Build the image ---
echo ""
echo "[*] Building opencli-container container image..."
$RUNTIME build -t opencli-container -f "$VAULT_DIR/Containerfile" "$VAULT_DIR"

# --- Start the stack ---
echo ""
echo "[*] Starting vault stack..."
cd "$VAULT_DIR"
$COMPOSE up -d --no-build

echo ""
echo "[+] Vault stack is running."
echo "    Container: opencli-container"
echo "    Proxy:     vault-proxy (mitmproxy on internal network)"
echo ""

# --- Wait for proxy to be ready ---
echo "[*] Waiting for proxy to initialize..."
# Resolve the proxy container by compose service label so this works
# regardless of project name or `container_name:` overrides.
# See docs/specs/2026-05-10-script-container-resolution.md
PROXY_CONTAINER=$($RUNTIME ps -a \
    --filter "label=com.docker.compose.service=vault-proxy" \
    --format '{{.Names}}' 2>/dev/null | head -n 1)
for i in $(seq 1 15); do
    if [ -n "$PROXY_CONTAINER" ] && $RUNTIME exec "$PROXY_CONTAINER" sh -c 'echo OK' &>/dev/null; then
        echo "[+] Proxy is ready."
        break
    fi
    if [ "$i" = "15" ]; then
        echo "[!] Proxy did not become ready in 15 seconds. Running verification anyway..."
    fi
    sleep 1
done

# --- Run verification ---
echo "[*] Running security verification..."
bash "$VAULT_DIR/scripts/verify.sh"

echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║                    VAULT IS READY                    ║"
echo "╠══════════════════════════════════════════════════════╣"
echo "║  Attach: $RUNTIME exec -it opencli-container sh        ║"
echo "║  Logs:   $COMPOSE logs -f                           ║"
echo "║  Stop:   bash scripts/kill.sh --soft                 ║"
echo "║  Nuke:   bash scripts/kill.sh --hard                ║"
echo "╚══════════════════════════════════════════════════════╝"
