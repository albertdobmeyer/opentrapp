#!/usr/bin/env bash
# OpenCli-Container: Workspace Audit Tool
#
# Provides complete visibility into what the agent has done.
# Runs from the host. Works whether the container is running or stopped.
#
# Usage: bash scripts/vault-audit.sh [options]
#   --full       Full workspace listing with sizes and timestamps
#   --changes    Files created or modified since last audit
#   --diff FILE  Show content of a specific workspace file
#   --memory     Show all memory files
#   --sessions   Show session transcript summaries
#   --network    Parse proxy logs (domains, blocked, payload sizes)
#   --config     Verify running config matches expected shell level
#   --injection  Scan workspace for prompt injection patterns
#   --all        Run all of the above (except --diff)
#
# No arguments: same as --all

set -uo pipefail

VAULT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RUNTIME="podman"
command -v podman &>/dev/null || RUNTIME="docker"

# Resolve a compose service name to the actual container name via the
# `com.docker.compose.service` label. Works regardless of project name or
# `container_name:` overrides — see docs/specs/2026-05-10-script-container-resolution.md
resolve_service_container() {
    local service container
    for service in "$@"; do
        container=$($RUNTIME ps -a \
            --filter "label=com.docker.compose.service=$service" \
            --format '{{.Names}}' 2>/dev/null | head -n 1)
        if [ -n "$container" ]; then
            echo "$container"
            return 0
        fi
    done
    return 1
}

CONTAINER=$(resolve_service_container vault) || CONTAINER=""
PROXY_CONTAINER=$(resolve_service_container vault-proxy) || PROXY_CONTAINER=""
AUDIT_STATE_FILE="$VAULT_DIR/.vault-audit-timestamp"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# --- Helper functions ---

is_container_running() {
    $RUNTIME inspect "$1" --format '{{.State.Status}}' 2>/dev/null | grep -q "running"
}

exec_in_vault() {
    if is_container_running "$CONTAINER"; then
        $RUNTIME exec "$CONTAINER" sh -c "$1" 2>/dev/null
    else
        echo "[audit] Container not running — some checks require a running container" >&2
        return 1
    fi
}

exec_in_proxy() {
    if is_container_running "$PROXY_CONTAINER"; then
        $RUNTIME exec "$PROXY_CONTAINER" sh -c "$1" 2>/dev/null
    else
        echo "[audit] Proxy container not running — network audit unavailable" >&2
        return 1
    fi
}

section() {
    echo ""
    echo -e "${BOLD}${CYAN}=== $1 ===${NC}"
    echo ""
}

warn() {
    echo -e "  ${YELLOW}WARNING:${NC} $1"
}

flag() {
    echo -e "  ${RED}FLAG:${NC} $1"
}

ok() {
    echo -e "  ${GREEN}OK:${NC} $1"
}

# --- Audit functions ---

audit_full() {
    section "Workspace File Listing"

    if ! exec_in_vault "true" 2>/dev/null; then
        echo "  Container not running. Attempting via volume mount..."
        # Try to read the volume directly from the host
        local vol_path
        vol_path=$($RUNTIME volume inspect opencli-container_vault-data --format '{{.Mountpoint}}' 2>/dev/null)
        if [ -n "$vol_path" ] && [ -d "$vol_path/workspace" ]; then
            find "$vol_path/workspace" -type f -printf '  %T+ %8s %p\n' 2>/dev/null | sort
            echo ""
            echo "  Total files: $(find "$vol_path/workspace" -type f 2>/dev/null | wc -l)"
            local total_size
            total_size=$(du -sh "$vol_path/workspace" 2>/dev/null | cut -f1)
            echo "  Total size: $total_size"
        else
            echo "  No persistent volume found. Workspace is on tmpfs (volatile)."
        fi
        return
    fi

    exec_in_vault "find /home/vault/.openclaw/workspace/ -type f -exec ls -la {} \;" | \
        awk '{printf "  %-12s %8s  %s %s %s  %s\n", $1, $5, $6, $7, $8, $9}' | sort -k6

    echo ""
    local file_count
    file_count=$(exec_in_vault "find /home/vault/.openclaw/workspace/ -type f | wc -l")
    local total_size
    total_size=$(exec_in_vault "du -sh /home/vault/.openclaw/workspace/ | cut -f1")
    echo "  Total files: $file_count"
    echo "  Total size: $total_size"

    # Check for files outside workspace
    local outside_files
    outside_files=$(exec_in_vault "find /home/vault/.openclaw/ -path '*/workspace' -prune -o -newer /home/vault/.openclaw/openclaw.json -type f -print" 2>/dev/null)
    if [ -n "$outside_files" ]; then
        warn "Files modified outside workspace:"
        echo "$outside_files" | while read -r f; do
            echo "    $f"
        done
    fi
}

audit_changes() {
    section "Changes Since Last Audit"

    local last_audit="1970-01-01"
    if [ -f "$AUDIT_STATE_FILE" ]; then
        last_audit=$(cat "$AUDIT_STATE_FILE")
        echo "  Last audit: $last_audit"
    else
        echo "  First audit (no previous timestamp)"
    fi

    if ! exec_in_vault "true" 2>/dev/null; then
        echo "  Container not running — cannot check changes"
        return
    fi

    echo ""
    echo "  Changes:"

    # Create a reference file inside the container with the last audit timestamp.
    # This avoids depending on AGENTS.md which may not exist.
    local ref_created=false
    if [ "$last_audit" != "1970-01-01" ]; then
        exec_in_vault "touch -d '$last_audit' /tmp/.audit-ref 2>/dev/null" && ref_created=true
    fi

    local changes
    if $ref_created; then
        changes=$(exec_in_vault "find /home/vault/.openclaw/workspace/ -type f -newer /tmp/.audit-ref 2>/dev/null")
    else
        # First audit or no reference — list all workspace files
        changes=$(exec_in_vault "find /home/vault/.openclaw/workspace/ -type f 2>/dev/null")
    fi

    # Clean up reference file
    if $ref_created; then
        exec_in_vault "rm -f /tmp/.audit-ref 2>/dev/null" || true
    fi

    if [ -z "$changes" ]; then
        ok "No files modified since last audit"
    else
        echo "$changes" | while read -r f; do
            local size
            size=$(exec_in_vault "ls -la '$f' 2>/dev/null | awk '{print \$5}'")
            local timestamp
            timestamp=$(exec_in_vault "stat -c '%Y' '$f' 2>/dev/null")
            local date
            date=$(exec_in_vault "date -d @$timestamp '+%Y-%m-%d %H:%M:%S' 2>/dev/null" || echo "unknown")
            echo "    MODIFIED  $(basename "$f")  ($size bytes)  $date"
        done
    fi

    # Save current timestamp for next audit
    date -Iseconds > "$AUDIT_STATE_FILE"
}

audit_diff() {
    local filepath="$1"
    section "File Content: $filepath"

    if ! exec_in_vault "true" 2>/dev/null; then
        echo "  Container not running"
        return
    fi

    local full_path="/home/vault/.openclaw/workspace/$filepath"
    if exec_in_vault "test -f '$full_path'" 2>/dev/null; then
        exec_in_vault "cat '$full_path'"
    else
        echo "  File not found: $full_path"
        echo "  Available files:"
        exec_in_vault "find /home/vault/.openclaw/workspace/ -type f -name '*.md'" | \
            sed 's|/home/vault/.openclaw/workspace/|    |'
    fi
}

audit_memory() {
    section "Memory Files"

    if ! exec_in_vault "true" 2>/dev/null; then
        echo "  Container not running"
        return
    fi

    # Check for MEMORY.md (long-term)
    if exec_in_vault "test -f /home/vault/.openclaw/workspace/MEMORY.md" 2>/dev/null; then
        echo -e "  ${BOLD}MEMORY.md (long-term memory):${NC}"
        exec_in_vault "cat /home/vault/.openclaw/workspace/MEMORY.md" | sed 's/^/    /'
        echo ""
    else
        echo "  MEMORY.md: does not exist yet"
    fi

    # Check for daily memory files
    local memory_files
    memory_files=$(exec_in_vault "find /home/vault/.openclaw/workspace/memory/ -name '*.md' -type f 2>/dev/null | sort")
    if [ -n "$memory_files" ]; then
        echo -e "  ${BOLD}Daily memory files:${NC}"
        echo "$memory_files" | while read -r f; do
            local size
            size=$(exec_in_vault "wc -c < '$f' 2>/dev/null")
            local basename
            basename=$(basename "$f")
            echo "    $basename ($size bytes)"
            exec_in_vault "head -5 '$f'" | sed 's/^/      /'
            echo "      ..."
            echo ""
        done
    else
        echo "  No daily memory files yet (memory/ directory empty or missing)"
    fi

    # Check personality files for modifications
    echo -e "  ${BOLD}Personality files:${NC}"
    for pfile in SOUL.md IDENTITY.md USER.md; do
        if exec_in_vault "test -f /home/vault/.openclaw/workspace/$pfile" 2>/dev/null; then
            local size
            size=$(exec_in_vault "wc -c < /home/vault/.openclaw/workspace/$pfile 2>/dev/null")
            echo "    $pfile ($size bytes)"
        fi
    done
}

audit_sessions() {
    section "Session Transcripts"

    if ! exec_in_vault "true" 2>/dev/null; then
        echo "  Container not running"
        return
    fi

    local session_files
    session_files=$(exec_in_vault "find /home/vault/.openclaw/agents/main/sessions/ -name '*.jsonl' -type f 2>/dev/null")
    if [ -z "$session_files" ]; then
        echo "  No session transcripts found"
        return
    fi

    local total_sessions=0
    local total_size=0
    echo "$session_files" | while read -r f; do
        local lines
        lines=$(exec_in_vault "wc -l < '$f' 2>/dev/null")
        local size
        size=$(exec_in_vault "wc -c < '$f' 2>/dev/null")
        local basename
        basename=$(basename "$f")
        echo "    $basename: $lines entries, $size bytes"
        total_sessions=$((total_sessions + 1))
    done

    local session_count
    session_count=$(echo "$session_files" | wc -l)
    local total_sz
    total_sz=$(exec_in_vault "du -sh /home/vault/.openclaw/agents/main/sessions/ 2>/dev/null | cut -f1")
    echo ""
    echo "  Total sessions: $session_count"
    echo "  Total transcript size: $total_sz"
}

audit_network() {
    section "Network Activity (Proxy Logs)"

    local log_content
    if is_container_running "$PROXY_CONTAINER"; then
        log_content=$(exec_in_proxy "cat /var/log/vault-proxy/requests.jsonl 2>/dev/null")
    else
        # Try reading from volume
        local vol_path
        vol_path=$($RUNTIME volume inspect opencli-container_vault-proxy-logs --format '{{.Mountpoint}}' 2>/dev/null)
        if [ -n "$vol_path" ] && [ -f "$vol_path/requests.jsonl" ]; then
            log_content=$(cat "$vol_path/requests.jsonl")
        else
            echo "  No proxy logs available"
            return
        fi
    fi

    if [ -z "$log_content" ]; then
        echo "  Proxy log is empty"
        return
    fi

    # Count by action
    local allowed blocked
    allowed=$(echo "$log_content" | grep -c '"ALLOWED"' 2>/dev/null || echo 0)
    blocked=$(echo "$log_content" | grep -c '"BLOCKED"' 2>/dev/null || echo 0)
    local exfil_blocked
    exfil_blocked=$(echo "$log_content" | grep -c '"EXFIL_BLOCKED"' || true)
    exfil_blocked=${exfil_blocked:-0}
    local key_reflected
    key_reflected=$(echo "$log_content" | grep -c '"KEY_REFLECTED"' || true)
    key_reflected=${key_reflected:-0}

    echo "  Requests allowed: $allowed"
    echo "  Requests blocked: $blocked"
    if [ "$exfil_blocked" -gt 0 ]; then
        flag "Exfiltration attempts blocked: $exfil_blocked"
    fi
    if [ "$key_reflected" -gt 0 ]; then
        flag "API key reflections detected and redacted: $key_reflected"
    fi

    # Domains contacted
    echo ""
    echo -e "  ${BOLD}Domains contacted:${NC}"
    echo "$log_content" | grep '"ALLOWED"' | \
        grep -o '"host": *"[^"]*"' | sort | uniq -c | sort -rn | \
        while read -r count domain; do
            domain=$(echo "$domain" | sed 's/"host": *"//;s/"//')
            echo "    $count requests → $domain"
        done

    # Blocked domains
    local blocked_domains
    blocked_domains=$(echo "$log_content" | grep '"BLOCKED"' | \
        grep -o '"host": *"[^"]*"' | sort -u)
    if [ -n "$blocked_domains" ]; then
        echo ""
        echo -e "  ${BOLD}Blocked domains:${NC}"
        echo "$blocked_domains" | while read -r domain; do
            domain=$(echo "$domain" | sed 's/"host": *"//;s/"//')
            echo -e "    ${RED}BLOCKED${NC} → $domain"
        done
    fi

    # Largest requests (potential exfiltration)
    echo ""
    echo -e "  ${BOLD}Largest outbound requests:${NC}"
    echo "$log_content" | grep '"ALLOWED"' | grep '"request_bytes"' | \
        grep -o '"request_bytes": *[0-9]*' | sed 's/"request_bytes": *//' | \
        sort -rn | head -5 | while read -r size; do
            if [ -n "$size" ] && [ "$size" -gt 102400 ] 2>/dev/null; then
                warn "Large request: $size bytes (>100KB)"
            elif [ -n "$size" ]; then
                echo "    $size bytes"
            fi
        done
}

audit_config() {
    section "Configuration Verification"

    if ! exec_in_vault "true" 2>/dev/null; then
        echo "  Container not running — cannot verify running config"
        return
    fi

    # Check shell level
    local tool_profile
    tool_profile=$(exec_in_vault "cat /home/vault/.openclaw/openclaw.json" 2>/dev/null | \
        grep -o '"profile"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | \
        grep -o '"[^"]*"$' | tr -d '"')

    local exec_security
    exec_security=$(exec_in_vault "cat /home/vault/.openclaw/openclaw.json" 2>/dev/null | \
        grep -o '"security"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | \
        grep -o '"[^"]*"$' | tr -d '"')

    # Extract sandbox.mode specifically — grep for "mode" after "sandbox" context.
    # The config has multiple "mode" keys (compaction.mode, sandbox.mode, gateway.mode).
    # We use python for reliable JSON parsing instead of fragile grep chains.
    local sandbox_mode
    sandbox_mode=$(exec_in_vault "cat /home/vault/.openclaw/openclaw.json" 2>/dev/null | \
        python3 -c "import sys,json; c=json.loads(sys.stdin.read()); print(c.get('agents',{}).get('defaults',{}).get('sandbox',{}).get('mode','unknown'))" 2>/dev/null || echo "unknown")

    local elevated
    elevated=$(exec_in_vault "cat /home/vault/.openclaw/openclaw.json" 2>/dev/null | \
        grep -o '"enabled"[[:space:]]*:[[:space:]]*[a-z]*' | head -1 | \
        grep -o '[a-z]*$')

    echo "  Tool profile:     $tool_profile"
    echo "  Exec security:    $exec_security"
    echo "  Sandbox mode:     $sandbox_mode"
    echo "  Elevated access:  $elevated"

    # Determine shell level by checking exec security, profile, and deny list.
    # Hard Shell:  profile=minimal, exec.security=deny, group:fs in deny list
    # Split Shell: profile=coding,  exec.security=allowlist, exec.ask=always,
    #              browser/web/sessions denied, safeBins present
    # Soft Shell:  not yet implemented
    local has_group_fs_denied
    has_group_fs_denied=$(exec_in_vault "grep -c 'group:fs' /home/vault/.openclaw/openclaw.json" 2>/dev/null || echo "0")

    local exec_ask
    exec_ask=$(exec_in_vault "cat /home/vault/.openclaw/openclaw.json" 2>/dev/null | \
        grep -o '"ask"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | \
        grep -o '"[^"]*"$' | tr -d '"')

    local has_browser_denied
    has_browser_denied=$(exec_in_vault "grep -c '\"browser\"' /home/vault/.openclaw/openclaw.json" 2>/dev/null || echo "0")

    if [ "$exec_security" = "deny" ] && [ "$tool_profile" = "minimal" ]; then
        ok "Shell level: HARD SHELL (maximum lockdown — exec denied, fs denied)"
    elif [ "$exec_security" = "allowlist" ] && [ "$tool_profile" = "coding" ] && [ "$exec_ask" = "always" ] && [ "$has_browser_denied" -gt 0 ]; then
        ok "Shell level: SPLIT SHELL (controlled exec via safeBins + approval, browser/web denied)"
    elif [ "$exec_security" = "deny" ]; then
        ok "Shell level: HARD SHELL (exec denied)"
    else
        flag "Shell level: UNKNOWN — config does not match Hard Shell or Split Shell. Soft Shell is not yet implemented."
    fi

    # Check critical settings
    if [ "$elevated" = "false" ]; then
        ok "Elevated access: disabled"
    else
        flag "Elevated access is NOT disabled!"
    fi

    # Check deny list
    local deny_list
    deny_list=$(exec_in_vault "cat /home/vault/.openclaw/openclaw.json" 2>/dev/null | \
        grep -o '"deny"[[:space:]]*:[[:space:]]*\[' | wc -l)
    if [ "$deny_list" -gt 0 ]; then
        ok "Tool deny list: present"
    else
        flag "No tool deny list found in config!"
    fi
}

audit_tools() {
    section "Tool Status"

    if ! exec_in_vault "true" 2>/dev/null; then
        echo "  Container not running — cannot check tool status"
        return
    fi

    local MANIFEST="$VAULT_DIR/config/tool-manifest.yml"
    local CORE="$VAULT_DIR/scripts/tool-control-core.py"

    if [ ! -f "$MANIFEST" ] || [ ! -f "$CORE" ]; then
        echo "  Tool manifest or core script not found — skipping"
        return
    fi

    local config_json
    config_json=$(exec_in_vault "cat /home/vault/.openclaw/openclaw.json 2>/dev/null") || {
        echo "  Cannot read config from container"
        return
    }

    # Use tool-control-core.py --status mode for per-tool analysis
    local status_json
    status_json=$(python3 "$CORE" --manifest "$MANIFEST" --output status --status-json "$config_json" 2>/dev/null) || {
        echo "  Failed to analyze tool status"
        return
    }

    echo "$status_json" | python3 -c "
import sys, json

s = json.loads(sys.stdin.read())

# Colors
colors = {'critical': '\033[1;31m', 'high': '\033[0;31m', 'medium': '\033[0;33m', 'low': '\033[0;36m'}
nc = '\033[0m'
green = '\033[0;32m'
red = '\033[0;31m'
yellow = '\033[1;33m'

print(f'  Profile:        {s[\"profile\"]}')
print(f'  Exec security:  {s[\"exec_security\"]}')
print(f'  SafeBins:       {s[\"safeBins_count\"]}')
print(f'  Risk score:     {s[\"risk_score\"]}')
print(f'  Enabled tools:  {s[\"enabled_count\"]} of {len(s[\"tools\"])}')
print()

enabled = []
denied = []
never = []

for name, t in sorted(s['tools'].items()):
    if t['status'] == 'ENABLED':
        enabled.append((name, t))
    elif t['status'] == 'NEVER':
        never.append((name, t))
    else:
        denied.append((name, t))

if enabled:
    print(f'  {green}Enabled:{nc}')
    for name, t in enabled:
        c = colors.get(t['risk'], '')
        print(f'    {c}{t[\"risk\"]:<10}{nc} {name:<22} {t[\"description\"]}')
    print()

if denied:
    print(f'  Denied:')
    for name, t in denied:
        print(f'    {\"\":<10} {name:<22} {t[\"description\"]}')
    print()

if never:
    print(f'  {red}NEVER enabled:{nc}')
    for name, t in never:
        c = colors.get(t['risk'], '')
        print(f'    {c}{t[\"risk\"]:<10}{nc} {name:<22} {t[\"description\"]}')
    print()

# Security flags
safebins = []
for name, t in s['tools'].items():
    if t['status'] == 'NEVER':
        # Check if it's actually in the deny list (verify enforcement)
        pass  # This is handled by verify.sh checks 19-23

if s['risk_score'] > 0.5:
    print(f'  {yellow}WARNING: Risk score above 0.5 — high autonomy level{nc}')
elif s['risk_score'] == 0.0:
    print(f'  {green}Maximum lockdown — no tools enabled{nc}')
else:
    print(f'  {green}Risk score within normal range{nc}')
"
}

audit_injection() {
    section "Prompt Injection Scan"

    if ! exec_in_vault "true" 2>/dev/null; then
        echo "  Container not running"
        return
    fi

    local workspace_files
    workspace_files=$(exec_in_vault "find /home/vault/.openclaw/workspace/ -type f -name '*.md' -o -name '*.txt' -o -name '*.json'" 2>/dev/null)

    if [ -z "$workspace_files" ]; then
        echo "  No text files to scan"
        return
    fi

    local flags_found=0

    # Known prompt injection patterns
    local patterns=(
        "ignore previous instructions"
        "ignore all previous"
        "you are now"
        "new instructions:"
        "system prompt override"
        "forget everything"
        "disregard your"
        "override your"
        "pretend you are"
        "act as if you"
        "ADMIN MODE"
        "DEVELOPER MODE"
        "DAN mode"
        "jailbreak"
    )

    echo "  Scanning workspace files for injection patterns..."
    echo ""

    for pattern in "${patterns[@]}"; do
        local matches
        matches=$(exec_in_vault "grep -ril '$pattern' /home/vault/.openclaw/workspace/ 2>/dev/null")
        if [ -n "$matches" ]; then
            flag "Pattern '$pattern' found in:"
            echo "$matches" | while read -r f; do
                local line
                line=$(exec_in_vault "grep -in '$pattern' '$f' 2>/dev/null | head -3")
                echo "      $f:"
                echo "$line" | sed 's/^/        /'
            done
            flags_found=$((flags_found + 1))
        fi
    done

    # Check for base64 encoded blocks (potential hidden payloads)
    local b64_matches
    b64_matches=$(exec_in_vault "grep -rl '[A-Za-z0-9+/]\{64,\}' /home/vault/.openclaw/workspace/ 2>/dev/null")
    if [ -n "$b64_matches" ]; then
        warn "Possible base64-encoded blocks found in:"
        echo "$b64_matches" | sed 's/^/      /'
        flags_found=$((flags_found + 1))
    fi

    # Check for unusually large files
    local large_files
    large_files=$(exec_in_vault "find /home/vault/.openclaw/workspace/ -type f -size +100k 2>/dev/null")
    if [ -n "$large_files" ]; then
        warn "Unusually large workspace files (>100KB):"
        echo "$large_files" | while read -r f; do
            local size
            size=$(exec_in_vault "ls -lh '$f' 2>/dev/null | awk '{print \$5}'")
            echo "      $f ($size)"
        done
        flags_found=$((flags_found + 1))
    fi

    echo ""
    if [ "$flags_found" -eq 0 ]; then
        ok "No injection patterns detected"
    else
        flag "$flags_found potential issues found — review flagged files manually"
    fi
}

# --- Main ---

print_banner() {
    echo ""
    echo -e "${BOLD}OpenCli-Container Workspace Audit${NC}"
    echo "=============================="
    echo ""
    echo "  Container:  ${CONTAINER:-vault} ($([ -n "$CONTAINER" ] && is_container_running "$CONTAINER" && echo 'running' || echo 'not found'))"
    echo "  Proxy:      ${PROXY_CONTAINER:-vault-proxy} ($([ -n "$PROXY_CONTAINER" ] && is_container_running "$PROXY_CONTAINER" && echo 'running' || echo 'not found'))"
    echo "  Audit time: $(date '+%Y-%m-%d %H:%M:%S')"
}

MODE="${1:---all}"

case "$MODE" in
    --full)
        print_banner
        audit_full
        ;;
    --changes)
        print_banner
        audit_changes
        ;;
    --diff)
        if [ -z "${2:-}" ]; then
            echo "Usage: $0 --diff <filepath>"
            echo "  filepath is relative to workspace/ (e.g., memory/2026-03-25.md)"
            exit 1
        fi
        print_banner
        audit_diff "$2"
        ;;
    --memory)
        print_banner
        audit_memory
        ;;
    --sessions)
        print_banner
        audit_sessions
        ;;
    --network)
        print_banner
        audit_network
        ;;
    --config)
        print_banner
        audit_config
        ;;
    --tools)
        print_banner
        audit_tools
        ;;
    --injection)
        print_banner
        audit_injection
        ;;
    --all)
        print_banner
        audit_full
        audit_changes
        audit_memory
        audit_sessions
        audit_network
        audit_config
        audit_tools
        audit_injection
        echo ""
        echo -e "${BOLD}=============================="
        echo -e "Audit complete.${NC}"
        ;;
    *)
        echo "Usage: $0 [--full|--changes|--diff FILE|--memory|--sessions|--network|--config|--tools|--injection|--all]"
        echo ""
        echo "  --full       Full workspace listing with sizes and timestamps"
        echo "  --changes    Files created or modified since last audit"
        echo "  --diff FILE  Show content of a specific workspace file"
        echo "  --memory     Show all memory files (memory/*.md + MEMORY.md)"
        echo "  --sessions   Show session transcript summaries"
        echo "  --network    Parse proxy logs (domains, blocked, payload sizes)"
        echo "  --config     Verify running config matches expected shell level"
        echo "  --tools      Per-tool status (enabled/denied/never, risk score)"
        echo "  --injection  Scan workspace for prompt injection patterns"
        echo "  --all        Run all of the above (default)"
        exit 1
        ;;
esac
