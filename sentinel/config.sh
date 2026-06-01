#!/usr/bin/env bash
# Sentinel — shared configuration for the local judgment layer.
#
# Sourced by judge.sh and by any shield that embeds the Sentinel lib. Values
# can be overridden by the environment or by a per-deployment sentinel.conf.
# Defaults reuse the local Ollama the skills CDR pipeline already depends on —
# no new dependency tier (the v0.6 anti-bloat contract).

# Rung-2 model. The spec default is a sub-1B model; we ship qwen2.5-coder:1.5b
# as the available default (it is what CDR already uses) and let the user
# downsize/upsize via the environment. Local, no API key, zero marginal cost.
SENTINEL_MODEL="${SENTINEL_MODEL:-qwen2.5-coder:1.5b}"
SENTINEL_ENDPOINT="${SENTINEL_ENDPOINT:-http://localhost:11434/api/generate}"
SENTINEL_TIMEOUT="${SENTINEL_TIMEOUT:-60}"

# Confidence floor below which rung 2 escalates instead of deciding (the
# alert-fatigue budget — keep this conservative; escalation must be rare).
SENTINEL_ESCALATE_BELOW="${SENTINEL_ESCALATE_BELOW:-0.35}"
