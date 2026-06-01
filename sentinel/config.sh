#!/usr/bin/env bash
# Sentinel — shared configuration for the local judgment layer.
#
# Sourced by judge.sh and by any shield that embeds the Sentinel lib. Values
# can be overridden by the environment or by a per-deployment sentinel.conf.
# Defaults reuse the local Ollama the skills CDR pipeline already depends on —
# no new dependency tier (the v0.6 anti-bloat contract).

# Rung-2 JUDGE model. qwen2.5-coder:3b (~1.9 GB) — empirically the smallest
# local model with adequate gray-zone precision: it allows a benign command
# shown as a documentation example (5/5 in testing) while still blocking
# exfiltration and resisting injection of the judge itself. The 1.5b
# over-blocked the benign gray zone (the old D3 limitation); 3b resolves it and
# still fits alongside the user's agent. Local, no API key, zero marginal cost.
# NOTE: the everyday PARSER (CDR "describe" step) stays on the leaner 1.5b
# (config/cdr.conf CDR_MODEL) — parsing failures are schema-detectable and
# retry-recoverable, so the tiniest model suffices there; judgment is not, so
# it gets the bigger model. Override either via the environment.
SENTINEL_MODEL="${SENTINEL_MODEL:-qwen2.5-coder:3b}"
SENTINEL_ENDPOINT="${SENTINEL_ENDPOINT:-http://localhost:11434/api/generate}"
SENTINEL_TIMEOUT="${SENTINEL_TIMEOUT:-60}"

# Confidence floor below which rung 2 escalates instead of deciding (the
# alert-fatigue budget — keep this conservative; escalation must be rare).
SENTINEL_ESCALATE_BELOW="${SENTINEL_ESCALATE_BELOW:-0.35}"
