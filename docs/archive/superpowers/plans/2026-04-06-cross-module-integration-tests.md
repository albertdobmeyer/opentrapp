# Cross-Module Integration Tests — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `tests/integration-test.sh` at the lobster-trapp root that validates the operational seams between vault, forge, and pioneer (22 checks across 5 categories).

**Architecture:** A single bash script matching `orchestrator-check.sh` style (colored PASS/FAIL/WARN output, section headers, exit 1 on failure). Tests cross-module data contracts at the file/format level — no running containers needed. Complements the existing 39-check orchestrator validation.

**Tech Stack:** Bash, python3, jq

**Spec:** `docs/superpowers/specs/2026-04-06-cross-module-integration-tests-design.md`

---

## File Structure

| Action | Path | Responsibility |
|--------|------|----------------|
| Create | `tests/integration-test.sh` | All 22 integration checks |
| Modify | `docs/trifecta.md` | Fix stale references (maturity %, pattern counts, implementation status) |

---

### Task 1: Script Skeleton + Submodule Health (Category 4)

**Files:**
- Create: `tests/integration-test.sh`

- [ ] **Step 1: Create integration-test.sh with skeleton and Category 4 checks**

```bash
#!/usr/bin/env bash
# =============================================================================
# Lobster-TrApp Cross-Module Integration Tests
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
# Usage: bash tests/integration-test.sh
# Dependencies: bash, python3, jq
# =============================================================================

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PASS=0
FAIL=0
WARN=0

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

pass() { PASS=$((PASS+1)); echo -e "  ${GREEN}[PASS]${NC} $1"; }
fail() { FAIL=$((FAIL+1)); echo -e "  ${RED}[FAIL]${NC} $1"; }
warn() { WARN=$((WARN+1)); echo -e "  ${YELLOW}[WARN]${NC} $1"; }
section() { echo -e "\n${BLUE}=== $1 ===${NC}"; }

cd "$REPO_ROOT"

VAULT="components/openclaw-vault"
FORGE="components/clawhub-forge"
PIONEER="components/moltbook-pioneer"

# =============================================================================
section "4. Submodule Health"
# =============================================================================

# 4.1 All submodule directories have component.yml
for mod in "$VAULT" "$FORGE" "$PIONEER"; do
  if [ -f "$mod/component.yml" ]; then
    pass "4.1 component.yml exists: $mod"
  else
    fail "4.1 Missing component.yml: $mod"
  fi
done

# 4.2 Submodules are on a branch (not detached HEAD)
for mod in "$VAULT" "$FORGE" "$PIONEER"; do
  name="$(basename "$mod")"
  if git -C "$mod" symbolic-ref HEAD >/dev/null 2>&1; then
    pass "4.2 On branch: $name"
  else
    warn "4.2 Detached HEAD: $name"
  fi
done

# 4.3 Submodule working trees are clean
for mod in "$VAULT" "$FORGE" "$PIONEER"; do
  name="$(basename "$mod")"
  if [ -z "$(git -C "$mod" status --porcelain 2>/dev/null)" ]; then
    pass "4.3 Clean working tree: $name"
  else
    warn "4.3 Uncommitted changes: $name"
  fi
done

# =============================================================================
# Results
# =============================================================================
echo ""
echo -e "Results: ${GREEN}${PASS} passed${NC}, ${RED}${FAIL} failed${NC}, ${YELLOW}${WARN} warnings${NC}"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
```

- [ ] **Step 2: Make executable and run**

Run: `chmod +x tests/integration-test.sh && bash tests/integration-test.sh`
Expected: Category 4 checks pass (3 PASS for component.yml, 3 PASS/WARN for branch status, 3 PASS/WARN for clean trees)

- [ ] **Step 3: Commit**

```bash
git add tests/integration-test.sh
git commit -m "feat: integration-test.sh skeleton with submodule health checks"
```

---

### Task 2: Orchestrator Passthrough (Category 5)

**Files:**
- Modify: `tests/integration-test.sh`

- [ ] **Step 1: Add Category 5 checks before Category 4**

Insert this section after the variable declarations and before `section "4. Submodule Health"`:

```bash
# =============================================================================
section "5. Orchestrator Passthrough"
# =============================================================================

# 5.1 orchestrator-check.sh passes
if bash tests/orchestrator-check.sh >/dev/null 2>&1; then
  pass "5.1 orchestrator-check.sh passes"
else
  fail "5.1 orchestrator-check.sh has failures"
fi

# 5.2 Component roles are correct
for pair in "$VAULT:runtime" "$FORGE:toolchain" "$PIONEER:network"; do
  mod="${pair%%:*}"
  expected_role="${pair##*:}"
  name="$(basename "$mod")"
  actual_role="$(python3 -c "
import yaml, sys
with open('$mod/component.yml') as f:
    data = yaml.safe_load(f)
print(data.get('identity', {}).get('role', ''))
")"
  if [ "$actual_role" = "$expected_role" ]; then
    pass "5.2 Role correct: $name = $expected_role"
  else
    fail "5.2 Role wrong: $name = $actual_role (expected $expected_role)"
  fi
done

```

- [ ] **Step 2: Run and verify**

Run: `bash tests/integration-test.sh`
Expected: Category 5 checks pass (1 for orchestrator-check, 3 for role validation). Category 4 still passes.

- [ ] **Step 3: Commit**

```bash
git add tests/integration-test.sh
git commit -m "feat: add orchestrator passthrough checks (Category 5)"
```

---

### Task 3: Clearance Report Contract (Category 1)

**Files:**
- Modify: `tests/integration-test.sh`

- [ ] **Step 1: Add Category 1 checks after Category 5**

Insert after the Category 5 section:

```bash
# =============================================================================
section "1. Clearance Report Contract (Forge -> Vault)"
# =============================================================================

TEST_SKILL="api-dev"
SKILL_DIR="$FORGE/skills/$TEST_SKILL"
REPORT="$SKILL_DIR/clearance-report.json"

# 1.1 Forge certify produces clearance-report.json
rm -f "$REPORT"
if make -C "$FORGE" certify SKILL="$TEST_SKILL" >/dev/null 2>&1; then
  if [ -f "$REPORT" ]; then
    pass "1.1 Forge certify produces clearance-report.json"
  else
    fail "1.1 Certify succeeded but clearance-report.json missing"
  fi
else
  fail "1.1 Forge certify failed for $TEST_SKILL"
fi

# 1.2 Report has required fields
if [ -f "$REPORT" ]; then
  missing_fields="$(python3 -c "
import json, sys
with open('$REPORT') as f:
    r = json.load(f)
required = {
    'skill': r.get('skill'),
    'version': r.get('version'),
    'scan.status': r.get('scan', {}).get('status'),
    'scan.critical': r.get('scan', {}).get('critical'),
    'verify.verdict': r.get('verify', {}).get('verdict'),
    'checksum': r.get('checksum'),
}
missing = [k for k, v in required.items() if v is None]
if missing:
    print(','.join(missing))
    sys.exit(1)
" 2>&1)" && pass "1.2 Report has all required fields" || fail "1.2 Missing fields: $missing_fields"

  # 1.3 Field types are correct
  python3 -c "
import json, sys
with open('$REPORT') as f:
    r = json.load(f)
errors = []
if not isinstance(r.get('scan', {}).get('status'), str):
    errors.append('scan.status not string')
if not isinstance(r.get('scan', {}).get('critical'), int):
    errors.append('scan.critical not int')
if not isinstance(r.get('checksum', ''), str) or not r.get('checksum', '').startswith('sha256:'):
    errors.append('checksum missing sha256: prefix')
if errors:
    print(', '.join(errors))
    sys.exit(1)
" && pass "1.3 Field types correct" || fail "1.3 Type errors in report"

  # 1.4 SHA-256 checksum matches SKILL.md
  python3 -c "
import json, hashlib, sys
with open('$REPORT') as f:
    r = json.load(f)
expected = r.get('checksum', '')[7:]  # strip 'sha256:'
with open('$SKILL_DIR/SKILL.md', 'rb') as sf:
    actual = hashlib.sha256(sf.read()).hexdigest()
if actual != expected:
    print(f'mismatch: {actual[:12]}... != {expected[:12]}...')
    sys.exit(1)
" && pass "1.4 SHA-256 checksum matches SKILL.md" || fail "1.4 Checksum mismatch"

  # 1.5 Pattern count matches actual scanner
  report_count="$(jq '.scan.pattern_count' "$REPORT")"
  actual_count="$(grep -cE '^\s*"' "$FORGE/tools/lib/patterns.sh" 2>/dev/null || echo 0)"
  # Count patterns by counting pipe-delimited data lines (non-comment, non-empty, contains |)
  actual_count="$(grep -v '^#' "$FORGE/tools/lib/patterns.sh" | grep -v '^\s*$' | grep -c '|')"
  if [ "$report_count" = "$actual_count" ]; then
    pass "1.5 Pattern count matches: $report_count"
  else
    fail "1.5 Pattern count mismatch: report=$report_count actual=$actual_count"
  fi
else
  fail "1.2 Skipped (no report)"
  fail "1.3 Skipped (no report)"
  fail "1.4 Skipped (no report)"
  fail "1.5 Skipped (no report)"
fi

# 1.6 Forge export packages correctly
EXPORT_DIR="$FORGE/exports/$TEST_SKILL"
rm -rf "$EXPORT_DIR"
if make -C "$FORGE" export SKILL="$TEST_SKILL" >/dev/null 2>&1; then
  all_present=true
  for f in SKILL.md clearance-report.json .trust; do
    [ -f "$EXPORT_DIR/$f" ] || all_present=false
  done
  if $all_present; then
    pass "1.6 Export packages SKILL.md + clearance-report.json + .trust"
  else
    fail "1.6 Export missing files in $EXPORT_DIR"
  fi
  rm -rf "$EXPORT_DIR"
else
  fail "1.6 Forge export failed"
fi
```

- [ ] **Step 2: Run and verify**

Run: `bash tests/integration-test.sh`
Expected: Category 1 checks pass (6 PASS). This runs the actual forge certify pipeline, so it will take a few seconds.

- [ ] **Step 3: Commit**

```bash
git add tests/integration-test.sh
git commit -m "feat: add clearance report contract checks (Category 1)"
```

---

### Task 4: Pattern Export Contract (Category 2)

**Files:**
- Modify: `tests/integration-test.sh`

- [ ] **Step 1: Add Category 2 checks after Category 1**

Insert after the Category 1 section:

```bash
# =============================================================================
section "2. Pattern Export Contract (Pioneer -> Vault)"
# =============================================================================

PATTERNS_EXPORT="$PIONEER/data/patterns-export.yml"
PATTERNS_SOURCE="$PIONEER/config/injection-patterns.yml"

# 2.1 Pattern export produces YAML
rm -f "$PATTERNS_EXPORT"
if make -C "$PIONEER" export-patterns >/dev/null 2>&1; then
  if [ -f "$PATTERNS_EXPORT" ]; then
    pass "2.1 Pioneer export-patterns produces YAML"
  else
    fail "2.1 Export succeeded but patterns-export.yml missing"
  fi
else
  fail "2.1 Pioneer export-patterns failed"
fi

if [ -f "$PATTERNS_EXPORT" ]; then
  # 2.2 All patterns have required fields
  python3 -c "
import yaml, sys
with open('$PATTERNS_EXPORT') as f:
    data = yaml.safe_load(f)
patterns = data.get('patterns', [])
errors = []
for i, p in enumerate(patterns):
    for field in ['id', 'severity', 'regex']:
        if field not in p:
            errors.append(f'pattern {i}: missing {field}')
if errors:
    print('; '.join(errors[:5]))
    sys.exit(1)
" && pass "2.2 All patterns have id, severity, regex" || fail "2.2 Missing fields in patterns"

  # 2.3 All regexes compile
  python3 -c "
import yaml, re, sys
with open('$PATTERNS_EXPORT') as f:
    data = yaml.safe_load(f)
errors = []
for p in data.get('patterns', []):
    try:
        re.compile(p['regex'])
    except re.error as e:
        errors.append(f\"{p['id']}: {e}\")
if errors:
    print('; '.join(errors))
    sys.exit(1)
" && pass "2.3 All regexes compile" || fail "2.3 Regex compilation errors"

  # 2.4 Integrity hash is present and valid
  if grep -q '^# Integrity: sha256:' "$PATTERNS_EXPORT"; then
    python3 -c "
import yaml, hashlib, sys
with open('$PATTERNS_EXPORT') as f:
    lines = f.readlines()
# Extract stored hash from header
stored_hash = None
for line in lines:
    if line.startswith('# Integrity: sha256:'):
        stored_hash = line.strip().split('sha256:')[1]
        break
if not stored_hash:
    print('no hash found')
    sys.exit(1)
# Recompute: hash all regex strings sorted by pattern id
data = yaml.safe_load(''.join(lines))
patterns = sorted(data.get('patterns', []), key=lambda p: p['id'])
content = '\n'.join(p['regex'] for p in patterns)
computed = hashlib.sha256(content.encode()).hexdigest()
if computed != stored_hash:
    print(f'mismatch: {computed[:12]}... != {stored_hash[:12]}...')
    sys.exit(1)
" && pass "2.4 Integrity hash valid" || fail "2.4 Integrity hash mismatch"
  else
    fail "2.4 No integrity hash in export"
  fi

  # 2.5 Pattern count matches source config
  export_count="$(python3 -c "
import yaml
with open('$PATTERNS_EXPORT') as f:
    data = yaml.safe_load(f)
print(len(data.get('patterns', [])))
")"
  source_count="$(python3 -c "
import yaml
with open('$PATTERNS_SOURCE') as f:
    data = yaml.safe_load(f)
# Count patterns across all categories
total = 0
for cat in data.get('categories', []):
    total += len(cat.get('patterns', []))
print(total)
")"
  if [ "$export_count" = "$source_count" ]; then
    pass "2.5 Pattern count matches source: $export_count"
  else
    fail "2.5 Count mismatch: export=$export_count source=$source_count"
  fi

  # 2.6 Severity values are valid
  python3 -c "
import yaml, sys
with open('$PATTERNS_EXPORT') as f:
    data = yaml.safe_load(f)
valid = {'CRITICAL', 'HIGH', 'MEDIUM', 'LOW'}
bad = [p['id'] for p in data.get('patterns', []) if p.get('severity') not in valid]
if bad:
    print(f\"invalid severity in: {', '.join(bad)}\")
    sys.exit(1)
" && pass "2.6 All severity values valid" || fail "2.6 Invalid severity values"

  # Cleanup
  rm -f "$PATTERNS_EXPORT"
else
  fail "2.2 Skipped (no export)"
  fail "2.3 Skipped (no export)"
  fail "2.4 Skipped (no export)"
  fail "2.5 Skipped (no export)"
  fail "2.6 Skipped (no export)"
fi
```

- [ ] **Step 2: Run and verify**

Run: `bash tests/integration-test.sh`
Expected: Category 2 checks pass (6 PASS). The pattern export uses python3, so it should be fast.

Note: If the pioneer source YAML format differs from what this script expects (e.g., the YAML structure for counting patterns), the python3 parsing may need adjustment. The `source_count` extraction assumes `categories[].patterns[]` structure — verify this matches the actual `injection-patterns.yml` layout. If it uses a flat list, change to `len(data.get('patterns', []))`.

- [ ] **Step 3: Commit**

```bash
git add tests/integration-test.sh
git commit -m "feat: add pattern export contract checks (Category 2)"
```

---

### Task 5: Cross-Reference Integrity (Category 3)

**Files:**
- Modify: `tests/integration-test.sh`

- [ ] **Step 1: Add Category 3 checks after Category 2**

Insert after the Category 2 section:

```bash
# =============================================================================
section "3. Cross-Reference Integrity"
# =============================================================================

# 3.1 trifecta.md referenced design docs exist
trifecta_ok=true
for ref_path in \
  "components/clawhub-forge/docs/forge-identity-and-design.md" \
; do
  if [ ! -f "$ref_path" ]; then
    fail "3.1 trifecta.md references missing file: $ref_path"
    trifecta_ok=false
  fi
done
$trifecta_ok && pass "3.1 trifecta.md referenced files exist"

# 3.2 Vault skill-install spec fields match forge clearance report
# Vault expects: scan.status, scan.critical, verify.verdict, checksum
# Forge produces these in clearance-report.json — verify by checking the vault spec
vault_spec="$VAULT/docs/specs/2026-03-30-skill-installation-path.md"
if [ -f "$vault_spec" ]; then
  spec_ok=true
  for field in "scan.status" "scan.critical" "verify.verdict" "checksum"; do
    if ! grep -q "$field" "$vault_spec" 2>/dev/null; then
      # Field might be referenced differently in prose
      short="${field##*.}"
      if ! grep -qi "$short" "$vault_spec" 2>/dev/null; then
        fail "3.2 Vault spec missing field reference: $field"
        spec_ok=false
      fi
    fi
  done
  $spec_ok && pass "3.2 Vault spec references match forge clearance fields"
else
  fail "3.2 Vault skill-installation spec missing: $vault_spec"
fi

# 3.3 Feed scanning spec references pioneer pattern format
feed_spec="$VAULT/docs/specs/2026-03-30-feed-scanning-deferred.md"
if [ -f "$feed_spec" ]; then
  if grep -qi "patterns" "$feed_spec" && grep -qi "severity" "$feed_spec"; then
    pass "3.3 Feed scanning spec references pattern format"
  else
    fail "3.3 Feed scanning spec missing pattern/severity references"
  fi
else
  fail "3.3 Feed scanning spec missing: $feed_spec"
fi

# 3.4 Each module has CLAUDE.md and component.yml
all_present=true
for mod in "$VAULT" "$FORGE" "$PIONEER"; do
  name="$(basename "$mod")"
  for f in CLAUDE.md component.yml; do
    if [ ! -f "$mod/$f" ]; then
      fail "3.4 Missing $f in $name"
      all_present=false
    fi
  done
done
$all_present && pass "3.4 All modules have CLAUDE.md and component.yml"

# 3.5 Ownership matrix tools exist
ownership_ok=true
for tool_path in \
  "$FORGE/tools/skill-scan.sh" \
  "$FORGE/tools/skill-verify.sh" \
  "$FORGE/tools/skill-lint.sh" \
  "$PIONEER/tools/feed-scanner.sh" \
  "$PIONEER/tools/agent-census.sh" \
  "$PIONEER/tools/identity-checklist.sh" \
  "$VAULT/scripts/verify.sh" \
  "$VAULT/proxy/vault-proxy.py" \
; do
  if [ ! -f "$tool_path" ]; then
    fail "3.5 Ownership matrix tool missing: $tool_path"
    ownership_ok=false
  fi
done
$ownership_ok && pass "3.5 Ownership matrix tools all exist"
```

- [ ] **Step 2: Run and verify**

Run: `bash tests/integration-test.sh`
Expected: Category 3 checks pass (5 PASS). If any referenced files don't exist, the check correctly fails.

- [ ] **Step 3: Commit**

```bash
git add tests/integration-test.sh
git commit -m "feat: add cross-reference integrity checks (Category 3)"
```

---

### Task 6: Update Stale trifecta.md

**Files:**
- Modify: `docs/trifecta.md`

The integration tests found these stale references in trifecta.md (last updated 2026-03-27). Fix each one.

- [ ] **Step 1: Fix pioneer pattern count**

In `docs/trifecta.md`, change "30 injection patterns" to "25 injection patterns" in the pioneer section (near line 71).

- [ ] **Step 2: Fix verification point count**

Change "23-point security verification" to "24-point security verification" in the vault section (near line 92) and ownership matrix.

- [ ] **Step 3: Fix maturity percentages and next phases**

Update the Current Status table (near line 222):

```markdown
| Module | Maturity | Key Achievement | Next Phase |
|---|---|---|---|
| **openclaw-vault** | 100% | All 3 shell levels operational, 24-point verify, Soft Shell live | Phase 8: Certification |
| **clawhub-forge** | 100% | 87-pattern scanner, CDR pipeline, AI skill creation, 25 skills certified | Phase 5: Deferred (ClawHub API) |
| **moltbook-pioneer** | 100% | 3 tools, 25 patterns, pattern export with ReDoS hardening | Complete |
```

- [ ] **Step 4: Fix integration status table**

Update the Integration Status table (near line 229):

```markdown
### Integration Status

| Integration | Status | Blocking? |
|---|---|---|
| Skill installation path (forge -> vault) | Both sides implemented | Not blocking (manual workflow works) |
| Security certificates (forge -> vault) | Implemented (forge Phase 2) | Not blocking |
| CDR pipeline (forge internal) | Implemented (forge Phase 3) | Not blocking |
| Feed scanning integration (pioneer -> vault) | Designed, deferred | Not blocking (Moltbook domains not in allowlist) |
| GUI discovery (all -> lobster-trapp) | Implemented via component.yml | Not blocking |
| Pattern export (pioneer -> vault) | Pioneer-side complete, vault-side deferred | Not blocking |
```

- [ ] **Step 5: Fix Workflow 1 status**

Update Workflow 1 header (near line 117) from "Not yet implemented" to "Implemented on both sides":

```markdown
### Workflow 1: Skill Installation Path (Forge -> Vault)

**Status:** Implemented on both sides. Forge exports with clearance report, vault validates and installs.
```

- [ ] **Step 6: Fix Workflow 3 monitoring status**

Update Workflow 3 (near line 159). Replace `[NOT YET]` markers:

```markdown
        -> network-log-parser.py (anomaly detection on proxy logs)
        -> session-report.py     (post-session summary)
```

And remove the "What needs to happen" items for monitoring since they're done.

- [ ] **Step 7: Update the document date**

Change `**Updated:** 2026-03-27` to `**Updated:** 2026-04-06`.

- [ ] **Step 8: Run integration tests to confirm fixes**

Run: `bash tests/integration-test.sh`
Expected: All checks pass. Cross-reference checks (Category 3) should still be green after edits.

- [ ] **Step 9: Commit**

```bash
git add docs/trifecta.md
git commit -m "docs: update trifecta.md — all modules complete, fix stale references"
```

---

### Task 7: Final Verification + Spec Commit

- [ ] **Step 1: Run the full integration test suite**

Run: `cd ~/Repositories/lobster-trapp && bash tests/integration-test.sh`
Expected: All 22 checks pass. Output should look like:

```
=== 5. Orchestrator Passthrough ===
  [PASS] 5.1 orchestrator-check.sh passes
  [PASS] 5.2 Role correct: openclaw-vault = runtime
  [PASS] 5.2 Role correct: clawhub-forge = toolchain
  [PASS] 5.2 Role correct: moltbook-pioneer = network

=== 1. Clearance Report Contract (Forge -> Vault) ===
  [PASS] 1.1 Forge certify produces clearance-report.json
  [PASS] 1.2 Report has all required fields
  ...

Results: 22 passed, 0 failed, 0 warnings
```

- [ ] **Step 2: Run orchestrator-check.sh too**

Run: `bash tests/orchestrator-check.sh`
Expected: All 39 checks pass. Confirms the trifecta.md edits didn't break anything.

- [ ] **Step 3: Commit spec document**

```bash
git add docs/superpowers/specs/2026-04-06-cross-module-integration-tests-design.md
git commit -m "docs: add cross-module integration tests design spec"
```

---

## Verification Plan

After all tasks complete:

```bash
cd ~/Repositories/lobster-trapp
bash tests/orchestrator-check.sh    # 39 manifest/structure checks
bash tests/integration-test.sh      # 22 cross-module seam checks
```

Both should exit 0 with all checks passing.

## Implementation Notes

- **Check 1.5 (pattern count):** The pattern count extraction from `patterns.sh` counts pipe-delimited non-comment lines. If forge restructures the pattern file format, this grep will need updating.
- **Check 2.4 (integrity hash):** The hash recomputation assumes the algorithm documented in pioneer's spec (SHA-256 of sorted regex strings joined by newlines). If pioneer changes the hashing method, this check breaks first — which is the intended early warning.
- **Check 2.5 (source count):** The YAML structure for counting source patterns depends on pioneer's `injection-patterns.yml` layout. Verify the python3 parsing matches the actual structure during implementation — it may use `categories[].patterns[]` or a flat `patterns[]` list.
- **trifecta.md edits:** Read the actual file before editing. Line numbers are approximate — content may have shifted since exploration.
