#!/usr/bin/env bash
# Sentinel — egress advisor (adaptive containment, v0.6 M3).
#
# Reads the proxy's persistent egress log (requests.jsonl, the Zone-3 fix) plus
# the agent's current shell level, and proposes the SMALLEST shell that still
# covers the agent's observed behaviour — "least-privilege, discovered not
# configured".
#
#   egress-advisor.sh --log <requests.jsonl> --shell <hard|split|soft>
#
# Output (stdout): {"proposal":"tighten_to_<level>"|"no_change","reason":"..."}
#
# HARD INVARIANT (ADR-0002): this advisor can ONLY ever propose TIGHTENING or
# NO CHANGE. It is structurally incapable of proposing a loosening — loosening
# always requires an explicit human tap elsewhere, never an automatic proposal.
# The invariant is enforced below (a proposed level is clamped to never be
# looser than the current level) and pinned by egress-advisor.test.sh.
set -euo pipefail

LOG=""
SHELL_LEVEL=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --log) LOG="$2"; shift 2 ;;
    --shell) SHELL_LEVEL="$2"; shift 2 ;;
    *) echo "Unknown arg: $1" >&2; exit 2 ;;
  esac
done

# Shell ordering, tightest → loosest. Rank: hard=0 (tightest), soft=2 (loosest).
shell_rank() { case "$1" in hard) echo 0;; split) echo 1;; soft) echo 2;; *) echo 2;; esac; }

if [[ -z "$SHELL_LEVEL" ]]; then
  echo '{"proposal":"no_change","reason":"No current shell level was provided."}'; exit 0
fi
CUR_RANK=$(shell_rank "$SHELL_LEVEL")

# Hosts that are part of the agent's core reasoning/chat — reaching only these
# means the agent has not needed file/web tools (fits the tightest shell).
CORE_HOSTS_RE='api\.anthropic\.com|api\.openai\.com|api\.telegram\.org'
# Hosts that indicate file/skill work but not open web (fits the middle shell).
WORK_HOSTS_RE='raw\.githubusercontent\.com'

# Classify the observed behaviour from the log → the MINIMAL shell that covers it.
# Default (no log / empty) → assume the tightest, so we only ever suggest
# tightening, never loosening.
observed_min="hard"
if [[ -n "$LOG" && -f "$LOG" ]]; then
  # Distinct allowed destination hosts the agent actually reached.
  hosts=$(grep -oE '"host"[: ]*"[^"]*"' "$LOG" 2>/dev/null | sed -E 's/.*"host"[: ]*"([^"]*)".*/\1/' | sort -u || true)
  saw_other=false; saw_work=false
  while IFS= read -r h; do
    [[ -z "$h" ]] && continue
    if echo "$h" | grep -qE "$CORE_HOSTS_RE"; then
      continue
    elif echo "$h" | grep -qE "$WORK_HOSTS_RE"; then
      saw_work=true
    else
      saw_other=true
    fi
  done <<< "$hosts"
  if $saw_other; then observed_min="soft"
  elif $saw_work; then observed_min="split"
  else observed_min="hard"; fi
fi
OBS_RANK=$(shell_rank "$observed_min")

# Propose the looser of (observed_min, ... ) — but clamp so the proposal is
# NEVER looser than the current shell. This is the never-auto-loosen invariant:
# if observed_min is looser than current, we keep current (no_change); we only
# ever move toward the tighter end.
if (( OBS_RANK < CUR_RANK )); then
  # The agent used less than its current shell grants → propose tightening.
  printf '{"proposal":"tighten_to_%s","reason":"Your assistant has only needed %s-level access recently, so it can run with less. You can widen it again any time."}\n' \
    "$observed_min" "$observed_min"
else
  # Observed behaviour needs the current shell (or more, which we never grant
  # automatically) → no change.
  echo '{"proposal":"no_change","reason":"Your assistant is using about the access it currently has."}'
fi
