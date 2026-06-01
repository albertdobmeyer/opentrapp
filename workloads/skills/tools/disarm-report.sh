#!/usr/bin/env bash
# Disarm report — the skills trust artifact, GUI surface (v0.6).
#
# Lists the skills the cleanroom (CDR) has processed and what each disarm
# removed, by reading the DISARM-DIFF.txt files saved alongside delivered
# skills (skill-cdr.sh writes one per delivered skill). READ-ONLY: it surfaces
# what CDR ALREADY did — it adds no new capability and touches no policy.
#
# Runs inside vault-skills (where delivered skills + their DISARM-DIFF.txt live)
# and is exposed to the GUI via the `cleaned-skills` manifest command — the
# in-architecture channel (generic backend runs it; the frontend renders the
# JSON), so the host never reads a container volume directly and the untrusted
# agent never feeds the host a path.
#
#   disarm-report.sh [--root <skills-dir>]   (default: ./skills, relative to the module)
#
# Output (stdout, JSON):
#   {"cleaned":[{"skill":"<name>","report":"<plain-language text>"}],"count":N}
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODULE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT="$MODULE_ROOT/skills"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --root) ROOT="$2"; shift 2 ;;
    *) echo "Unknown arg: $1" >&2; exit 2 ;;
  esac
done

PY=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || {
  echo '{"cleaned":[],"count":0}'; exit 0
}

"$PY" - "$ROOT" <<'PYEOF'
import json, sys, pathlib

root = pathlib.Path(sys.argv[1])
cleaned = []
if root.is_dir():
    for diff in sorted(root.glob("*/DISARM-DIFF.txt")):
        try:
            text = diff.read_text(encoding="utf-8", errors="replace").strip()
        except Exception:
            continue
        cleaned.append({"skill": diff.parent.name, "report": text})

print(json.dumps({"cleaned": cleaned, "count": len(cleaned)}))
PYEOF
