#!/usr/bin/env bash
set -euo pipefail

echo "=== Setting up ClawHub Workbench ==="

# Install npm + molthub globally
npm install -g npm@latest
npm install -g molthub

# Install Python pyyaml for frontmatter validation
pip install --quiet pyyaml

# Create wrapper that intercepts 'molthub install' with a warning
# Blocks third-party skill installs unless ALLOW_INSTALL=1 is set
# With ALLOW_INSTALL=1: runs post-install scan and quarantines failures
cat > /usr/local/bin/molthub-safe <<'WRAPPER'
#!/usr/bin/env bash
if [[ "${1:-}" == "install" && "${ALLOW_INSTALL:-0}" != "1" ]]; then
  echo ""
  echo "  BLOCKED: molthub install is disabled in this workbench."
  echo ""
  echo "  11.9% of ClawHub skills were malicious during ClawHavoc."
  echo "  To install anyway: ALLOW_INSTALL=1 molthub install <skill>"
  echo ""
  exit 1
fi

# Post-install scanning when ALLOW_INSTALL=1
if [[ "${1:-}" == "install" && "${ALLOW_INSTALL:-0}" == "1" ]]; then
  LAB_ROOT="${CLAWHUB_FORGE_ROOT:-/workspaces/clawhub-forge}"
  SCANNER="$LAB_ROOT/tools/skill-scan.sh"
  VERIFIER="$LAB_ROOT/tools/skill-verify.sh"
  SKILLS_DIR="$LAB_ROOT/skills"

  # Snapshot skill directories before install
  BEFORE=$(mktemp)
  ls -1d "$SKILLS_DIR"/*/ 2>/dev/null | sort > "$BEFORE" || true

  # Run real molthub install
  molthub "$@"
  INSTALL_RC=$?

  if (( INSTALL_RC != 0 )); then
    rm -f "$BEFORE"
    exit $INSTALL_RC
  fi

  # Detect newly created directories
  AFTER=$(mktemp)
  ls -1d "$SKILLS_DIR"/*/ 2>/dev/null | sort > "$AFTER" || true
  NEW_DIRS=$(comm -13 "$BEFORE" "$AFTER")
  rm -f "$BEFORE" "$AFTER"

  if [[ -z "$NEW_DIRS" ]]; then
    echo "  No new skill directories detected."
    exit 0
  fi

  # Scan each new directory with --strict
  QUARANTINE_DIR="$LAB_ROOT/.quarantine"
  ALL_CLEAN=true

  while IFS= read -r new_dir; do
    [[ -z "$new_dir" ]] && continue
    slug=$(basename "$new_dir")
    echo ""
    echo "  POST-INSTALL SCAN: $slug"

    if [[ -f "$SCANNER" ]]; then
      if bash "$SCANNER" --strict "$new_dir" 2>/dev/null; then
        echo "  PASS: $slug passed blocklist scan"
        # Stage 2: Zero-trust verification (allowlist check)
        if [[ -f "$VERIFIER" ]]; then
          if bash "$VERIFIER" --strict "$new_dir" 2>/dev/null; then
            echo "  VERIFIED: $slug passed zero-trust verification"
          else
            ALL_CLEAN=false
            TIMESTAMP=$(date +%Y%m%d-%H%M%S)
            QUARANTINE_PATH="$QUARANTINE_DIR/${slug}-${TIMESTAMP}"
            mkdir -p "$QUARANTINE_DIR"
            mv "$new_dir" "$QUARANTINE_PATH"
            echo ""
            echo "  QUARANTINED: $slug failed zero-trust verification"
            echo "  Review: bash $VERIFIER --report $QUARANTINE_PATH"
            continue
          fi
        fi
        echo "  CLEAN: $slug passed all checks"
      else
        ALL_CLEAN=false
        TIMESTAMP=$(date +%Y%m%d-%H%M%S)
        QUARANTINE_PATH="$QUARANTINE_DIR/${slug}-${TIMESTAMP}"
        mkdir -p "$QUARANTINE_DIR"
        mv "$new_dir" "$QUARANTINE_PATH"
        echo ""
        echo "  QUARANTINED: $slug moved to $QUARANTINE_PATH"
        echo "  Review findings before use: bash $SCANNER $QUARANTINE_PATH"
      fi
    else
      echo "  WARNING: Scanner not found at $SCANNER — skipping post-install scan"
    fi
  done <<< "$NEW_DIRS"

  if [[ "$ALL_CLEAN" != true ]]; then
    echo ""
    echo "  Some installed skills were quarantined. Review before use."
    exit 1
  fi
  exit 0
fi

exec molthub "$@"
WRAPPER
chmod +x /usr/local/bin/molthub-safe

# Set up alias and environment in shell profile
echo 'alias molthub="molthub-safe"' >> ~/.bashrc
echo 'export CLAWHUB_FORGE_ROOT=/workspaces/clawhub-forge' >> ~/.bashrc

echo ""
echo "  Workbench ready."
echo "  Run 'make help' to see available commands."
echo ""
