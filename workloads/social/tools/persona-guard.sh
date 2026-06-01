#!/usr/bin/env bash
# Persona-drift outgoing guard (v0.6 M4 §2c).
#
# The OUTGOING mirror of the semantic firewall. Before an agent's post leaves,
# rung-1 drift compares it to the agent's OWN recent voice + task: a hijacked
# agent posting off-character (exfil, spam, a different persona) drifts far from
# its established voice and is HELD for the user. This is the capability the 25
# incoming-only regexes cannot provide (spec 04 §2c) — it guards what the agent
# SENDS, not just what it reads.
#
#   echo "<post>" | persona-guard.sh --history <recent-posts.json> --task "<task>"
#   echo "<post>" | persona-guard.sh --adapter mock --handle <self> --task "<task>" --send
#
# Decision:
#   in-character -> ALLOW (exit 0); with --send, the adapter actually posts.
#   drifted      -> HOLD  (exit 1); never sent — surfaced for the user.
#   can't verify -> HOLD  (exit 3); fail-safe — an unverified post is never
#                  auto-sent (the conservative posture: holding is always safe).
#
# rung-1 only: drift vs the agent's own voice is the RELIABLE embedding signal
# (see sentinel/lib/sentinel_embed.py). User-facing messages stay plain
# language (the banned-vocabulary rule).
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOCIAL_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Resolve the shared rung-1 embedding engine (same candidate pattern the
# firewall uses for the judge). In-container builds stage sentinel/ alongside.
EMBED=""
for cand in "$SOCIAL_ROOT/../../sentinel/embed.sh" "$SOCIAL_ROOT/sentinel/embed.sh" "/opt/sentinel/embed.sh"; do
  [[ -f "$cand" ]] && { EMBED="$cand"; break; }
done

ADAPTER="file"; HANDLE=""; HISTORY=""; TASK=""; SEND=0; POST=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --history) HISTORY="$2"; shift 2 ;;
    --adapter) ADAPTER="$2"; shift 2 ;;
    --handle)  HANDLE="$2"; shift 2 ;;
    --task)    TASK="$2"; shift 2 ;;
    --post)    POST="$2"; shift 2 ;;
    --send)    SEND=1; shift ;;
    *) echo "Unknown arg: $1" >&2; exit 2 ;;
  esac
done

# Post content: --post, or stdin.
[[ -n "$POST" ]] || POST="$(cat)"
[[ -n "${POST// }" ]] || { echo "No post content given." >&2; exit 2; }

hold() { echo "[HOLD] $1"; }

if [[ -z "$EMBED" ]]; then
  hold "Couldn't check this post — the on-device check isn't available — so it was held for your review."
  exit 3
fi

# Resolve the agent's recent voice: an explicit --history file, else fetch the
# agent's own recent posts via the adapter.
HIST_FILE=""; CLEANUP=""
if [[ -n "$HISTORY" ]]; then
  HIST_FILE="$HISTORY"
elif [[ -n "$HANDLE" ]]; then
  ADAPTER_SCRIPT="$SCRIPT_DIR/lib/adapters/${ADAPTER}.sh"
  [[ -f "$ADAPTER_SCRIPT" ]] || { echo "Adapter '$ADAPTER' not found." >&2; exit 2; }
  HIST_FILE="$(mktemp)"; CLEANUP="$HIST_FILE"
  bash "$ADAPTER_SCRIPT" fetch_agent "$HANDLE" > "$HIST_FILE" 2>/dev/null || true
fi
if [[ -z "$HIST_FILE" || ! -s "$HIST_FILE" ]]; then
  [[ -n "$CLEANUP" ]] && rm -f "$CLEANUP"
  hold "No recent activity to compare against, so this post was held for your review."
  exit 3
fi

# rung-1 drift: the post vs the agent's recent voice + task.
DRIFT_JSON="$(printf '%s' "$POST" | bash "$EMBED" drift "$HIST_FILE" "$TASK" 2>/dev/null || true)"
[[ -n "$CLEANUP" ]] && rm -f "$CLEANUP"

SIGNAL="$(echo "$DRIFT_JSON" | jq -r '.signal // empty' 2>/dev/null)"

if [[ -z "$SIGNAL" ]]; then
  hold "Couldn't check this post against your assistant's usual activity, so it was held for your review."
  exit 3
fi

if [[ "$SIGNAL" == "in_character" ]]; then
  echo "[ALLOW] This post matches what your assistant has been doing."
  if [[ "$SEND" -eq 1 ]]; then
    ADAPTER_SCRIPT="$SCRIPT_DIR/lib/adapters/${ADAPTER}.sh"
    if [[ -f "$ADAPTER_SCRIPT" ]] && bash "$ADAPTER_SCRIPT" post "$POST" >/dev/null 2>&1; then
      echo "Sent to the network."
    fi
  fi
  exit 0
fi

# drifted — never sent, even with --send.
hold "This post doesn't match what your assistant has been doing — it was held so you can decide whether to allow it."
exit 1
