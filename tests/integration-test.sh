#!/usr/bin/env bash
# =============================================================================
# OpenTrApp Cross-Module Integration Tests
# =============================================================================
# Validates operational seams between vault, forge, and pioneer:
#   - Clearance report contract (forge -> vault)
#   - Pattern export contract (pioneer -> vault)
#   - Cross-reference integrity
#   - Submodule health
#   - Orchestrator passthrough
#
# Complements orchestrator-check.sh (manifest/structure validation) with
# data contract validation (actual cross-module data flows).
#
# Usage: bash tests/integration-test.sh [--ci]
# Dependencies: bash, python3, jq
# =============================================================================

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

CI_MODE=false
if [[ "${1:-}" == "--ci" ]]; then
  CI_MODE=true
fi

PASS=0
FAIL=0
WARN=0
SKIP=0

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

pass() { PASS=$((PASS+1)); echo -e "  ${GREEN}[PASS]${NC} $1"; }
fail() { FAIL=$((FAIL+1)); echo -e "  ${RED}[FAIL]${NC} $1"; }
warn() { WARN=$((WARN+1)); echo -e "  ${YELLOW}[WARN]${NC} $1"; }
skip() { SKIP=$((SKIP+1)); echo -e "  ${YELLOW}[SKIP]${NC} $1"; }
section() { echo -e "\n${BLUE}=== $1 ===${NC}"; }

cd "$REPO_ROOT"

VAULT="components/opencli-container"
FORGE="components/openskill-forge"
PIONEER="components/openagent-social"

# =============================================================================
section "1. Clearance Report Contract (forge -> vault)"
# =============================================================================

TEST_SKILL="api-dev"
SKILL_DIR="$FORGE/skills/$TEST_SKILL"
REPORT="$SKILL_DIR/clearance-report.json"

# 1.1: Certify produces a clearance report
if make -C "$FORGE" certify SKILL="$TEST_SKILL" > /dev/null 2>&1; then
  if [[ -f "$REPORT" ]]; then
    pass "1.1 make certify produces clearance-report.json"
  else
    fail "1.1 make certify succeeded but clearance-report.json missing"
  fi
else
  fail "1.1 make certify failed for $TEST_SKILL"
fi

# 1.2: Report has required fields
if [[ -f "$REPORT" ]]; then
  FIELDS_OK=$(python3 -c "
import json, sys
with open('$REPORT') as f:
    r = json.load(f)
required = ['skill', 'version', 'checksum']
nested = [('scan','status'), ('scan','critical'), ('verify','verdict')]
ok = True
for k in required:
    if k not in r:
        print(f'missing top-level: {k}', file=sys.stderr)
        ok = False
for section, key in nested:
    if section not in r or key not in r[section]:
        print(f'missing {section}.{key}', file=sys.stderr)
        ok = False
print('OK' if ok else 'FAIL')
" 2>/dev/null)
  if [[ "$FIELDS_OK" == "OK" ]]; then
    pass "1.2 Report has all required fields"
  else
    fail "1.2 Report missing required fields"
  fi
else
  fail "1.2 Report file not available (skipped)"
fi

# 1.3: Report field types are correct
if [[ -f "$REPORT" ]]; then
  TYPES_OK=$(python3 -c "
import json, sys
with open('$REPORT') as f:
    r = json.load(f)
ok = True
if not isinstance(r['scan']['status'], str):
    print('scan.status not string', file=sys.stderr); ok = False
if not isinstance(r['scan']['critical'], int):
    print('scan.critical not int', file=sys.stderr); ok = False
if not isinstance(r['checksum'], str) or not r['checksum'].startswith('sha256:'):
    print('checksum not sha256: prefixed', file=sys.stderr); ok = False
print('OK' if ok else 'FAIL')
" 2>/dev/null)
  if [[ "$TYPES_OK" == "OK" ]]; then
    pass "1.3 Report field types correct (string, int, sha256: prefix)"
  else
    fail "1.3 Report field types incorrect"
  fi
else
  fail "1.3 Report file not available (skipped)"
fi

# 1.4: Checksum matches actual SKILL.md hash
if [[ -f "$REPORT" ]]; then
  CHECKSUM_OK=$(python3 -c "
import json, hashlib
with open('$REPORT') as f:
    r = json.load(f)
stored = r['checksum'].replace('sha256:', '')
with open('$SKILL_DIR/SKILL.md', 'rb') as f:
    actual = hashlib.sha256(f.read()).hexdigest()
print('OK' if stored == actual else 'FAIL')
" 2>/dev/null)
  if [[ "$CHECKSUM_OK" == "OK" ]]; then
    pass "1.4 Checksum matches SKILL.md content"
  else
    fail "1.4 Checksum mismatch (report stale or tampered)"
  fi
else
  fail "1.4 Report file not available (skipped)"
fi

# 1.5: Pattern count matches actual patterns.sh
if [[ -f "$REPORT" ]]; then
  report_count=$(jq '.scan.pattern_count' "$REPORT" 2>/dev/null)
  actual_count="$(grep -v '^#' "$FORGE/tools/lib/patterns.sh" | grep -v '^\s*$' | grep -c '|' || echo 0)"
  if [[ "$report_count" == "$actual_count" ]]; then
    pass "1.5 Pattern count matches ($actual_count patterns)"
  else
    fail "1.5 Pattern count mismatch: report=$report_count, actual=$actual_count"
  fi
else
  fail "1.5 Report file not available (skipped)"
fi

# 1.6: Export packages required files
EXPORT_DIR="$FORGE/exports/$TEST_SKILL"
if make -C "$FORGE" export SKILL="$TEST_SKILL" > /dev/null 2>&1; then
  EXPORT_OK=true
  for f in "$EXPORT_DIR/SKILL.md" "$EXPORT_DIR/clearance-report.json" "$EXPORT_DIR/.trust"; do
    if [[ ! -f "$f" ]]; then
      EXPORT_OK=false
      break
    fi
  done
  if [[ "$EXPORT_OK" == true ]]; then
    pass "1.6 Export contains SKILL.md + clearance-report.json + .trust"
  else
    fail "1.6 Export missing required files"
  fi
  # Clean up
  rm -rf "$EXPORT_DIR"
else
  fail "1.6 make export failed for $TEST_SKILL"
fi

# Clean up certify artifacts
rm -f "$REPORT"

# =============================================================================
section "2. Pattern Export Contract (pioneer -> vault)"
# =============================================================================

PATTERNS_EXPORT="$PIONEER/data/patterns-export.yml"
PATTERNS_SOURCE="$PIONEER/config/injection-patterns.yml"

# 2.1: Export produces the file
if make -C "$PIONEER" export-patterns > /dev/null 2>&1; then
  if [[ -f "$PATTERNS_EXPORT" ]]; then
    pass "2.1 make export-patterns produces patterns-export.yml"
  else
    fail "2.1 make export-patterns succeeded but file missing"
  fi
else
  fail "2.1 make export-patterns failed"
fi

# 2.2: Each exported pattern has required fields
if [[ -f "$PATTERNS_EXPORT" ]]; then
  FIELDS_OK=$(python3 -c "
import yaml, sys
with open('$PATTERNS_EXPORT') as f:
    d = yaml.safe_load(f)
ok = True
for p in d['patterns']:
    for field in ('id', 'severity', 'regex'):
        if field not in p:
            print(f'pattern missing {field}: {p}', file=sys.stderr)
            ok = False
print('OK' if ok else 'FAIL')
" 2>/dev/null)
  if [[ "$FIELDS_OK" == "OK" ]]; then
    pass "2.2 All exported patterns have id, severity, regex"
  else
    fail "2.2 Exported patterns missing required fields"
  fi
else
  fail "2.2 Export file not available (skipped)"
fi

# 2.3: All regexes compile
if [[ -f "$PATTERNS_EXPORT" ]]; then
  REGEX_OK=$(python3 -c "
import yaml, re, sys
with open('$PATTERNS_EXPORT') as f:
    d = yaml.safe_load(f)
ok = True
for p in d['patterns']:
    try:
        re.compile(p['regex'])
    except re.error as e:
        print(f'{p[\"id\"]}: {e}', file=sys.stderr)
        ok = False
print('OK' if ok else 'FAIL')
" 2>/dev/null)
  if [[ "$REGEX_OK" == "OK" ]]; then
    pass "2.3 All exported regexes compile"
  else
    fail "2.3 Some regexes fail to compile"
  fi
else
  fail "2.3 Export file not available (skipped)"
fi

# 2.4: Integrity hash matches
if [[ -f "$PATTERNS_EXPORT" ]]; then
  HEADER_HASH=$(grep '^# Integrity: sha256:' "$PATTERNS_EXPORT" | sed 's/^# Integrity: sha256://')
  if [[ -n "$HEADER_HASH" ]]; then
    HASH_OK=$(python3 -c "
import yaml, hashlib, sys
expected = sys.argv[1]
with open(sys.argv[2]) as f:
    d = yaml.safe_load(f)
regexes = sorted(d['patterns'], key=lambda x: x['id'])
hash_input = '\n'.join(p['regex'] for p in regexes)
computed = hashlib.sha256(hash_input.encode()).hexdigest()
print('OK' if computed == expected else 'FAIL')
" "$HEADER_HASH" "$PATTERNS_EXPORT" 2>/dev/null)
    if [[ "$HASH_OK" == "OK" ]]; then
      pass "2.4 Integrity hash matches exported regexes"
    else
      fail "2.4 Integrity hash mismatch"
    fi
  else
    fail "2.4 No integrity header found in export"
  fi
else
  fail "2.4 Export file not available (skipped)"
fi

# 2.5: Exported count matches source count
if [[ -f "$PATTERNS_EXPORT" ]]; then
  export_count=$(python3 -c "
import yaml
with open('$PATTERNS_EXPORT') as f:
    d = yaml.safe_load(f)
print(len(d['patterns']))
" 2>/dev/null)
  # Source YAML has regex escapes that break PyYAML — count '- id:' lines instead
  source_count=$(grep -c '^\s*- id:' "$PATTERNS_SOURCE" || echo 0)
  if [[ "$export_count" == "$source_count" ]]; then
    pass "2.5 Pattern count matches source ($export_count patterns)"
  else
    fail "2.5 Pattern count mismatch: export=$export_count, source=$source_count"
  fi
else
  fail "2.5 Export file not available (skipped)"
fi

# 2.6: All severities are valid
if [[ -f "$PATTERNS_EXPORT" ]]; then
  SEV_OK=$(python3 -c "
import yaml, sys
with open('$PATTERNS_EXPORT') as f:
    d = yaml.safe_load(f)
valid = {'CRITICAL', 'HIGH', 'MEDIUM', 'LOW'}
ok = True
for p in d['patterns']:
    if p['severity'] not in valid:
        print(f'{p[\"id\"]}: invalid severity {p[\"severity\"]}', file=sys.stderr)
        ok = False
print('OK' if ok else 'FAIL')
" 2>/dev/null)
  if [[ "$SEV_OK" == "OK" ]]; then
    pass "2.6 All severities valid (CRITICAL/HIGH/MEDIUM/LOW)"
  else
    fail "2.6 Invalid severity values found"
  fi
else
  fail "2.6 Export file not available (skipped)"
fi

# Clean up export
rm -f "$PATTERNS_EXPORT"

# =============================================================================
section "3. Cross-Reference Integrity"
# =============================================================================

# 3.1: trifecta.md referenced files exist
trifecta_ok=true
for ref_path in \
  "$FORGE/docs/forge-identity-and-design.md" \
  "GLOSSARY.md" \
  "$VAULT/docs/roadmap.md" \
  "$FORGE/docs/roadmap.md" \
  "$PIONEER/docs/roadmap.md" \
; do
  if [[ ! -f "$ref_path" ]]; then
    fail "3.1 trifecta.md references missing file: $ref_path"
    trifecta_ok=false
  fi
done
if [[ "$trifecta_ok" == true ]]; then
  pass "3.1 trifecta.md referenced files exist"
fi

# 3.2: Vault skill installation spec mentions clearance report fields
SPEC_SKILL="$VAULT/docs/archive/specs/2026-03-30-skill-installation-path.md"
if [[ -f "$SPEC_SKILL" ]]; then
  MENTIONS=0
  for term in "scan" "critical" "verdict" "checksum"; do
    if grep -qi "$term" "$SPEC_SKILL" 2>/dev/null; then
      MENTIONS=$((MENTIONS+1))
    fi
  done
  if [[ "$MENTIONS" -ge 3 ]]; then
    pass "3.2 Skill installation spec references clearance report fields ($MENTIONS/4 terms)"
  else
    fail "3.2 Skill installation spec missing clearance report references ($MENTIONS/4 terms)"
  fi
else
  fail "3.2 Missing: $SPEC_SKILL"
fi

# 3.3: Feed scanning spec mentions patterns and severity
SPEC_FEED="$VAULT/docs/archive/specs/2026-03-30-feed-scanning-deferred.md"
if [[ -f "$SPEC_FEED" ]]; then
  HAS_PATTERNS=false
  HAS_SEVERITY=false
  grep -qi "patterns" "$SPEC_FEED" 2>/dev/null && HAS_PATTERNS=true
  grep -qi "severity" "$SPEC_FEED" 2>/dev/null && HAS_SEVERITY=true
  if [[ "$HAS_PATTERNS" == true && "$HAS_SEVERITY" == true ]]; then
    pass "3.3 Feed scanning spec mentions patterns and severity"
  else
    fail "3.3 Feed scanning spec missing references (patterns=$HAS_PATTERNS, severity=$HAS_SEVERITY)"
  fi
else
  fail "3.3 Missing: $SPEC_FEED"
fi

# 3.4: All modules have CLAUDE.md and component.yml
ALL_MANIFESTS=true
for mod in "$VAULT" "$FORGE" "$PIONEER"; do
  for f in "CLAUDE.md" "component.yml"; do
    if [[ ! -f "$mod/$f" ]]; then
      fail "3.4 Missing: $mod/$f"
      ALL_MANIFESTS=false
    fi
  done
done
if [[ "$ALL_MANIFESTS" == true ]]; then
  pass "3.4 All modules have CLAUDE.md and component.yml"
fi

# 3.5: Ownership matrix — key tools exist in expected locations
TOOLS_OK=true
for f in \
  "$FORGE/tools/skill-scan.sh" \
  "$FORGE/tools/skill-verify.sh" \
  "$FORGE/tools/skill-lint.sh" \
  "$PIONEER/tools/feed-scanner.sh" \
  "$PIONEER/tools/agent-census.sh" \
  "$PIONEER/tools/identity-checklist.sh" \
  "$VAULT/scripts/verify.sh" \
  "$VAULT/proxy/vault-proxy.py"; do
  if [[ ! -f "$f" ]]; then
    fail "3.5 Missing tool: $f"
    TOOLS_OK=false
  fi
done
if [[ "$TOOLS_OK" == true ]]; then
  pass "3.5 Ownership matrix: all key tools present"
fi

# =============================================================================
section "4. Submodule Health"
# =============================================================================

for mod in "$VAULT" "$FORGE" "$PIONEER"; do
  mod_name="$(basename "$mod")"

  # 4.1: component.yml exists
  if [[ -f "$mod/component.yml" ]]; then
    pass "4.1 $mod_name has component.yml"
  else
    fail "4.1 $mod_name missing component.yml"
  fi

  # 4.2: Submodule is on a branch (not detached HEAD)
  if git -C "$mod" symbolic-ref HEAD > /dev/null 2>&1; then
    pass "4.2 $mod_name is on a branch"
  else
    warn "4.2 $mod_name has detached HEAD"
  fi

  # 4.3: Working tree is clean
  if [[ -z "$(git -C "$mod" status --porcelain 2>/dev/null)" ]]; then
    pass "4.3 $mod_name working tree is clean"
  else
    warn "4.3 $mod_name has uncommitted changes"
  fi
done

# =============================================================================
section "5. Orchestrator Passthrough"
# =============================================================================

# 5.1: Run orchestrator-check.sh silently
if [[ "$CI_MODE" == true ]]; then
  skip "5.1 orchestrator-check.sh (runs in separate CI job)"
else
  if bash tests/orchestrator-check.sh > /dev/null 2>&1; then
    pass "5.1 orchestrator-check.sh passes"
  else
    fail "5.1 orchestrator-check.sh failed (run it standalone for details)"
  fi
fi

# 5.2: Each component.yml declares the expected role
ROLE_OK=true
for pair in "opencli-container:runtime" "openskill-forge:toolchain" "openagent-social:network"; do
  comp="${pair%%:*}"
  expected="${pair##*:}"
  actual=$(python3 -c "
import yaml, sys
with open('components/$comp/component.yml') as f:
    d = yaml.safe_load(f)
print(d['identity']['role'])
" 2>/dev/null) || actual=""
  if [[ "$actual" != "$expected" ]]; then
    fail "5.2 $comp role is '$actual', expected '$expected'"
    ROLE_OK=false
  fi
done
if [[ "$ROLE_OK" == true ]]; then
  pass "5.2 All component roles correct (runtime, toolchain, network)"
fi

# =============================================================================
# Results
# =============================================================================
TOTAL=$((PASS + FAIL + WARN + SKIP))
echo ""
echo -e "Results: ${GREEN}${PASS} passed${NC}, ${RED}${FAIL} failed${NC}, ${YELLOW}${WARN} warnings${NC}, ${YELLOW}${SKIP} skipped${NC} (${TOTAL} total)"
if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
exit 0
