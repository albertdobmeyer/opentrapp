#!/usr/bin/env bash
# Tests for engagement-control.sh

SCRIPT="$REPO_ROOT/scripts/engagement-control.sh"
ENV_FILE="$REPO_ROOT/config/.env"

# Save original .env and restore after each test
_save_env() {
  if [[ -f "$ENV_FILE" ]]; then
    cp "$ENV_FILE" "$ENV_FILE.test-backup"
  fi
}

_restore_env() {
  if [[ -f "$ENV_FILE.test-backup" ]]; then
    mv "$ENV_FILE.test-backup" "$ENV_FILE"
  fi
}

# ── Basic operation ──────────────────────────────────────

test_help_exits_0() {
  assert_exit_code 0 bash "$SCRIPT" --help
}

test_help_shows_usage() {
  assert_output_contains "Usage" bash "$SCRIPT" --help
}

test_invalid_level_exits_1() {
  assert_exit_code 1 bash "$SCRIPT" --level bogus --dry-run
}

test_no_mode_exits_1() {
  assert_exit_code 1 bash "$SCRIPT" --level observer
}

# ── Dry-run ──────────────────────────────────────────────

test_dry_run_observer_exits_0() {
  assert_exit_code 0 bash "$SCRIPT" --level observer --dry-run
}

test_dry_run_shows_no_changes() {
  assert_output_contains "dry run" bash "$SCRIPT" --level observer --dry-run
}

test_dry_run_researcher_shows_guidelines() {
  assert_output_contains "DEDICATED API key" bash "$SCRIPT" --level researcher --dry-run
}

test_dry_run_participant_shows_retraction() {
  assert_output_contains "retraction plan" bash "$SCRIPT" --level participant --dry-run
}

# ── Apply observer ───────────────────────────────────────

test_apply_observer_exits_0() {
  _save_env
  local rc=0
  bash "$SCRIPT" --level observer --apply >/dev/null 2>&1 || rc=$?
  _restore_env
  if (( rc == 0 )); then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
  else
    echo "    FAIL: expected exit 0, got $rc"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

test_apply_observer_sets_level() {
  _save_env
  bash "$SCRIPT" --level observer --apply >/dev/null 2>&1
  local level
  level=$(grep "^ENGAGEMENT_LEVEL=" "$ENV_FILE" | cut -d= -f2-)
  _restore_env
  if [[ "$level" == "observer" ]]; then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
  else
    echo "    FAIL: expected ENGAGEMENT_LEVEL=observer, got $level"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

test_apply_observer_sets_zero_rate_limits() {
  _save_env
  bash "$SCRIPT" --level observer --apply >/dev/null 2>&1
  local posts
  posts=$(grep "^RATE_LIMIT_POSTS_PER_HOUR=" "$ENV_FILE" | cut -d= -f2-)
  _restore_env
  if [[ "$posts" == "0" ]]; then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
  else
    echo "    FAIL: expected RATE_LIMIT_POSTS_PER_HOUR=0, got $posts"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

# ── Apply preserves user values ──────────────────────────

test_apply_preserves_api_key() {
  _save_env
  # Set a test API key
  bash "$SCRIPT" --level observer --apply >/dev/null 2>&1
  sed -i 's|^MOLTBOOK_API_KEY=.*|MOLTBOOK_API_KEY=test-preserve-key|' "$ENV_FILE"
  # Switch to researcher
  bash "$SCRIPT" --level researcher --apply >/dev/null 2>&1
  local key
  key=$(grep "^MOLTBOOK_API_KEY=" "$ENV_FILE" | cut -d= -f2-)
  _restore_env
  if [[ "$key" == "test-preserve-key" ]]; then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
  else
    echo "    FAIL: API key not preserved, got: $key"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

test_apply_preserves_agent_handle() {
  _save_env
  bash "$SCRIPT" --level observer --apply >/dev/null 2>&1
  sed -i 's|^AGENT_HANDLE=.*|AGENT_HANDLE=test-handle|' "$ENV_FILE"
  bash "$SCRIPT" --level participant --apply >/dev/null 2>&1
  local handle
  handle=$(grep "^AGENT_HANDLE=" "$ENV_FILE" | cut -d= -f2-)
  _restore_env
  if [[ "$handle" == "test-handle" ]]; then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
  else
    echo "    FAIL: handle not preserved, got: $handle"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

# ── Round-trip ───────────────────────────────────────────

test_round_trip_all_levels() {
  _save_env
  local ok=true
  for level in observer researcher participant observer; do
    bash "$SCRIPT" --level "$level" --apply >/dev/null 2>&1 || { ok=false; break; }
    local got
    got=$(grep "^ENGAGEMENT_LEVEL=" "$ENV_FILE" | cut -d= -f2-)
    if [[ "$got" != "$level" ]]; then
      echo "    FAIL: after applying $level, got ENGAGEMENT_LEVEL=$got"
      ok=false
      break
    fi
  done
  _restore_env
  if $ok; then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
  else
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

# ── Status ───────────────────────────────────────────────

test_status_exits_0() {
  assert_exit_code 0 bash "$SCRIPT" --status
}

test_status_shows_level() {
  assert_output_contains "Level" bash "$SCRIPT" --status
}

# ── Verify adapts to level ───────────────────────────────

test_verify_passes_observer() {
  _save_env
  bash "$SCRIPT" --level observer --apply >/dev/null 2>&1
  local rc=0
  bash "$REPO_ROOT/scripts/verify.sh" >/dev/null 2>&1 || rc=$?
  _restore_env
  if (( rc == 0 )); then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
  else
    echo "    FAIL: verify.sh failed for observer level (exit $rc)"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

test_verify_passes_researcher() {
  _save_env
  bash "$SCRIPT" --level researcher --apply >/dev/null 2>&1
  local rc=0
  bash "$REPO_ROOT/scripts/verify.sh" >/dev/null 2>&1 || rc=$?
  _restore_env
  # Researcher without API key gets warnings but should not fail
  if (( rc == 0 )); then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
  else
    echo "    FAIL: verify.sh failed for researcher level (exit $rc)"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}
