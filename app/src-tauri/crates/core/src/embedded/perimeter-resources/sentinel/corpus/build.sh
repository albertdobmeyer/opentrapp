#!/usr/bin/env bash
# Rebuild the Sentinel rung-1 known-bad corpus from the repo fixtures.
#
# This is a cheap re-embed (NOT a retrain): adding a new known-bad example and
# re-running this script updates the corpus in seconds. The corpus is tagged
# with the embedding model; `score` refuses a corpus built with a different
# model, so a model change forces a rebuild. Safe to run any time Ollama is up.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/../.." && pwd)"
EMBED="$REPO/sentinel/embed.sh"

# Seed: the literal known-bad social posts. The corpus is the SIMILARITY anchor
# for rung-1 anomaly scoring — paraphrases that echo these score higher; novel
# attacks may not (the recall caveat), which is why `score` never gates rung 2.
"$EMBED" build-corpus "$SCRIPT_DIR/known-bad.json" content \
  "$REPO/workloads/social/tests/fixtures/malicious-posts.json"

echo "Sentinel rung-1 corpus rebuilt at $SCRIPT_DIR/known-bad.json"
