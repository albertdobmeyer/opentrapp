#!/usr/bin/env bash
# Build the published Skill Firewall projection repo (opentrapp/skill-firewall).
#
# This is the ONE source for both the first population and the automated release
# sync. It assembles a self-contained, single-action, offline-only copy of the
# Skill Firewall scanner with action.yml at the repository ROOT, which is what the
# GitHub Marketplace requires (the monorepo cannot be listed because its action
# lives in a subdirectory). The monorepo stays the source of truth; this output is
# a generated projection (one way), so the repo carries a "do not edit" banner.
#
# Only the offline Tier A engine (scan + verify) is vendored. The model-backed CDR
# (Tier B) is deliberately left out so the published action ships nothing that
# touches a network or a model.
#
# Usage: scripts/build-skill-firewall-projection.sh [OUT_DIR]
#   OUT_DIR defaults to /tmp/skill-firewall-build
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/workloads/skills"
ACT="$ROOT/actions/skill-scan"
OUT="${1:-/tmp/skill-firewall-build}"

# The exact offline scan + verify dependency set. Nothing else is shipped.
FILES=(
  "skill"
  "tools/skill-scan.sh"
  "tools/skill-verify.sh"
  "tools/lib/common.sh"
  "tools/lib/patterns.sh"
  "tools/lib/line-classifier.sh"
  "tools/lib/trust-manifest.sh"
  "tools/lib/sarif_formatter.py"
)

echo "Building Skill Firewall projection into: $OUT"
rm -rf "$OUT"
mkdir -p "$OUT"

# 1. Vendor the scanner allowlist (preserving the skill + tools/ layout the
#    dispatcher expects, since it resolves tools/ relative to its own location).
for f in "${FILES[@]}"; do
  mkdir -p "$OUT/$(dirname "$f")"
  cp "$SRC/$f" "$OUT/$f"
done
chmod +x "$OUT/skill"

# 2. Project action.yml to the repo root: the scanner now sits at the action's own
#    path, and the "how to reference me" error message names the published repo.
sed -e 's|/../../workloads/skills/skill|/skill|' \
    -e 's|albertdobmeyer/opentrapp/actions/skill-scan@|opentrapp/skill-firewall@|g' \
    "$ACT/action.yml" > "$OUT/action.yml"

# 3. Project the README: rewrite only the `uses:` ref (other links correctly point
#    back at the source project) and prepend a provenance banner.
{
  cat <<'BANNER'
> Generated, do not edit here. This repository is the published, Marketplace
> listed projection of the OpenTrApp Skill Firewall action. The source of truth,
> the full five-container perimeter, the issue tracker, and the tests all live in
> [albertdobmeyer/opentrapp](https://github.com/albertdobmeyer/opentrapp). This
> repo is regenerated one way from there; open issues and PRs against the source.

BANNER
  sed 's|albertdobmeyer/opentrapp/actions/skill-scan@skill-scan-v1|opentrapp/skill-firewall@v1|g' "$ACT/README.md"
} > "$OUT/README.md"

# 4. License (same MIT as the source).
cp "$ROOT/LICENSE" "$OUT/LICENSE"

# 5. A known-clean example skill so the repo can dogfood its own action in CI.
mkdir -p "$OUT/examples/clean-skill"
cat > "$OUT/examples/clean-skill/SKILL.md" <<'SKILL'
---
name: hello-clean
description: A minimal benign skill used to dogfood the Skill Firewall action in CI.
---

# Hello, clean skill

This skill does nothing dangerous. It exists so the action can scan a known clean
input on every push and prove the gate passes. A malicious skill in its place would
fail the job, which is the whole point.
SKILL

# 6. The repo's own self-scan workflow: dogfoods the root action end to end, which
#    is the only place a GitHub Action can actually be exercised.
mkdir -p "$OUT/.github/workflows"
cat > "$OUT/.github/workflows/self-scan.yml" <<'WF'
name: self-scan

# Dogfoods this repository's own action against a known clean example skill on every
# push and PR. It verifies the action end to end (the only place a GitHub Action can
# be exercised) and doubles as the canonical usage example.
on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read
  security-events: write   # upload SARIF findings to the Security tab

jobs:
  self-scan:
    name: Scan a known-clean skill with the Skill Firewall
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd # v6.0.2
      - name: OpenTrApp Skill Firewall
        uses: ./
        with:
          path: examples
          strict: false
WF

# 7. Provenance / regeneration note for maintainers.
cat > "$OUT/PROVENANCE.md" <<'PROV'
# Provenance

This repository is a generated, one-way projection of the Skill Firewall scanner
that lives in [albertdobmeyer/opentrapp](https://github.com/albertdobmeyer/opentrapp)
under `workloads/skills/` (the engine) and `actions/skill-scan/` (the action
metadata and README).

It exists only because a GitHub Marketplace listing requires a single action with
`action.yml` at the repository root, which the monorepo cannot provide. The monorepo
remains the source of truth.

## Regenerate

From a checkout of the source repo:

```bash
scripts/build-skill-firewall-projection.sh /tmp/skill-firewall-build
```

then sync `/tmp/skill-firewall-build` into this repository. In CI this is automated
on every `skill-scan-v*` tag (see `.github/workflows/sync-skill-firewall.yml` in the
source repo). Do not hand-edit files here; edit them in the source repo and let the
projection regenerate.

## What is and is not vendored

Only the offline Tier A engine is shipped: `scan` and `verify`. The model-backed
Content Disarm and Reconstruction (Tier B) is intentionally excluded so this
published action contains nothing that touches a network or a model.
PROV

echo "Done. Contents:"
( cd "$OUT" && find . -type f | sort | sed 's|^\./|  |' )
