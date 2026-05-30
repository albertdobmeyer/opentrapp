#!/usr/bin/env bash
# Assertion primitives for tool behavioral tests

TOOL_ASSERT_PASS=0
TOOL_ASSERT_FAIL=0

# Assert a command exits with expected code
# Usage: assert_exit_code <expected> <command...>
assert_exit_code() {
  local expected="$1"; shift
  local rc=0
  "$@" >/dev/null 2>&1 || rc=$?
  if (( rc == expected )); then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: expected exit $expected, got $rc: $*"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

# Assert command stdout contains pattern
# Usage: assert_output_contains <pattern> <command...>
assert_output_contains() {
  local pattern="$1"; shift
  local output
  output=$("$@" 2>&1) || true
  if echo "$output" | grep -qE "$pattern"; then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: output missing pattern '$pattern': $*"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

# Assert command stdout does NOT contain pattern
# Usage: assert_output_not_contains <pattern> <command...>
assert_output_not_contains() {
  local pattern="$1"; shift
  local output
  output=$("$@" 2>&1) || true
  if ! echo "$output" | grep -qE "$pattern"; then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: output unexpectedly contains '$pattern': $*"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

# Assert command succeeds (exit 0)
# Usage: assert_command_succeeds <command...>
assert_command_succeeds() {
  if "$@" >/dev/null 2>&1; then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: command failed (expected success): $*"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

# Assert command fails (exit != 0)
# Usage: assert_command_fails <command...>
assert_command_fails() {
  if "$@" >/dev/null 2>&1; then
    echo "    FAIL: command succeeded (expected failure): $*"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  else
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
    return 0
  fi
}

# Assert file exists
# Usage: assert_file_exists <path>
assert_file_exists() {
  local path="$1"
  if [[ -f "$path" ]]; then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: file not found: $path"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

# Assert directory exists
# Usage: assert_dir_exists <path>
assert_dir_exists() {
  local path="$1"
  if [[ -d "$path" ]]; then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: directory not found: $path"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}

# Assert JSON field via Python expression
# Usage: assert_json_field <json_string> <py_expr>
assert_json_field() {
  local json_str="$1" py_expr="$2"
  local py
  py=$(command -v python3 2>/dev/null || command -v python 2>/dev/null) || {
    echo "    SKIP: Python not found"
    return 0
  }
  local result
  result=$(echo "$json_str" | "$py" -c "
import sys, json
d = json.load(sys.stdin)
print('OK' if ($py_expr) else 'FAIL')
" 2>/dev/null || echo "ERROR")

  if [[ "$result" == "OK" ]]; then
    TOOL_ASSERT_PASS=$((TOOL_ASSERT_PASS + 1))
    return 0
  else
    echo "    FAIL: JSON assertion failed: $py_expr"
    TOOL_ASSERT_FAIL=$((TOOL_ASSERT_FAIL + 1))
    return 1
  fi
}
