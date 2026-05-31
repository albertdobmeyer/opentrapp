#!/usr/bin/env bash
# 12-point workbench verification — proves the system works
# Usage: workbench-verify.sh
# Exit 1 if any critical check fails
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/common.sh"

CACHE_DIR="$REPO_ROOT/.workbench-cache"
TODAY=$(date +%Y%m%d)
PYTHON=$(command -v python3 2>/dev/null || command -v python 2>/dev/null || echo "")
SKILLS_DIR="$REPO_ROOT/skills"
TESTS_DIR="$REPO_ROOT/tests"
TEMPLATES_DIR="$REPO_ROOT/templates"

TOTAL=12
PASSED=0
FAILED=0
WARNED=0

check_pass() { PASSED=$((PASSED + 1)); log_pass "$1"; }
check_fail() { FAILED=$((FAILED + 1)); log_fail "$1"; }
check_warn() { WARNED=$((WARNED + 1)); log_warn "$1"; }

log_header "12-Point Workbench Verification"
echo ""

# ─── Check 1: All skills have SKILL.md ───
missing_skill=0
for dir in "$SKILLS_DIR"/*/; do
  [[ ! -f "$dir/SKILL.md" ]] && { missing_skill=1; break; }
done
if [[ $missing_skill -eq 0 ]]; then
  skill_count=$(find "$SKILLS_DIR" -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')
  check_pass "[1/12] All skill directories have SKILL.md ($skill_count skills)"
else
  check_fail "[1/12] Some skill directories missing SKILL.md"
fi

# ─── Check 2: All skills have test files ───
missing_tests=()
for dir in "$SKILLS_DIR"/*/; do
  slug=$(basename "$dir")
  [[ ! -f "$TESTS_DIR/${slug}.test.sh" ]] && missing_tests+=("$slug")
done
if [[ ${#missing_tests[@]} -eq 0 ]]; then
  test_count=$(find "$TESTS_DIR" -maxdepth 1 -name "*.test.sh" | wc -l | tr -d ' ')
  check_pass "[2/12] All skills have test files ($test_count tests)"
else
  check_fail "[2/12] Missing test files: ${missing_tests[*]}"
fi

# ─── Check 3: Lint passes on all skills ───
if bash "$REPO_ROOT/tools/skill-lint.sh" "$SKILLS_DIR" > /dev/null 2>&1; then
  check_pass "[3/12] Lint passes on all skills"
else
  check_fail "[3/12] Lint has failures"
fi

# ─── Check 4: Scan passes on all skills ───
if bash "$REPO_ROOT/tools/skill-scan.sh" "$SKILLS_DIR" > /dev/null 2>&1; then
  check_pass "[4/12] Scan passes on all skills (no unresolved CRITICAL findings)"
else
  check_fail "[4/12] Scan has unresolved CRITICAL findings"
fi

# ─── Check 5: All tests pass ───
if bash "$REPO_ROOT/tools/skill-test.sh" > /dev/null 2>&1; then
  check_pass "[5/12] All behavioral tests pass"
else
  check_fail "[5/12] Some behavioral tests fail"
fi

# ─── Check 6: Scanner self-test passes ───
if bash "$TESTS_DIR/scanner-self-test/run.sh" > /dev/null 2>&1; then
  check_pass "[6/12] Scanner self-test passes (accuracy verified)"
else
  check_fail "[6/12] Scanner self-test has failures"
fi

# ─── Check 7: Templates are valid (lint them) ───
template_ok=true
for tpl_dir in "$TEMPLATES_DIR"/*/; do
  [[ ! -f "$tpl_dir/SKILL.md" ]] && continue
  # Templates have placeholders so full lint won't pass;
  # just verify they have frontmatter delimiters and basic structure
  if ! head -1 "$tpl_dir/SKILL.md" | grep -q '^---$'; then
    template_ok=false
    break
  fi
done
if $template_ok; then
  tpl_count=$(find "$TEMPLATES_DIR" -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')
  check_pass "[7/12] Templates have valid structure ($tpl_count templates)"
else
  check_fail "[7/12] Some templates have invalid structure"
fi

# ─── Check 8: Frontmatter name matches directory ───
name_mismatches=()
for dir in "$SKILLS_DIR"/*/; do
  slug=$(basename "$dir")
  [[ ! -f "$dir/SKILL.md" ]] && continue
  fm_name=$(get_frontmatter_field "$dir/SKILL.md" "name" 2>/dev/null || echo "")
  if [[ -n "$fm_name" && "$fm_name" != "$slug" ]]; then
    name_mismatches+=("$slug(→$fm_name)")
  fi
done
if [[ ${#name_mismatches[@]} -eq 0 ]]; then
  check_pass "[8/12] All frontmatter names match directory slugs"
else
  check_fail "[8/12] Name mismatches: ${name_mismatches[*]}"
fi

# ─── Check 9: No orphan test files ───
orphan_tests=()
for test_file in "$TESTS_DIR"/*.test.sh; do
  [[ ! -f "$test_file" ]] && continue
  slug=$(basename "$test_file" .test.sh)
  # Skip infrastructure tests (not tied to a specific skill)
  [[ "$slug" == cdr-* ]] && continue
  [[ ! -d "$SKILLS_DIR/$slug" ]] && orphan_tests+=("$slug")
done
if [[ ${#orphan_tests[@]} -eq 0 ]]; then
  check_pass "[9/12] No orphan test files"
else
  check_warn "[9/12] Orphan test files: ${orphan_tests[*]}"
fi

# ─── Check 10: Python available ───
if [[ -n "$PYTHON" ]]; then
  py_ver=$($PYTHON --version 2>&1 | head -1)
  check_pass "[10/12] Python available ($py_ver)"
else
  check_fail "[10/12] Python not found (required for SARIF/YAML tools)"
fi

# ─── Check 11: Stats cache exists and is <7d old ───
if [[ -d "$CACHE_DIR" ]]; then
  latest_stats=$(find "$CACHE_DIR" -name "stats-*.json" -type f 2>/dev/null | sort -r | head -1)
  if [[ -n "$latest_stats" ]]; then
    # Check age: compare filename date to today
    file_date=$(basename "$latest_stats" | sed 's/stats-//;s/\.json//')
    days_old=$($PYTHON -c "
from datetime import datetime
d1 = datetime.strptime('$file_date', '%Y%m%d')
d2 = datetime.strptime('$TODAY', '%Y%m%d')
print((d2-d1).days)
" 2>/dev/null || echo "999")
    if [[ "$days_old" -le 7 ]]; then
      check_pass "[11/12] Stats cache is current (${days_old}d old)"
    else
      check_warn "[11/12] Stats cache is stale (${days_old}d old, run 'make stats')"
    fi
  else
    check_warn "[11/12] No stats snapshots found (run 'make stats')"
  fi
else
  check_pass "[11/12] Stats cache not initialized (optional — run 'make stats')"
fi

# ─── Check 12: Git working tree is clean ───
if git -C "$REPO_ROOT" diff --quiet 2>/dev/null && git -C "$REPO_ROOT" diff --cached --quiet 2>/dev/null; then
  untracked=$(git -C "$REPO_ROOT" ls-files --others --exclude-standard 2>/dev/null | wc -l | tr -d ' ')
  if [[ "$untracked" -eq 0 ]]; then
    check_pass "[12/12] Git working tree is clean"
  else
    check_warn "[12/12] Git tree has $untracked untracked files"
  fi
else
  check_warn "[12/12] Git tree has uncommitted changes"
fi

# ─── Summary ───
echo ""
echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"

if [[ $FAILED -eq 0 ]]; then
  echo -e "${BOLD}  Verification: ${GREEN}${PASSED}/${TOTAL} passed${RESET}, ${YELLOW}${WARNED} warnings${RESET}"
else
  echo -e "${BOLD}  Verification: ${GREEN}${PASSED} passed${RESET}, ${RED}${FAILED} failed${RESET}, ${YELLOW}${WARNED} warnings${RESET}"
fi

echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo ""

[[ $FAILED -gt 0 ]] && exit 1
exit 0
