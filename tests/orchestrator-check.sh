#!/usr/bin/env bash
# =============================================================================
# OpenTrApp Orchestration Health Check
# =============================================================================
# Comprehensive validation of the monorepo orchestration layer:
#   - Schema validity
#   - Manifest parsing & cross-reference integrity
#   - Submodule synchronization
#   - Build verification
#   - Component contract compliance
#
# Usage: bash tests/orchestrator-check.sh [--fix]
#   --fix  Attempt to auto-fix detected issues (e.g., submodule sync)
# =============================================================================

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FIX_MODE="${1:-}"
PASS=0
FAIL=0
WARN=0

# Colors (safe for no-color terminals)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

pass() { PASS=$((PASS+1)); echo -e "  ${GREEN}[PASS]${NC} $1"; }
fail() { FAIL=$((FAIL+1)); echo -e "  ${RED}[FAIL]${NC} $1"; }
warn() { WARN=$((WARN+1)); echo -e "  ${YELLOW}[WARN]${NC} $1"; }
section() { echo -e "\n${BLUE}=== $1 ===${NC}"; }

# All Python calls use relative paths from REPO_ROOT to avoid Git Bash path issues on Windows
cd "$REPO_ROOT"

# =============================================================================
section "1. Repository Structure"
# =============================================================================

# Check essential directories exist (post ADR-0013: workloads + infra, no components)
for dir in workloads infra schemas app app/src app/src-tauri; do
  if [ -d "$dir" ]; then
    pass "Directory exists: $dir"
  else
    fail "Missing directory: $dir"
  fi
done

# Check essential files exist
for file in schemas/component.schema.json compose.yml README.md; do
  if [ -f "$file" ]; then
    pass "File exists: $file"
  else
    fail "Missing file: $file"
  fi
done

# =============================================================================
section "2. JSON Schema Validation"
# =============================================================================

if python3 -c "import json; json.load(open('schemas/component.schema.json'))" 2>/dev/null; then
  pass "Schema is valid JSON"
else
  fail "Schema is not valid JSON"
fi

# Verify schema has all required sections
python3 -c "
import json, sys
schema = json.load(open('schemas/component.schema.json'))
props = schema.get('properties', {})
required_sections = ['identity', 'status', 'commands', 'configs', 'health', 'workflows']
missing = [s for s in required_sections if s not in props]
if missing:
    print('Missing sections: ' + ', '.join(missing))
    sys.exit(1)
" 2>/dev/null && pass "Schema has all 6 sections" || fail "Schema missing sections"

# =============================================================================
section "3. Component Manifests"
# =============================================================================

MANIFEST_COUNT=0
MANIFEST_ERRORS=0

for manifest in workloads/*/component.yml; do
  if [ ! -f "$manifest" ]; then
    continue
  fi
  component_dir="$(dirname "$manifest")"
  component_name="$(basename "$component_dir")"
  MANIFEST_COUNT=$((MANIFEST_COUNT+1))

  # Parse YAML
  if python3 -c "
import sys
try:
    import yaml
except ImportError:
    sys.exit(2)
yaml.safe_load(open('$manifest'))
" 2>/dev/null; then
    pass "Manifest parses: $component_name"
  else
    fail "Manifest parse error: $component_name"
    MANIFEST_ERRORS=$((MANIFEST_ERRORS+1))
    continue
  fi

  # Validate identity fields
  python3 -c "
import yaml, sys
m = yaml.safe_load(open('$manifest'))
identity = m.get('identity', {})
errors = []
for field in ['id', 'name', 'version', 'role']:
    if not identity.get(field):
        errors.append(field)
if errors:
    print('Missing: ' + ', '.join(errors))
    sys.exit(1)
role = identity['role']
if role not in ['runtime', 'toolchain', 'network', 'placeholder']:
    print(f'Invalid role: {role}')
    sys.exit(1)
" 2>/dev/null && pass "Identity valid: $component_name" || fail "Identity invalid: $component_name"

  # Cross-reference validation
  python3 -c "
import yaml, sys
m = yaml.safe_load(open('$manifest'))
errors = []

# Collect state IDs
state_ids = [s['id'] for s in m.get('status', {}).get('states', [])]
cmd_ids = [c['id'] for c in m.get('commands', [])]

# Check available_when references valid states
for cmd in m.get('commands', []):
    for aw in cmd.get('available_when', []):
        if aw not in state_ids:
            errors.append(f'Command \"{cmd[\"id\"]}\" references unknown state \"{aw}\"')

# Check restart_command references valid command
for cfg in m.get('configs', []):
    rc = cfg.get('restart_command')
    if rc and rc not in cmd_ids:
        errors.append(f'Config \"{cfg[\"path\"]}\" restart_command \"{rc}\" not in commands')

# Check command IDs are unique
seen = set()
for cid in cmd_ids:
    if cid in seen:
        errors.append(f'Duplicate command ID: \"{cid}\"')
    seen.add(cid)

# Check health probe IDs are unique
health_ids = set()
for h in m.get('health', []):
    if h['id'] in health_ids:
        errors.append(f'Duplicate health probe ID: \"{h[\"id\"]}\"')
    health_ids.add(h['id'])

if errors:
    for e in errors:
        print(e)
    sys.exit(1)
" 2>/dev/null && pass "Cross-references valid: $component_name" || fail "Cross-reference errors: $component_name"

  # Validate command groups and danger levels
  python3 -c "
import yaml, sys
m = yaml.safe_load(open('$manifest'))
valid_groups = {'lifecycle', 'operations', 'monitoring', 'maintenance'}
valid_danger = {'safe', 'caution', 'destructive'}
valid_types = {'action', 'query', 'stream'}
errors = []
for cmd in m.get('commands', []):
    g = cmd.get('group', 'operations')
    if g not in valid_groups:
        errors.append(f'Command \"{cmd[\"id\"]}\": invalid group \"{g}\"')
    d = cmd.get('danger', 'safe')
    if d not in valid_danger:
        errors.append(f'Command \"{cmd[\"id\"]}\": invalid danger \"{d}\"')
    t = cmd.get('type', 'action')
    if t not in valid_types:
        errors.append(f'Command \"{cmd[\"id\"]}\": invalid type \"{t}\"')
if errors:
    for e in errors:
        print(e)
    sys.exit(1)
" 2>/dev/null && pass "Command enums valid: $component_name" || fail "Command enum errors: $component_name"
done

if [ "$MANIFEST_COUNT" -eq 0 ]; then
  fail "No component manifests found"
else
  pass "Found $MANIFEST_COUNT component manifests"
fi

# Check for placeholder components
python3 -c "
import yaml, sys, os, glob
manifests = glob.glob('workloads/*/component.yml')
for m_path in manifests:
    m = yaml.safe_load(open(m_path))
    role = m.get('identity', {}).get('role')
    name = m.get('identity', {}).get('name', os.path.basename(os.path.dirname(m_path)))
    if role == 'placeholder':
        cmds = m.get('commands', [])
        if cmds:
            print(f'{name}: placeholder should have no commands (has {len(cmds)})')
            sys.exit(1)
" 2>/dev/null && pass "Placeholder components have no commands" || fail "Placeholder component has commands"

# =============================================================================
section "4. Monorepo workload layout (post ADR-0013)"
# =============================================================================

# Post ADR-0013: no submodules. Verify the flat workloads/ + infra/ layout exists.
if [ -f ".gitmodules" ]; then
  fail ".gitmodules should NOT exist (ADR-0013 removed submodules)"
else
  pass "No .gitmodules (monorepo layout)"
fi

# Each workload must have a component.yml manifest.
for workload in agent forge social; do
  if [ -f "workloads/$workload/component.yml" ]; then
    pass "Workload manifest present: $workload"
  else
    fail "Workload manifest missing: workloads/$workload/component.yml"
  fi
done

# infra/proxy/ uses the upstream mitmproxy image (pinned by digest, bind-mounted script),
# so it has no Containerfile — only a script + allowlist. infra/egress/ builds locally.
if [ -f "infra/proxy/vault-proxy.py" ] && [ -f "infra/proxy/allowlist.txt" ]; then
  pass "Infra proxy script + allowlist present"
else
  fail "Infra proxy script/allowlist missing"
fi
if [ -f "infra/egress/Containerfile" ]; then
  pass "Infra Containerfile present: egress"
else
  fail "Infra Containerfile missing: infra/egress/Containerfile"
fi

# =============================================================================
section "5. Build Verification"
# =============================================================================

# Check Rust compilation
if [ -d "app/src-tauri" ]; then
  if [ -d "app/src-tauri/target" ]; then
    pass "Rust target directory exists (previously built)"
  fi

  # Check Cargo.toml is valid
  if [ -f "app/src-tauri/Cargo.toml" ]; then
    pass "Cargo.toml exists"
  else
    fail "Cargo.toml missing"
  fi

  # Check tauri.conf.json is valid JSON
  if python3 -c "import json; json.load(open('app/src-tauri/tauri.conf.json'))" 2>/dev/null; then
    pass "tauri.conf.json is valid JSON"
  else
    fail "tauri.conf.json is not valid JSON"
  fi
fi

# Check Node.js project
if [ -f "app/package.json" ]; then
  pass "package.json exists"

  # Verify required dependencies
  python3 -c "
import json, sys
pkg = json.load(open('app/package.json'))
deps = {**pkg.get('dependencies', {}), **pkg.get('devDependencies', {})}
required = ['react', 'react-dom', 'react-router-dom', '@tauri-apps/api', 'tailwindcss', 'lucide-react', 'typescript']
missing = [d for d in required if d not in deps]
if missing:
    print('Missing deps: ' + ', '.join(missing))
    sys.exit(1)
" 2>/dev/null && pass "Required npm dependencies present" || fail "Missing npm dependencies"
else
  fail "package.json missing"
fi

# Check TypeScript config
if [ -f "app/tsconfig.json" ]; then
  pass "tsconfig.json exists"
else
  fail "tsconfig.json missing"
fi

# =============================================================================
section "6. Frontend-Backend Contract"
# =============================================================================

# Verify Rust command handlers match frontend invoke() calls
python3 -c "
import re, os, sys, glob

# Extract Rust #[tauri::command] function names. Recursive scan so
# top-level modules (e.g. status_aggregator.rs) are picked up alongside
# the commands/ submodule.
rust_commands = set()
for rs_file in glob.glob('app/src-tauri/src/**/*.rs', recursive=True):
    with open(rs_file) as f:
        content = f.read()
    # Find pub async fn NAME after #[tauri::command]
    for match in re.finditer(r'#\[tauri::command\]\s*pub\s+(?:async\s+)?fn\s+(\w+)', content):
        rust_commands.add(match.group(1))

# Extract frontend invoke() calls
frontend_commands = set()
for ts_file in glob.glob('app/src/lib/tauri.ts'):
    with open(ts_file) as f:
        content = f.read()
    for match in re.finditer(r'invoke[<(].*?\"(\w+)\"', content):
        frontend_commands.add(match.group(1))

# Check registered in lib.rs
with open('app/src-tauri/src/lib.rs') as f:
    lib_content = f.read()
registered = set(re.findall(r'(\w+)::\w+', lib_content.split('generate_handler!')[1].split(']')[0])) if 'generate_handler!' in lib_content else set()

# Compare
errors = []

# Frontend calls not in Rust
for cmd in frontend_commands:
    if cmd not in rust_commands:
        errors.append(f'Frontend invokes \"{cmd}\" but no Rust handler exists')

# Rust handlers not in frontend (info only)
for cmd in rust_commands:
    if cmd not in frontend_commands:
        errors.append(f'Rust handler \"{cmd}\" has no frontend invoke (may be unused)')

if errors:
    for e in errors:
        print(e)
    sys.exit(1)
" 2>/dev/null && pass "Frontend-backend command contract matches" || warn "Frontend-backend contract mismatch (see output above)"

# =============================================================================
section "7. Manifest-Schema Alignment"
# =============================================================================

# Check that manifest field names match what Rust serde expects
python3 -c "
import yaml, sys, glob

manifests = glob.glob('workloads/*/component.yml')
valid_output_displays = ['log', 'table', 'badge', 'checklist', 'card-grid', 'terminal', 'report']
valid_config_formats = ['yaml', 'json', 'json5', 'env', 'line-list']
valid_parse_types = ['regex', 'json_path', 'line_count', 'exit_code']

errors = []
for m_path in manifests:
    m = yaml.safe_load(open(m_path))
    name = m.get('identity', {}).get('id', 'unknown')

    # Check output.display values
    for cmd in m.get('commands', []):
        output = cmd.get('output', {})
        display = output.get('display')
        if display and display not in valid_output_displays:
            errors.append(f'{name}: command \"{cmd[\"id\"]}\" has invalid output.display \"{display}\"')

    # Check config.format values
    for cfg in m.get('configs', []):
        fmt = cfg.get('format')
        if fmt and fmt not in valid_config_formats:
            errors.append(f'{name}: config \"{cfg[\"path\"]}\" has invalid format \"{fmt}\"')

    # Check health parse types
    for h in m.get('health', []):
        pt = h.get('parse', {}).get('type')
        if pt and pt not in valid_parse_types:
            errors.append(f'{name}: health \"{h[\"id\"]}\" has invalid parse type \"{pt}\"')

if errors:
    for e in errors:
        print(e)
    sys.exit(1)
" 2>/dev/null && pass "All manifest enum values are valid" || fail "Invalid enum values in manifests"

# =============================================================================
section "8. Prerequisites Validation"
# =============================================================================

python3 -c "
import yaml, sys, glob

manifests = glob.glob('workloads/*/component.yml')
errors = []
for m_path in manifests:
    m = yaml.safe_load(open(m_path))
    name = m.get('identity', {}).get('id', 'unknown')
    prereqs = m.get('prerequisites')
    if not prereqs:
        continue

    cmd_ids = [c['id'] for c in m.get('commands', [])]

    # setup_command must reference a declared command
    setup_cmd = prereqs.get('setup_command')
    if setup_cmd and setup_cmd not in cmd_ids:
        errors.append(f'{name}: prerequisites.setup_command \"{setup_cmd}\" not in commands')

    # config_files paths should not escape component directory
    for cf in prereqs.get('config_files', []):
        path = cf.get('path', '')
        if '..' in path or path.startswith('/'):
            errors.append(f'{name}: prerequisites config_file path \"{path}\" looks unsafe')

if errors:
    for e in errors:
        print(e)
    sys.exit(1)
" 2>/dev/null && pass "Prerequisites cross-references valid" || fail "Prerequisites cross-reference errors"

# =============================================================================
section "9. Workflow Validation"
# =============================================================================

# Workflow step commands must reference valid command IDs
python3 -c "
import yaml, sys, glob

manifests = glob.glob('workloads/*/component.yml')
errors = []
total_workflows = 0
for m_path in manifests:
    m = yaml.safe_load(open(m_path))
    name = m.get('identity', {}).get('id', 'unknown')
    cmd_ids = {c['id'] for c in m.get('commands', [])}
    workflows = m.get('workflows', [])
    total_workflows += len(workflows)

    wf_ids = set()
    for wf in workflows:
        wf_id = wf.get('id', '')
        # Check unique workflow IDs within component
        if wf_id in wf_ids:
            errors.append(f'{name}: duplicate workflow id \"{wf_id}\"')
        wf_ids.add(wf_id)

        # Check step command references
        step_ids = {s['id'] for s in wf.get('steps', [])}
        for step in wf.get('steps', []):
            if step['command'] not in cmd_ids:
                errors.append(f'{name}: workflow \"{wf_id}\" step \"{step[\"id\"]}\" references unknown command \"{step[\"command\"]}\"')
            dep = step.get('depends_on')
            if dep and dep not in step_ids:
                errors.append(f'{name}: workflow \"{wf_id}\" step \"{step[\"id\"]}\" depends_on unknown step \"{dep}\"')

        # Check trigger enum
        trigger = wf.get('trigger', 'manual')
        valid_triggers = ['manual', 'on-demand', 'automatic', 'scheduled']
        if trigger not in valid_triggers:
            errors.append(f'{name}: workflow \"{wf_id}\" has invalid trigger \"{trigger}\"')

        # Check shell_requirement enum
        shell_req = wf.get('shell_requirement', 'any')
        valid_shells = ['hard', 'split', 'soft', 'any']
        if shell_req not in valid_shells:
            errors.append(f'{name}: workflow \"{wf_id}\" has invalid shell_requirement \"{shell_req}\"')

if errors:
    for e in errors:
        print(e)
    sys.exit(1)
print(f'{total_workflows} workflows validated')
" 2>/dev/null && pass "Workflow step→command references valid" || fail "Workflow cross-reference errors"

# Orchestrator workflows must reference valid component IDs and commands/workflows
python3 -c "
import yaml, sys, glob, os

# Load orchestrator workflows
orch_path = 'config/orchestrator-workflows.yml'
if not os.path.exists(orch_path):
    print('No orchestrator workflows file — skipping')
    sys.exit(0)

orch = yaml.safe_load(open(orch_path))
orch_wfs = orch.get('workflows', [])

# Load component data
components = {}
for m_path in glob.glob('workloads/*/component.yml'):
    m = yaml.safe_load(open(m_path))
    cid = m.get('identity', {}).get('id', 'unknown')
    components[cid] = {
        'commands': {c['id'] for c in m.get('commands', [])},
        'workflows': {w['id'] for w in m.get('workflows', [])},
    }

errors = []
for wf in orch_wfs:
    wf_id = wf.get('id', '')
    step_ids = {s['id'] for s in wf.get('steps', [])}
    for step in wf.get('steps', []):
        comp = step.get('component', '')
        if comp not in components:
            errors.append(f'orchestrator workflow \"{wf_id}\" step \"{step[\"id\"]}\" references unknown component \"{comp}\"')
            continue
        # Step references either a command or a workflow
        cmd = step.get('command')
        wf_ref = step.get('workflow')
        if cmd and cmd not in components[comp]['commands']:
            errors.append(f'orchestrator workflow \"{wf_id}\" step \"{step[\"id\"]}\" references unknown command \"{comp}.{cmd}\"')
        if wf_ref and wf_ref not in components[comp]['workflows']:
            errors.append(f'orchestrator workflow \"{wf_id}\" step \"{step[\"id\"]}\" references unknown workflow \"{comp}.{wf_ref}\"')
        if not cmd and not wf_ref:
            errors.append(f'orchestrator workflow \"{wf_id}\" step \"{step[\"id\"]}\" must have either command or workflow')
        dep = step.get('depends_on')
        if dep and dep not in step_ids:
            errors.append(f'orchestrator workflow \"{wf_id}\" step \"{step[\"id\"]}\" depends_on unknown step \"{dep}\"')

if errors:
    for e in errors:
        print(e)
    sys.exit(1)
print(f'{len(orch_wfs)} orchestrator workflows validated')
" 2>/dev/null && pass "Orchestrator workflow references valid" || fail "Orchestrator workflow cross-reference errors"

# =============================================================================
section "10. Five-container Perimeter Topology (ADR-0009)"
# =============================================================================
#
# Verifies the post-ADR-0009 perimeter shape:
#   - Five services declared in compose.yml
#   - vault-egress exists with NET_ADMIN, no API key env vars
#   - vault-proxy is NOT attached to external-net
#   - vault-egress IS the only service on external-net
#   - egress-net is internal and uses the documented 10.230.0.0/24 subnet

python3 - <<'PY' 2>/dev/null && pass "compose.yml declares five perimeter services" || fail "compose.yml service count is not five"
import sys, yaml
with open('compose.yml') as f:
    c = yaml.safe_load(f)
expected = {'vault-agent', 'vault-proxy', 'vault-forge', 'vault-social', 'vault-egress'}
got = set(c.get('services', {}).keys())
sys.exit(0 if got == expected else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "vault-egress declares NET_ADMIN; vault-proxy does NOT" || fail "Capability boundary between vault-egress and vault-proxy is broken"
import sys, yaml
with open('compose.yml') as f:
    c = yaml.safe_load(f)
svcs = c.get('services', {})
egress = svcs.get('vault-egress', {})
proxy  = svcs.get('vault-proxy',  {})
egress_caps = set(egress.get('cap_add', []) or [])
proxy_caps  = set(proxy.get('cap_add',  []) or [])
sys.exit(0 if ('NET_ADMIN' in egress_caps and 'NET_ADMIN' not in proxy_caps) else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "vault-proxy is NOT attached to external-net" || fail "vault-proxy still has external-net attachment (ADR-0009 violation)"
import sys, yaml
with open('compose.yml') as f:
    c = yaml.safe_load(f)
nets = c['services'].get('vault-proxy', {}).get('networks', []) or []
sys.exit(0 if 'external-net' not in nets else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "vault-egress is the only service on external-net" || fail "More than one service on external-net (ADR-0009 violation)"
import sys, yaml
with open('compose.yml') as f:
    c = yaml.safe_load(f)
on_external = [
    name for name, svc in c['services'].items()
    if 'external-net' in (svc.get('networks', []) or [])
]
sys.exit(0 if on_external == ['vault-egress'] else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "egress-net is internal and uses the documented 10.230.0.0/24 subnet" || fail "egress-net is not configured as internal 10.230.0.0/24"
import sys, yaml
with open('compose.yml') as f:
    c = yaml.safe_load(f)
egress_net = c.get('networks', {}).get('egress-net', {})
ipam = egress_net.get('ipam', {}).get('config', [{}])[0]
subnet_ok = ipam.get('subnet') == '10.230.0.0/24'
internal_ok = egress_net.get('internal') is True
sys.exit(0 if (subnet_ok and internal_ok) else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "vault-egress holds no API key env vars (secret-free)" || fail "vault-egress has API key env vars (must be secret-free per ADR-0009)"
import sys, yaml
with open('compose.yml') as f:
    c = yaml.safe_load(f)
env = c['services'].get('vault-egress', {}).get('environment', [])
# Accept env as either list or dict
keys = []
if isinstance(env, list):
    keys = [e.split('=', 1)[0] for e in env if isinstance(e, str)]
elif isinstance(env, dict):
    keys = list(env.keys())
banned = {'ANTHROPIC_API_KEY', 'OPENAI_API_KEY', 'TELEGRAM_BOT_TOKEN'}
sys.exit(0 if not (set(keys) & banned) else 1)
PY

# =============================================================================
section "11. Bot vocabulary hygiene (Zone 5 / B8)"
# =============================================================================
# Guards the CONSTRAINTS.md content that the agent reads on startup. The bot
# mimics what it reads, so any developer-jargon or raw container path in this
# file leaks straight into user-facing replies — that's the B8 failure mode
# from the 2026-05-20 E2E. The heredoc lives inside entrypoint.sh; we extract
# it and scan for banned tokens.

python3 - <<'PY' 2>/dev/null && pass "CONSTRAINTS.md heredoc has no raw container paths" || fail "CONSTRAINTS.md leaks internal container paths to the bot (e.g. /home/vault/.openclaw/...)"
import re, sys
src = open('workloads/agent/scripts/entrypoint.sh').read()
m = re.search(r"<<\s*'CONSTRAINTSEOF'\s*\n(.*?)\nCONSTRAINTSEOF", src, re.DOTALL)
if not m:
    sys.exit(2)
body = m.group(1)
# Match container-internal paths the bot would mimic. Requires meaningful
# content after the vault token (`.openclaw`, `workspace`, etc.) so bare-prefix
# tokens used as counter-examples in the "Do NOT say" table (e.g. `/vault/`,
# `/home/`) do NOT trigger.
if re.search(r'(/home/vault|/opt/vault|/var/log/vault|/vault)/[A-Za-z._][\w./_-]*', body):
    sys.exit(1)
sys.exit(0)
PY

python3 - <<'PY' 2>/dev/null && pass "CONSTRAINTS.md heredoc instructs the bot away from 'sandbox' / 'container' / 'vault' self-descriptions" || fail "CONSTRAINTS.md lacks an explicit vocabulary guard against 'sandbox' / 'container' / 'vault' (B8 fix)"
import re, sys
src = open('workloads/agent/scripts/entrypoint.sh').read()
m = re.search(r"<<\s*'CONSTRAINTSEOF'\s*\n(.*?)\nCONSTRAINTSEOF", src, re.DOTALL)
if not m:
    sys.exit(2)
body = m.group(1).lower()
# The fix must add an explicit "do not say these words" guidance. We look for
# a marker section + at least one banned term mentioned as a counter-example.
has_marker = ('do not use' in body or "don't use" in body or 'avoid these words' in body
              or 'never use these words' in body)
# At least one of the three banned self-descriptions called out.
mentions_banned = ('sandbox' in body or 'sandboxed' in body)
# Suggested replacement vocabulary must be present too — otherwise the bot
# has nothing to fall back to.
has_replacement = ('walled off' in body or 'kept separate' in body or 'protected room' in body)
sys.exit(0 if (has_marker and mentions_banned and has_replacement) else 1)
PY

# =============================================================================
section "12. Proxy log volume persistence (Zone 3)"
# =============================================================================
# vault-proxy.py writes requests.jsonl to /var/log/vault-proxy as a non-root
# user (mitmproxy). The named volume defaults to container-root ownership on
# rootless podman, so the addon silently falls back to in-container /tmp.
# The fix is podman's ':U' suffix on the mount, which chowns the volume to
# the container's user namespace mapping at mount time. Pin this in BOTH the
# shipped perimeter.yml mount declaration (via a chown flag) AND the dev
# compose.yml (via the ':U' syntax) so the bug can't quietly come back.

python3 - <<'PY' 2>/dev/null && pass "perimeter.yml vault-proxy-logs mount declares chown-on-mount" || fail "perimeter.yml vault-proxy-logs mount is missing 'chown: true' — non-root mitmproxy can't write the log volume (Zone 3 fix)"
import sys, yaml
with open('app/src-tauri/resources/perimeter.yml') as f:
    spec = yaml.safe_load(f)
mounts = spec['services'].get('vault-proxy', {}).get('volumes', []) or []
log_mount = next((m for m in mounts if m.get('source') == 'vault-proxy-logs'), None)
sys.exit(0 if log_mount and log_mount.get('chown') is True else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "compose.yml vault-proxy-logs mount uses ':U' chown-on-mount" || fail "compose.yml vault-proxy-logs mount lacks ':U' suffix — dev-mode podman compose hits the same write-fallback bug"
import sys, yaml
with open('compose.yml') as f:
    c = yaml.safe_load(f)
vols = c['services'].get('vault-proxy', {}).get('volumes', []) or []
hit = False
for v in vols:
    s = v if isinstance(v, str) else v.get('source', '')
    if isinstance(v, str) and v.startswith('vault-proxy-logs:'):
        hit = True
        if not (v.endswith(':U') or ':U,' in v or ':U:' in v):
            sys.exit(1)
sys.exit(0 if hit else 1)
PY

# =============================================================================
section "13. verify.sh + dogfood harness freshness (Zone 6a)"
# =============================================================================
# verify.sh is the architecture-invariant baseline the dogfood harness asserts
# on session-start and session-end (must be identical). Two staleness vectors:
#
# 1. verify.sh resolves the agent container by compose-service label. The
#    service used to be `vault` and is now `vault-agent` (ADR-0009 era rename).
#    If the script only looks up `vault`, it silently sets CONTAINER="" and
#    runs every `exec` against an empty container ID — the harness then sees
#    "all checks passed" for the wrong reason.
#
# 2. The dogfood CHECKLIST + findings-template instruct operators to run
#    `podman exec vault-agent /vault/scripts/verify.sh`. That path doesn't
#    exist in the container — verify.sh is a HOST-side script (it execs IN
#    via `$RUNTIME exec`). The correct invocation is
#    `bash workloads/agent/scripts/verify.sh` from the repo root.

python3 - <<'PY' 2>/dev/null && pass "verify.sh resolves the current vault-agent service name" || fail "verify.sh still resolves only the legacy 'vault' service name — CONTAINER comes back empty post-rename (Zone 6a fix)"
import re, sys
src = open('workloads/agent/scripts/verify.sh').read()
# Find the line that assigns CONTAINER from resolve_service_container.
# It must include 'vault-agent' (as primary, or in the fallback list).
m = re.search(r"CONTAINER=\$\(resolve_service_container\s+([^)]+)\)", src)
if not m:
    sys.exit(2)
args = m.group(1).split()
sys.exit(0 if 'vault-agent' in args else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "dogfood CHECKLIST + findings-template invoke verify.sh from the host" || fail "dogfood docs still tell operators to 'podman exec vault-agent /vault/scripts/verify.sh' — wrong path, verify.sh is a host-side script"
import sys, pathlib
bad_phrase = 'podman exec vault-agent /vault/scripts/verify.sh'
hits = []
for p in [
    pathlib.Path('tests/dogfood/CHECKLIST.md'),
    pathlib.Path('tests/dogfood/findings-template.md'),
]:
    if not p.exists():
        continue
    text = p.read_text()
    if bad_phrase in text:
        hits.append(str(p))
sys.exit(0 if not hits else 1)
PY

# =============================================================================
section "14. Forge spotlight (MISSION Thread D)"
# =============================================================================
# openskill-forge is the most novel piece of the project (per MISSION.md
# Thread D). It got buried under UI/UX issues during the v0.5.0 push. These
# checks pin the editorial work that lifts it back into view, so the spotlight
# can't quietly decay back into a one-line mention in the next refactor.

python3 - <<'PY' 2>/dev/null && pass "docs/forge-spotlight.md exists and is substantive" || fail "docs/forge-spotlight.md missing or too thin — Thread D spotlight isn't shipped"
import sys, pathlib
p = pathlib.Path('docs/forge-spotlight.md')
if not p.exists():
    sys.exit(1)
text = p.read_text()
# Substance check: long enough to be a real narrative, mentions the key
# distinguishing concepts (scanner + CDR + MITRE + ClawHavoc origin),
# and links to where forge actually lives now.
ok = (
    len(text) > 2000
    and 'Content Disarm' in text
    and ('MITRE' in text or 'ATT&CK' in text)
    and 'ClawHavoc' in text
    and 'workloads/forge' in text
)
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "README has a dedicated forge spotlight section linked to the narrative doc" || fail "README lacks a dedicated forge spotlight section or doesn't link to docs/forge-spotlight.md (Thread D)"
import re, sys, pathlib
readme = pathlib.Path('README.md').read_text()
# Require a Markdown heading whose text contains either 'forge' or 'skill scanner'
# (case-insensitive) AND the section links to forge-spotlight.md somewhere nearby.
# Simplest pin: a heading on a line containing forge/scanner concepts, and the
# doc reference appears somewhere in README.
has_heading = bool(re.search(r'^#+ .*(forge|skill scan|skill scanner|content disarm).*$',
                              readme, re.IGNORECASE | re.MULTILINE))
links_doc = 'forge-spotlight.md' in readme or 'docs/forge-spotlight.md' in readme
sys.exit(0 if (has_heading and links_doc) else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "workloads/forge/README.md is monorepo-aware (no stale 'four containers' or dead-repo links)" || fail "workloads/forge/README.md still reads as a separate submodule repo — update for the monorepo layout (Thread D)"
import sys, pathlib, re
text = pathlib.Path('workloads/forge/README.md').read_text()
problems = []
# Stale container count (we are five since ADR-0009).
if re.search(r'four containers|four services|4-container perimeter', text, re.IGNORECASE):
    problems.append('stale four-container language')
# Dead standalone-repo references (post ADR-0013).
if 'https://github.com/albertdobmeyer/opencli-container' in text:
    problems.append('links to archived opencli-container repo')
if 'https://github.com/albertdobmeyer/openagent-social' in text:
    problems.append('links to archived openagent-social repo')
# "This repository serves two roles" / "standalone toolchain" framing was the
# submodule-era pitch — incompatible with the monorepo layout.
if 'This repository serves two roles' in text:
    problems.append('residual two-roles framing from submodule era')
sys.exit(0 if not problems else 1)
PY

# =============================================================================
section "15. Skill-install flow honesty (Zone 4b)"
# =============================================================================
# Today the bot's CONSTRAINTS.md tells users to use the desktop app's
# "Browse the Skill Library" feature. That feature does not exist in the
# frontend (zero hits in app/src/). A4 in the 2026-05-20 dogfood findings
# confirms the install path is broken end-to-end. Zone 4b's job is twofold:
# (1) document the actual + intended install paths and the phasing between
# them, (2) stop the bot from promising vaporware.

python3 - <<'PY' 2>/dev/null && pass "docs/skill-install-flow.md exists and is substantive" || fail "docs/skill-install-flow.md missing or too thin — Zone 4b decision isn't documented"
import sys, pathlib
p = pathlib.Path('docs/skill-install-flow.md')
if not p.exists():
    sys.exit(1)
text = p.read_text()
ok = (
    len(text) > 2000
    and 'current state' in text.lower()
    and 'workloads/forge' in text
    and ('interim' in text.lower() or 'today' in text.lower())
    and ('GUI' in text or 'gui' in text or 'desktop app' in text.lower())
    # The doc must explicitly name the v0.6 / interim path so we don't
    # ship a doc that's only about the future.
    and ('cli' in text.lower() or 'terminal' in text.lower() or 'host' in text.lower())
)
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "CONSTRAINTS.md doesn't promise the not-yet-shipped 'Browse the Skill Library' feature" || fail "CONSTRAINTS.md tells the bot to refer to 'Browse the Skill Library' — that feature doesn't exist in app/src/ yet (Zone 4b honesty fix)"
import re, sys, pathlib
src = pathlib.Path('workloads/agent/scripts/entrypoint.sh').read_text()
m = re.search(r"<<\s*'CONSTRAINTSEOF'\s*\n(.*?)\nCONSTRAINTSEOF", src, re.DOTALL)
if not m:
    sys.exit(2)
body = m.group(1)
# Fail if the heredoc names the vaporware feature.
promises_vaporware = bool(
    re.search(r'browse the skill library', body, re.IGNORECASE)
    or "library-browse action" in body
)
sys.exit(0 if not promises_vaporware else 1)
PY

# =============================================================================
section "Summary"
# =============================================================================

TOTAL=$((PASS + FAIL + WARN))
echo ""
echo -e "Results: ${GREEN}$PASS passed${NC}, ${RED}$FAIL failed${NC}, ${YELLOW}$WARN warnings${NC} (total: $TOTAL checks)"

if [ "$FAIL" -gt 0 ]; then
  echo -e "\n${RED}ORCHESTRATION CHECK FAILED${NC}"
  echo "Fix the failures above before proceeding."
  exit 1
elif [ "$WARN" -gt 0 ]; then
  echo -e "\n${YELLOW}ORCHESTRATION CHECK PASSED WITH WARNINGS${NC}"
  exit 0
else
  echo -e "\n${GREEN}ORCHESTRATION CHECK PASSED${NC}"
  exit 0
fi
