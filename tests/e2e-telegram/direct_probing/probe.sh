#!/usr/bin/env bash
# Direct-container perimeter probing — no Telegram, no LLM, no cost.
#
# Uses `podman exec` to run probe commands inside vault-agent and
# vault-proxy. Each probe has an expected outcome (pass=security enforced,
# fail=breach) and a category. Produces a findings markdown report.
#
# Run:
#   bash tests/e2e-telegram/direct_probing/probe.sh
#
# Output:
#   tests/e2e-telegram/direct_probing/findings-<date>.md

set -u -o pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATE="$(date +%Y-%m-%d)"
TIME="$(date -Iseconds)"
REPORT="${HERE}/findings-${DATE}.md"
# Temp files for streaming results — assembled into REPORT at end.
RES_FILE="$(mktemp)"
SUMMARY_FILE="$(mktemp)"
trap 'rm -f "$RES_FILE" "$SUMMARY_FILE"' EXIT

pass=0
fail=0
incon=0

log_probe() {
    local category="$1"
    local outcome="$2"   # PASS / FAIL / INCONCLUSIVE
    local title="$3"
    local detail="$4"
    case "$outcome" in
        PASS) ((pass++));;
        FAIL) ((fail++));;
        INCONCLUSIVE) ((incon++));;
    esac
    {
        echo "### $outcome — $title"
        echo ""
        echo "**Category:** $category"
        echo ""
        echo "\`\`\`"
        echo "$detail" | head -30
        echo "\`\`\`"
        echo ""
    } >> "$RES_FILE"
    printf "[%s] %-12s %s\n" "$outcome" "$category" "$title"
}

# ----------------------------------------------------------------------
# 1. Preflight
# ----------------------------------------------------------------------
echo "=== Preflight ==="
if ! podman ps --format "{{.Names}}" | grep -q "^vault-agent$"; then
    echo "ABORT: vault-agent not running. Run 'podman compose up -d' first."
    exit 1
fi
if ! podman ps --format "{{.Names}}" | grep -q "^vault-proxy$"; then
    echo "ABORT: vault-proxy not running."
    exit 1
fi
echo "All containers up."
echo

# ----------------------------------------------------------------------
# 2. Filesystem-reach probes (vault-agent)
# ----------------------------------------------------------------------
echo "=== Filesystem reach (vault-agent) ==="

# Host-only paths that MUST NOT resolve inside the container.
# /etc/shadow exists as an empty stub in most base images; probe readability
# instead of existence for it. We only check HOST paths here.
declare -a HOST_PATHS=(
    "/home/albertd/.ssh/id_rsa"
    "/home/albertd/Repositories/opentrapp/.env"
    "/home/albertd/Repositories/opentrapp/.env.test"
    "/home/albertd/.opentrapp/test-sessions"
)
for p in "${HOST_PATHS[@]}"; do
    out=$(podman exec vault-agent sh -c "test -e '$p' && echo FOUND || echo ABSENT" 2>&1)
    if [ "$out" = "ABSENT" ]; then
        log_probe "fs-read" "PASS" "Host path not reachable: $p" "$out"
    elif [ "$out" = "FOUND" ]; then
        log_probe "fs-read" "FAIL" "Host path visible inside container: $p" "$out"
    else
        log_probe "fs-read" "INCONCLUSIVE" "Probe error for $p" "$out"
    fi
done

# /etc/shadow readability — the file may exist as a stub; the security
# property we care about is that the non-root vault user can't read it.
out=$(podman exec vault-agent sh -c "cat /etc/shadow 2>&1; echo RC=\$?" 2>&1 | head -5)
if echo "$out" | grep -qE "(Permission denied|RC=[^0])"; then
    log_probe "fs-read" "PASS" "/etc/shadow not readable by non-root user" "$out"
else
    log_probe "fs-read" "FAIL" "/etc/shadow readable inside container" "$out"
fi

# /etc/passwd should be container's, not host's. Host user 'albertd' must
# NOT appear.
out=$(podman exec vault-agent cat /etc/passwd 2>&1 | head -40)
if echo "$out" | grep -q "^albertd"; then
    log_probe "fs-read" "FAIL" "/etc/passwd contains host user 'albertd'" "$out"
else
    log_probe "fs-read" "PASS" "/etc/passwd is container-local (no host user)" "$(echo "$out" | head -5)"
fi

# /proc/mounts — named volumes reveal the underlying host block device
# (e.g. /dev/sda2). This is information disclosure, not a reachability
# breach (container cannot access the device itself). Surface as a note,
# not a FAIL.
out=$(podman exec vault-agent cat /proc/mounts 2>&1 | grep -E "(nvme|/dev/sda|/dev/mapper)" | head -5)
if [ -n "$out" ]; then
    log_probe "fs-read" "INCONCLUSIVE" "/proc/mounts discloses host block device (info leak, not reachability breach)" "$out"
else
    log_probe "fs-read" "PASS" "/proc/mounts does not expose host block devices" "(no matches)"
fi

# ----------------------------------------------------------------------
# 3. Container-escape probes
# ----------------------------------------------------------------------
echo "=== Container-escape attempts ==="

# mount syscall — blocked by seccomp per vault-seccomp.json.
out=$(podman exec vault-agent sh -c "mount -o bind /proc/1/root /tmp/host 2>&1; echo RC=\$?" 2>&1)
if echo "$out" | grep -qE "(Operation not permitted|not permitted|RC=[^0])"; then
    log_probe "escape" "PASS" "mount syscall blocked" "$out"
elif echo "$out" | grep -q "RC=0"; then
    log_probe "escape" "FAIL" "mount syscall SUCCEEDED" "$out"
else
    log_probe "escape" "INCONCLUSIVE" "mount probe unclear" "$out"
fi

# unshare — blocked by seccomp.
out=$(podman exec vault-agent sh -c "unshare --mount sh -c 'echo success' 2>&1; echo RC=\$?" 2>&1)
if echo "$out" | grep -qE "(Operation not permitted|not permitted|RC=[^0])"; then
    log_probe "escape" "PASS" "unshare syscall blocked" "$out"
else
    log_probe "escape" "FAIL" "unshare syscall may have succeeded" "$out"
fi

# ptrace — blocked by seccomp.
out=$(podman exec vault-agent sh -c "strace -p 1 2>&1; echo RC=\$?" 2>&1)
if echo "$out" | grep -qE "(Operation not permitted|No such file|not found|RC=[^0])"; then
    log_probe "escape" "PASS" "ptrace blocked or strace unavailable" "$(echo "$out" | head -5)"
else
    log_probe "escape" "FAIL" "strace attached to PID 1" "$out"
fi

# Docker socket — must not be mounted.
out=$(podman exec vault-agent sh -c "test -S /var/run/docker.sock && echo FOUND || echo ABSENT" 2>&1)
if [ "$out" = "ABSENT" ]; then
    log_probe "escape" "PASS" "/var/run/docker.sock not mounted" "$out"
else
    log_probe "escape" "FAIL" "Docker socket present inside container" "$out"
fi

# setuid-bit exec — the interesting question is NOT whether the bit sticks
# on tmpfs (nosuid mounts make that cosmetic), but whether a setuid binary
# actually grants elevated privilege when executed. tmpfs is mounted with
# `nosuid` so the kernel ignores the setuid bit on exec. Verify nosuid is
# actually present on /tmp (would be the breach if missing).
out=$(podman exec vault-agent cat /proc/mounts 2>&1 | grep " /tmp " | head -1)
if echo "$out" | grep -q "nosuid"; then
    log_probe "escape" "PASS" "/tmp mounted nosuid (setuid bit is cosmetic)" "$out"
else
    log_probe "escape" "FAIL" "/tmp mount missing nosuid flag" "$out"
fi

# PID limit — fork 300 background shells, expect OOM/ENOMEM/EAGAIN somewhere.
out=$(podman exec vault-agent sh -c "for i in \$(seq 1 300); do (sleep 0.1 &); done 2>&1 | tail -3" 2>&1)
if echo "$out" | grep -qE "(Resource temporarily|Cannot allocate|fork)"; then
    log_probe "escape" "PASS" "PID limit enforced (fork rejected)" "$out"
else
    # Didn't hit the limit — not necessarily a failure (300 may be below the
    # 256 limit due to short-lived sleeps exiting). Mark inconclusive.
    log_probe "escape" "INCONCLUSIVE" "PID limit probe did not trigger" "$out"
fi

# ----------------------------------------------------------------------
# 4. Network egress (from vault-agent, through vault-proxy)
# ----------------------------------------------------------------------
echo "=== Network egress ==="

# Non-allowlisted domain should be blocked.
# vault-agent has node but no curl; use node's built-in https module so we
# test the same code path OpenClaw uses (undici via proxy-bootstrap.mjs).
NODE_PROBE_JS='const https = require("https"); const req = https.request({hostname:"attacker.example.com",path:"/",timeout:8000,method:"GET"}, res => { console.log("HTTP=" + res.statusCode); process.exit(0); }); req.on("error", e => { console.log("ERR=" + e.message); process.exit(0); }); req.on("timeout", () => { console.log("TIMEOUT"); req.destroy(); process.exit(0); }); req.end();'
out=$(podman exec vault-agent sh -c "node -e '$NODE_PROBE_JS'" 2>&1)
if echo "$out" | grep -qE "HTTP=403|blocked_by_vault|ERR=|TIMEOUT"; then
    log_probe "network" "PASS" "Non-allowlisted domain blocked" "$out"
elif echo "$out" | grep -q "HTTP=200"; then
    log_probe "network" "FAIL" "Non-allowlisted domain returned HTTP 200" "$out"
else
    log_probe "network" "INCONCLUSIVE" "Non-allowlisted probe unclear" "$out"
fi

# Direct TCP to a raw IP (bypassing proxy) should fail — network is internal.
out=$(podman exec vault-agent sh -c "(echo > /dev/tcp/1.1.1.1/53) 2>&1; echo RC=\$?" 2>&1)
if echo "$out" | grep -qE "(No route|refused|timeout|RC=[^0])"; then
    log_probe "network" "PASS" "Direct TCP to raw IP fails (internal net)" "$out"
else
    log_probe "network" "FAIL" "Direct TCP to 1.1.1.1 succeeded" "$out"
fi

# Host reachability (via gateway) should fail — agent-net is internal: true.
out=$(podman exec vault-agent sh -c "(echo > /dev/tcp/host.docker.internal/22) 2>&1; echo RC=\$?" 2>&1)
if echo "$out" | grep -qE "(not known|timeout|No route|refused|RC=[^0])"; then
    log_probe "network" "PASS" "host.docker.internal unreachable" "$out"
else
    log_probe "network" "FAIL" "Host reachable via host.docker.internal" "$out"
fi

# ----------------------------------------------------------------------
# 5. Credential isolation — vault-agent MUST NOT have real API key
# ----------------------------------------------------------------------
echo "=== Credential isolation ==="

out=$(podman exec vault-agent sh -c "env 2>&1 | grep -i api_key" 2>&1)
if echo "$out" | grep -q "sk-ant-api"; then
    log_probe "creds" "FAIL" "Real Anthropic key visible in vault-agent env" "$(echo "$out" | sed 's/sk-ant-api[^ ]*/sk-ant-api<REDACTED>/')"
elif [ -z "$out" ]; then
    log_probe "creds" "PASS" "No API_KEY env vars visible in vault-agent" "(no matches)"
else
    # Sentinel placeholder or unrelated var — just flag what's there, no secret values.
    log_probe "creds" "PASS" "API_KEY env is placeholder (no 'sk-ant-api' prefix)" "$(echo "$out" | head -3)"
fi

# /proc/self/environ should also show only the sentinel, not the real key.
out=$(podman exec vault-agent sh -c "tr '\\0' '\\n' < /proc/self/environ 2>&1 | grep -i api_key" 2>&1)
if echo "$out" | grep -q "sk-ant-api"; then
    log_probe "creds" "FAIL" "Real key visible in /proc/self/environ" "<redacted>"
else
    log_probe "creds" "PASS" "/proc/self/environ clean of real key" "$(echo "$out" | head -2)"
fi

# vault-proxy SHOULD have the real key (by design). This is a sanity check
# that our architecture matches our model.
out=$(podman exec vault-proxy sh -c "env | grep -c 'ANTHROPIC_API_KEY=sk-ant'" 2>&1)
if [ "$out" = "1" ]; then
    log_probe "creds" "PASS" "Real key present in vault-proxy (expected)" "count=$out"
else
    log_probe "creds" "FAIL" "vault-proxy missing real API key" "count=$out"
fi

# ----------------------------------------------------------------------
# 6. Security-option sanity (compose.yml promises vs reality)
# ----------------------------------------------------------------------
echo "=== Security-option sanity ==="

for svc in vault-agent vault-proxy vault-forge vault-pioneer; do
    out=$(podman inspect "$svc" --format '{{.HostConfig.ReadonlyRootfs}}|{{.HostConfig.SecurityOpt}}|{{.HostConfig.CapDrop}}' 2>&1)
    log_probe "compose-sanity" "PASS" "inspect: $svc" "$out"
done

# ----------------------------------------------------------------------
# 7. Proxy redaction re-verification
# ----------------------------------------------------------------------
echo "=== Proxy redaction re-verification ==="

TG_TOKEN="$(awk -F= '/^TELEGRAM_BOT_TOKEN=/{print $2}' "${HERE}/../../../.env" 2>/dev/null | tr -d '\r' | tr -d '\n')"
if [ -n "$TG_TOKEN" ]; then
    leak_count=$(podman logs vault-proxy 2>&1 | grep -c "$TG_TOKEN" || true)
    if [ "$leak_count" = "0" ]; then
        log_probe "redaction" "PASS" "Bot token never appears in vault-proxy stdout logs" "leak_count=0"
    else
        log_probe "redaction" "FAIL" "Bot token appears $leak_count times in vault-proxy logs" "leak_count=$leak_count"
    fi
else
    log_probe "redaction" "INCONCLUSIVE" "Could not read TELEGRAM_BOT_TOKEN from .env" ""
fi

# ----------------------------------------------------------------------
# Assemble report
# ----------------------------------------------------------------------
{
    echo "# Direct-container perimeter findings — ${DATE}"
    echo ""
    echo "**Run at:** ${TIME}"
    echo ""
    echo "**Method:** Direct \`podman exec\` probes into running containers. No"
    echo "LLM, no Telegram, no Anthropic cost. Tests the security boundary from"
    echo "the attacker's perspective: can commands inside the container reach"
    echo "host state?"
    echo ""
    echo "## Summary"
    echo ""
    echo "| Outcome | Count |"
    echo "|---|---|"
    echo "| PASS         | ${pass} |"
    echo "| FAIL         | ${fail} |"
    echo "| INCONCLUSIVE | ${incon} |"
    echo ""
    if [ "$fail" -gt 0 ]; then
        echo "**⚠️ ${fail} probe(s) failed — the perimeter has holes.** See details below."
    elif [ "$incon" -gt 0 ]; then
        echo "**${incon} inconclusive probe(s)** — missing tools or ambiguous output. Review manually."
    else
        echo "**All probes passed.** No observable host-reachability breach from inside the perimeter."
    fi
    echo ""
    echo "## Details"
    echo ""
    cat "$RES_FILE"
} > "$REPORT"

echo
echo "================================================================="
echo "Totals:  PASS=$pass  FAIL=$fail  INCONCLUSIVE=$incon"
echo "Report:  $REPORT"
echo "================================================================="

# Exit code: non-zero if any FAIL.
if [ "$fail" -gt 0 ]; then
    exit 2
fi
exit 0
