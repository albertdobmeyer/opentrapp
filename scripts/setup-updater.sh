#!/usr/bin/env bash
# =============================================================================
# OpenTrApp Updater Signing Key Setup
# =============================================================================
# Generates a signing keypair for the Tauri updater and configures the project.
#
# What this does:
#   1. Generates a signing keypair using tauri signer
#   2. Updates tauri.conf.json with the public key and activates the updater
#   3. Prints instructions for storing the private key in GitHub Secrets
#
# Prerequisites:
#   - Node.js installed
#   - npm dependencies installed (cd app && npm ci)
#
# Usage: bash scripts/setup-updater.sh
# =============================================================================

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CONF="$REPO_ROOT/app/src-tauri/tauri.conf.json"
KEY_DIR="$HOME/.tauri"
KEY_FILE="$KEY_DIR/opentrapp.key"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== OpenTrApp Updater Key Setup ===${NC}"
echo ""

# Check prerequisites
if ! command -v npx &> /dev/null; then
  echo -e "${RED}Error: npx not found. Install Node.js first.${NC}"
  exit 1
fi

if [[ -f "$KEY_FILE" ]]; then
  echo -e "${YELLOW}Warning: Key file already exists at $KEY_FILE${NC}"
  echo "To regenerate, delete it first: rm $KEY_FILE"
  exit 1
fi

# Generate keypair
echo -e "${BLUE}Step 1/3: Generating signing keypair...${NC}"
echo "You will be prompted for a password. Remember it — you'll need it for GitHub Secrets."
echo ""

mkdir -p "$KEY_DIR"
cd "$REPO_ROOT/app"
npx tauri signer generate -w "$KEY_FILE"

echo ""

# Extract public key from the generate output
# The public key is stored in KEY_FILE.pub
PUB_KEY_FILE="${KEY_FILE}.pub"
if [[ -f "$PUB_KEY_FILE" ]]; then
  PUBKEY=$(cat "$PUB_KEY_FILE")
elif [[ -f "$KEY_FILE" ]]; then
  echo -e "${YELLOW}Note: Public key file not found at ${PUB_KEY_FILE}${NC}"
  echo "The public key was printed above. Copy it and enter it here:"
  read -r PUBKEY
else
  echo -e "${RED}Error: Key generation failed — no key file at $KEY_FILE${NC}"
  exit 1
fi

# Update tauri.conf.json
echo -e "${BLUE}Step 2/3: Updating tauri.conf.json...${NC}"

python3 -c "
import json
with open('$CONF') as f:
    conf = json.load(f)
conf['plugins']['updater']['active'] = True
conf['plugins']['updater']['pubkey'] = '$PUBKEY'
with open('$CONF', 'w') as f:
    json.dump(conf, f, indent=2)
    f.write('\n')
"

echo -e "${GREEN}Updated: updater.active = true, pubkey set${NC}"

# Instructions for GitHub Secrets
echo ""
echo -e "${BLUE}Step 3/3: Store private key in GitHub Secrets${NC}"
echo ""
echo "Go to: https://github.com/albertdobmeyer/opentrapp/settings/secrets/actions"
echo ""
echo "Create two secrets:"
echo ""
echo -e "  ${GREEN}TAURI_SIGNING_PRIVATE_KEY${NC}"
echo "  Value: contents of $KEY_FILE"
echo "  (run: cat $KEY_FILE | pbcopy   or   cat $KEY_FILE | xclip)"
echo ""
echo -e "  ${GREEN}TAURI_SIGNING_PRIVATE_KEY_PASSWORD${NC}"
echo "  Value: the password you entered above"
echo ""
echo -e "${YELLOW}IMPORTANT: Never commit the private key file.${NC}"
echo "The file at $KEY_FILE should stay local."
echo ""
echo -e "${GREEN}Done! The updater is configured.${NC}"
echo "To test: push a tag (e.g., git tag v0.1.0-rc.1 && git push --tags)"
echo "CI will produce a draft release with binaries and latest.json."
