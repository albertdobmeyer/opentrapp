#!/usr/bin/env bash
# Sentinel rung-1 — embeddings CLI (similarity / anomaly / drift).
#
# The cheap, always-affordable layer between rung 0 (static regex) and rung 2
# (the tiny LLM judge). Lib-first, exactly like judge.sh: any shield embeds it
# and calls it directly against local Ollama, with no GUI or parent app — the
# v0.6 lib-first design (docs/specs/v0.6/01-sentinel-spine.md §5).
#
# Subcommands (text to judge on stdin):
#   vector                       -> {"model","dim","vector":[...]}
#   build-corpus OUT FIELD IN... -> embed each item[FIELD] from JSON arrays IN...
#   score CORPUS                 -> {"max_similarity","nearest_ref","signal"}
#   drift HISTORY [TASK_HINT]    -> {"similarity","drift","signal"}
#
# Exit 3 if the local embed engine is unreachable (caller picks its policy).
# See sentinel/lib/sentinel_embed.py for the full contract + the recall caveat.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config.sh"

PY=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || {
  echo '{"error":"No Python runtime available to run the local similarity engine."}'; exit 3
}

export SENTINEL_EMBED_MODEL SENTINEL_EMBED_ENDPOINT SENTINEL_EMBED_TIMEOUT \
       SENTINEL_SIM_HIGH SENTINEL_SIM_LOW SENTINEL_DRIFT_SIM_MIN

exec "$PY" "$SCRIPT_DIR/lib/sentinel_embed.py" "$@"
