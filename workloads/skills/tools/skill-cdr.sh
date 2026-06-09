#!/usr/bin/env bash
# CDR Orchestrator — Content Disarm & Reconstruction pipeline
# Usage:
#   skill-cdr.sh <path-to-SKILL.md>          Local file CDR
#   skill-cdr.sh --download <skill-name>      Download + CDR
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

PY=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || {
  echo "Error: Python not found"; exit 1
}

# Parse arguments
DOWNLOAD_MODE=false
INPUT_PATH=""
SKILL_NAME=""

if [[ "${1:-}" == "--download" ]]; then
  DOWNLOAD_MODE=true
  SKILL_NAME="${2:-}"
  if [[ -z "$SKILL_NAME" ]]; then
    echo "Usage: skill-cdr.sh --download <skill-name>"
    exit 1
  fi
else
  INPUT_PATH="${1:-}"
  if [[ -z "$INPUT_PATH" || ! -f "$INPUT_PATH" ]]; then
    echo "Usage: skill-cdr.sh <path-to-SKILL.md>"
    echo "   or: skill-cdr.sh --download <skill-name>"
    exit 1
  fi
fi

log_header "CDR Pipeline: Content Disarm & Reconstruction"
echo ""

# ── Stage 1: Quarantine ──
echo -e "${BOLD}[1/8] Quarantine${RESET}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

if [[ "$DOWNLOAD_MODE" == true ]]; then
  echo "  Downloading $SKILL_NAME from ClawHub..."
  QUARANTINE_OUTPUT=$(bash "$SCRIPT_DIR/skill-download.sh" "$SKILL_NAME" 2>&1) || {
    echo -e "${RED}  Download failed.${RESET}"
    echo "$QUARANTINE_OUTPUT" | tail -3
    exit 1
  }
  INPUT_PATH=$(echo "$QUARANTINE_OUTPUT" | tail -1)
  QUARANTINE_DIR=$(dirname "$INPUT_PATH")
else
  # Local file — copy to quarantine
  TEMP_NAME=$(basename "$INPUT_PATH" .md | tr '[:upper:]' '[:lower:]' | tr ' ' '-')
  QUARANTINE_DIR="$REPO_ROOT/quarantine/${TEMP_NAME}-${TIMESTAMP}"
  mkdir -p "$QUARANTINE_DIR"
  cp "$INPUT_PATH" "$QUARANTINE_DIR/SKILL.md"
  INPUT_PATH="$QUARANTINE_DIR/SKILL.md"
fi

# Ensure cleanup on exit (success or failure)
cleanup() {
  if [[ -d "${QUARANTINE_DIR:-}" ]]; then
    rm -rf "$QUARANTINE_DIR"
  fi
}
trap cleanup EXIT

echo -e "  ${GREEN}PASS${RESET} — quarantined at $QUARANTINE_DIR/"

# Scan the ORIGINAL (pre-CDR) skill so the disarm diff can report, in plain
# language, what the original contained — the trust artifact the user reads.
ORIGINAL_SCAN="$QUARANTINE_DIR/original-scan.json"
# skill-scan exits NON-ZERO when it finds blocking content, so capture stdout
# unconditionally (|| true) — a non-zero exit here means it found findings,
# which is exactly what the diff needs. Only fall back to empty if no JSON
# actually landed.
bash "$SCRIPT_DIR/skill-scan.sh" --json "$QUARANTINE_DIR" > "$ORIGINAL_SCAN" 2>/dev/null || true
[ -s "$ORIGINAL_SCAN" ] || echo '{"findings":[]}' > "$ORIGINAL_SCAN"

# emit_disarm_diff <--delivered|--quarantined> [dest_dir]
# Prints a plain-language summary of what CDR removed/caught; if a dest_dir is
# given (the delivered skill), also writes it there as DISARM-DIFF.txt.
emit_disarm_diff() {
  local mode="$1" dest="${2:-}"
  echo ""
  echo -e "${BOLD}What I did (disarm summary):${RESET}"
  local diff
  diff=$("$PY" "$SCRIPT_DIR/lib/cdr-diff.py" "$ORIGINAL_SCAN" "$mode" 2>/dev/null || true)
  echo "$diff" | sed 's/^/  /'
  if [[ -n "$dest" && -d "$dest" ]]; then
    printf '%s\n' "$diff" > "$dest/DISARM-DIFF.txt"
  fi
}

# ── Stage 2: Structural Parse ──
echo -e "${BOLD}[2/8] Structural Parse${RESET}"
STRUCTURED_JSON="$QUARANTINE_DIR/structured.json"

if ! "$PY" "$SCRIPT_DIR/lib/cdr-parse.py" "$INPUT_PATH" > "$STRUCTURED_JSON" 2>&1; then
  echo -e "  ${RED}FAIL — could not parse markdown structure${RESET}"
  exit 1
fi
SECTION_COUNT=$("$PY" -c "import json; print(len(json.load(open('$STRUCTURED_JSON')).get('sections',[])))")
echo -e "  ${GREEN}PASS${RESET} — $SECTION_COUNT sections extracted"

# ── Stage 3: Pre-Filter ──
echo -e "${BOLD}[3/8] Pre-Filter${RESET}"
FILTERED_JSON="$QUARANTINE_DIR/filtered.json"

PREFILTER_STDERR=""
if ! bash "$SCRIPT_DIR/lib/cdr-prefilter.sh" "$STRUCTURED_JSON" > "$FILTERED_JSON" 2>/tmp/cdr-prefilter-err.txt; then
  PREFILTER_STDERR=$(cat /tmp/cdr-prefilter-err.txt)
  rm -f /tmp/cdr-prefilter-err.txt
  echo -e "  ${RED}REJECTED — dangerous content detected${RESET}"
  echo "  $PREFILTER_STDERR"
  emit_disarm_diff --quarantined
  exit 1
fi
PREFILTER_STDERR=$(cat /tmp/cdr-prefilter-err.txt 2>/dev/null || true)
rm -f /tmp/cdr-prefilter-err.txt
if [[ -n "$PREFILTER_STDERR" ]]; then
  echo "  $PREFILTER_STDERR"
fi
echo -e "  ${GREEN}PASS${RESET} — safe content extracted"

# ── Stages 4-6: Describe → Validate → Regenerate (with retry-repair) ──
#
# The intent extractor (stage 4) is LLM-backed and non-deterministic: on a
# CLEAN skill it intermittently emits malformed/marginal JSON that fails the
# schema (stage 5) or breaks reconstruction (stage 6). Failing closed on that
# blocks legitimate skills (the ZONE-4a bug). The fix is the spec's
# "describe → validate → regenerate; quarantine on un-describable, never
# silent": retry the describe step with the prior error as a repair hint, and
# only quarantine — explicitly, with a reason — after the budget is exhausted.
#
# This does NOT weaken security: the retry only helps the model produce a
# VALID *clean* description; the reconstruction is still scanned at stage 7,
# and any embedded malice was already stripped at stage 3 (pre-filter). A
# malicious skill can't be "retried" into passing — its content never survives
# the parse → rebuild, and the scan re-runs on the output.
INTENT_JSON="$QUARANTINE_DIR/intent.json"
RECON_DIR="$QUARANTINE_DIR/reconstructed"
mkdir -p "$RECON_DIR"
CDR_MAX_RETRIES="${CDR_MAX_RETRIES:-3}"

echo -e "${BOLD}[4-7/8] Describe → Validate → Regenerate → Verify${RESET}"
cdr_ok=false
repair_hint=""
last_issue=""
for attempt in $(seq 1 "$CDR_MAX_RETRIES"); do
  hint_note=""
  [[ -n "$repair_hint" ]] && hint_note=" (repair attempt $attempt)"

  # Stage 4 — describe (intent extraction), passing any prior error as a hint.
  if ! bash "$SCRIPT_DIR/lib/cdr-intent.sh" "$FILTERED_JSON" "$repair_hint" > "$INTENT_JSON" 2>"$QUARANTINE_DIR/intent.err"; then
    last_issue="intent extraction failed: $(tr '\n' ' ' < "$QUARANTINE_DIR/intent.err" | head -c 300)"
    repair_hint="$last_issue"
    echo -e "  ${YELLOW}retry${RESET} — describe attempt $attempt failed, repairing"
    continue
  fi

  # Stage 5 — validate against the strict schema.
  if ! validation_errors=$("$PY" "$SCRIPT_DIR/lib/cdr-validate.py" "$INTENT_JSON" 2>&1); then
    last_issue="schema validation: $(echo "$validation_errors" | tr '\n' ' ' | head -c 300)"
    repair_hint="$last_issue"
    echo -e "  ${YELLOW}retry${RESET} — schema invalid on attempt $attempt, repairing"
    continue
  fi

  # Stage 6 — regenerate the skill from the validated description only.
  if ! "$PY" "$SCRIPT_DIR/lib/cdr-reconstruct.py" "$INTENT_JSON" "$RECON_DIR/SKILL.md" > "$QUARANTINE_DIR/recon.err" 2>&1; then
    last_issue="reconstruction failed: $(tr '\n' ' ' < "$QUARANTINE_DIR/recon.err" | head -c 300)"
    repair_hint="$last_issue"
    echo -e "  ${YELLOW}retry${RESET} — reconstruction failed on attempt $attempt, repairing"
    continue
  fi

  # Stage 7 (post-verify) is now INSIDE the retry loop: a marginal but CLEAN
  # reconstruction earns a repair attempt instead of a terminal quarantine
  # (the ZONE-4a false-quarantine class). SECURITY: skill-scan + skill-verify
  # still gate DELIVERY (stage 8), and every retry re-derives intent from the
  # SAME already-prefiltered (stage-3, malice-stripped) JSON — so a malicious
  # skill is always either dropped or quarantined, never "retried into passing".
  if ! pv_lint=$(bash "$SCRIPT_DIR/skill-lint.sh" "$RECON_DIR" 2>&1); then
    last_issue="reconstruction failed lint: $(echo "$pv_lint" | grep -iE 'FAIL' | tr '\n' ' ' | head -c 250)"
    repair_hint="$last_issue. Do not include TODO/FIXME/XXX placeholder tokens; write finished content only."
    echo -e "  ${YELLOW}retry${RESET} — reconstruction failed lint on attempt $attempt, repairing"
    continue
  fi
  pv_scan=$(bash "$SCRIPT_DIR/skill-scan.sh" --json "$RECON_DIR" 2>/dev/null) || true
  pv_blocked=$("$PY" -c "import sys,json; print(json.load(sys.stdin).get('blocked',1))" <<< "$pv_scan" 2>/dev/null) || pv_blocked=1
  if [[ "$pv_blocked" != "0" ]]; then
    last_issue="reconstruction tripped the security scanner"
    repair_hint="$last_issue — regenerate without content resembling shell exfiltration, remote downloads, or credential access"
    echo -e "  ${YELLOW}retry${RESET} — reconstruction had scanner findings on attempt $attempt, repairing"
    continue
  fi
  if ! pv_verify=$(bash "$SCRIPT_DIR/skill-verify.sh" "$RECON_DIR" 2>&1); then
    last_issue="reconstruction failed zero-trust verification: $(echo "$pv_verify" | grep -iE 'SUSPICIOUS|MALICIOUS|unrecogni' | tr '\n' ' ' | head -c 250)"
    repair_hint="$last_issue"
    echo -e "  ${YELLOW}retry${RESET} — reconstruction failed zero-trust verify on attempt $attempt, repairing"
    continue
  fi

  cdr_ok=true
  RECON_LINES=$(wc -l < "$RECON_DIR/SKILL.md")
  echo -e "  ${GREEN}PASS${RESET} — described, validated, rebuilt + verified ($RECON_LINES lines)${hint_note}"
  break
done

if [[ "$cdr_ok" != "true" ]]; then
  # Explicit quarantine — NEVER a silent flaky failure (ZONE-4a invariant).
  echo -e "  ${RED}QUARANTINE${RESET} — could not produce a valid clean reconstruction after ${CDR_MAX_RETRIES} attempts."
  echo -e "  Last issue: ${last_issue}"
  echo -e "  This skill is held in quarantine; it was not delivered. If it is a"
  echo -e "  legitimate skill, re-run — the describe step is model-backed and a"
  echo -e "  fresh attempt often succeeds."
  emit_disarm_diff --quarantined
  exit 1
fi

# ── Stage 7: Post-Verify (performed inside the describe→regenerate loop above) ──
# Reaching here means lint + scan + verify already passed on the reconstruction
# within the retry budget; a failure would have retried or quarantined above.
echo -e "${BOLD}[7/8] Post-Verification${RESET}"
echo -e "  ${GREEN}PASS${RESET} — lint + scan + verify all clean"

# ── Stage 8: Deliver + Cleanup ──
echo -e "${BOLD}[8/8] Deliver${RESET}"

# Extract skill name from reconstructed frontmatter
CDR_SKILL_NAME=$("$PY" -c "
import sys
with open('$RECON_DIR/SKILL.md') as f:
    for line in f:
        line = line.strip()
        if line.startswith('name:'):
            print(line.split(':', 1)[1].strip())
            break
" 2>/dev/null)

if [[ -z "$CDR_SKILL_NAME" ]]; then
  echo -e "  ${RED}FAIL — could not extract skill name from reconstruction${RESET}"
  exit 1
fi

DEST_DIR="$REPO_ROOT/skills/$CDR_SKILL_NAME"
if [[ -d "$DEST_DIR" ]]; then
  echo "  Skill '$CDR_SKILL_NAME' already exists — overwriting with CDR'd version."
fi

mkdir -p "$DEST_DIR"
cp "$RECON_DIR/SKILL.md" "$DEST_DIR/SKILL.md"

# Generate trust file
bash "$SCRIPT_DIR/skill-verify.sh" --trust "$DEST_DIR" > /dev/null 2>&1 || true

echo -e "  ${GREEN}PASS${RESET} — delivered to skills/$CDR_SKILL_NAME/"

# Disarm diff — the trust artifact. Tells the user, in plain language, what the
# rebuild did and what (if anything) the original contained. Saved alongside
# the delivered skill so the GUI/agent can surface it.
emit_disarm_diff --delivered "$DEST_DIR"

# Cleanup happens via trap
echo ""
echo -e "${GREEN}CDR complete: $CDR_SKILL_NAME${RESET}"
echo "  Run 'make certify SKILL=$CDR_SKILL_NAME' to generate security certificate."
