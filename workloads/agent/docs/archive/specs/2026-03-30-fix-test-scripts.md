# Spec: Fix 7 Failing Test Scripts

**Date:** 2026-03-30
**Phase:** 3 (Split Shell Completion) — follow-up
**Security implications:** These are verification tests. Broken tests mean broken visibility into our security posture.

---

## Purpose

Fix the 7 test scripts that fail when run by `make test`. These were written pre-Phase-1 with assumptions that no longer hold (wrong config paths, missing runtime detection, regex bugs).

## Diagnosis Per Script

### 1. test-config-integrity.sh — REWRITE
**Problem:** Looks for config at `/home/vault/.config/openclaw/config.yml` (YAML). Actual path is `/home/vault/.openclaw/openclaw.json` (JSON5). Also checks for keys that don't exist in current OpenClaw schema (`persistence`, `telemetry.enabled`, `mdns`, `scope: session`).
**Fix:** Rewrite to verify the actual JSON5 config. Check the security-critical settings: profile, exec.security, exec.ask, elevated.enabled, sandbox.mode. Use `node` for JSON parsing (no python3 in container).

### 2. test-container-escape-vectors.sh — FIX /proc/kcore CHECK
**Problem:** Check 3 (`/proc/kcore`) fails with "readable". On Podman rootless, `/proc/kcore` exists but is a zero-length file — `head -c 1` exits 0 because the file exists, even though it's empty.
**Fix:** Check if `/proc/kcore` has readable content (non-zero size), not just if the command succeeds. This is a low-risk finding — rootless Podman's kcore is empty/inaccessible for real reads.

### 3. test-key-not-visible.sh — FIX REGEX SELF-MATCH
**Problem:** Check 3 greps for `sk-ant-api|sk-[a-zA-Z0-9]{20,}|api_key|apikey` in `/proc/*/cmdline`, but the grep pattern itself contains `api_key` and `apikey`, which match the grep command's own args in `/proc/self/cmdline`.
**Fix:** Filter out the grep process itself (`grep -v grep`), or use a pattern that won't match itself.

### 4. test-network-isolation.sh — FIX CONTAINER DETECTION
**Problem:** Uses `$RUNTIME inspect` but the test output says "Container not running" despite the container running. The `set -uo pipefail` combined with the inspect check may be exiting early.
**Fix:** Check runtime detection and inspect command. This script already has proper runtime detection (lines 9-10). May be a timing issue or the `set -uo` causing unbound variable exit. Need live debugging.

### 5. test-no-new-privileges.sh — FIX AWK QUOTING
**Problem:** The awk command uses `{print \$2}` inside `sh -c` which has quoting issues. The `$RUNTIME exec` output includes error text from Podman that the test doesn't expect.
**Fix:** Use `grep -o` or a simpler extraction method instead of awk. Ensure the exec command runs cleanly.

### 6. test-proxy-hardening.sh — FIX INSPECT FORMAT
**Problem:** `$RUNTIME inspect` fails with "no container found". Uses `${RUNTIME:-podman}` which should work, but may be hitting a Podman version-specific format issue.
**Fix:** Verify the inspect format string works with Podman 4.9.3. Test container detection logic.

### 7. test-seccomp-enforcement.sh — SAME AWK QUOTING
**Problem:** Same awk quoting issue as test-no-new-privileges.sh — `{print \$2}` inside `sh -c`.
**Fix:** Same approach — replace awk with grep or simpler parsing.

## Implementation Approach

Fix one script at a time. After each fix:
1. Run the individual script to confirm it passes
2. Run `make test` to confirm no regressions
3. Commit the fix

## Verification Plan

After all 7 are fixed: `make test` should show 12/12 PASS. Then run `make verify` to confirm the 18-point check still passes (no interference).
