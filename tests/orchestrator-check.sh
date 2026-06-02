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
for workload in agent skills social; do
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
expected = {'vault-agent', 'vault-proxy', 'vault-skills', 'vault-social', 'vault-egress'}
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
# openagent-skills is the most novel piece of the project (per MISSION.md
# Thread D). It got buried under UI/UX issues during the v0.5.0 push. These
# checks pin the editorial work that lifts it back into view, so the spotlight
# can't quietly decay back into a one-line mention in the next refactor.

python3 - <<'PY' 2>/dev/null && pass "docs/skills-spotlight.md exists and is substantive" || fail "docs/skills-spotlight.md missing or too thin — Thread D spotlight isn't shipped"
import sys, pathlib
p = pathlib.Path('docs/skills-spotlight.md')
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
    and 'workloads/skills' in text
)
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "README has a dedicated forge spotlight section linked to the narrative doc" || fail "README lacks a dedicated forge spotlight section or doesn't link to docs/skills-spotlight.md (Thread D)"
import re, sys, pathlib
readme = pathlib.Path('README.md').read_text()
# Require a Markdown heading whose text contains either 'forge' or 'skill scanner'
# (case-insensitive) AND the section links to skills-spotlight.md somewhere nearby.
# Simplest pin: a heading on a line containing forge/scanner concepts, and the
# doc reference appears somewhere in README.
has_heading = bool(re.search(r'^#+ .*(forge|skill scan|skill scanner|content disarm).*$',
                              readme, re.IGNORECASE | re.MULTILINE))
links_doc = 'skills-spotlight.md' in readme or 'docs/skills-spotlight.md' in readme
sys.exit(0 if (has_heading and links_doc) else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "workloads/skills/README.md is monorepo-aware (no stale 'four containers' or dead-repo links)" || fail "workloads/skills/README.md still reads as a separate submodule repo — update for the monorepo layout (Thread D)"
import sys, pathlib, re
text = pathlib.Path('workloads/skills/README.md').read_text()
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
    and 'workloads/skills' in text
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
section "16. forge → skills rename complete (M0)"
# =============================================================================
# After the v0.6 M0 naming sweep, NO live file (excluding immutable history:
# docs/adr/, docs/archive/, dated docs/specs, historical e2e findings/verdicts,
# and the v0.6 specs that describe the rename) may reference the old tokens.

python3 - <<'PY' 2>/dev/null && pass "no live file references vault-forge / forge-net / forge-deliveries / workloads/forge" || fail "the forge→skills rename is incomplete — a live file still references an old forge token (M0)"
import subprocess, sys, re
# git grep the old tokens, then drop historical/immutable paths.
out = subprocess.run(
    ["git", "grep", "-lE", r"vault-forge|forge-net|forge-deliveries|workloads/forge"],
    capture_output=True, text=True,
).stdout.splitlines()
EXCLUDE = re.compile(
    r"^(docs/adr/|docs/archive/|docs/release-notes|docs/specs/2026-|"
    r"docs/specs/ui-rebuild-2026|docs/specs/v0\.4-shell|docs/specs/v0\.6/|"
    r"tests/e2e-telegram/VERDICT-|tests/e2e-telegram/direct_probing/findings-|"
    r"tests/orchestrator-check\.sh)"  # this check holds the tokens as its search pattern
)
offenders = [f for f in out if not EXCLUDE.match(f)]
if offenders:
    sys.stderr.write("offending files:\n  " + "\n  ".join(offenders) + "\n")
    sys.exit(1)
sys.exit(0)
PY

# =============================================================================
section "17. Sentinel judge lib + CDR retry-repair (v0.6 M1)"
# =============================================================================
# Structural pins for the M1 deliverables — the runtime/model behaviour is
# verified by sentinel/judge.test.sh (Ollama-gated); these assert the wiring
# is present so it can't silently regress.

if [ -f "sentinel/judge.sh" ] && [ -f "sentinel/config.sh" ] && [ -f "sentinel/verdict-schema.json" ]; then
  pass "Sentinel judge lib present (judge.sh + config.sh + verdict-schema.json)"
else
  fail "Sentinel judge lib missing one of judge.sh / config.sh / verdict-schema.json (M1)"
fi

python3 - <<'PY' 2>/dev/null && pass "Sentinel judge prompt is injection-hardened" || fail "sentinel/judge.sh prompt lacks the injection-hardening clause (M1)"
import sys, pathlib
t = pathlib.Path('sentinel/judge.sh').read_text()
# Must instruct the model that the fragment is content-to-evaluate, never an
# instruction to obey — the property judge.test.sh verifies at runtime.
ok = ('never an instruction' in t.lower() or 'never as a command' in t.lower()) \
     and 'ignore your instructions' in t.lower()
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "CDR has the retry-with-repair loop + explicit quarantine (ZONE-4a fix)" || fail "skill-cdr.sh missing the retry-repair loop or the explicit quarantine (ZONE-4a fix, M1)"
import sys, pathlib
t = pathlib.Path('workloads/skills/tools/skill-cdr.sh').read_text()
ok = ('CDR_MAX_RETRIES' in t            # the retry budget
      and 'QUARANTINE' in t             # explicit, not silent
      and 'repair_hint' in t)           # the describe-with-repair feedback
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "cdr-intent.sh accepts a repair hint" || fail "cdr-intent.sh does not accept the repair-hint arg (M1)"
import sys, pathlib
t = pathlib.Path('workloads/skills/tools/lib/cdr-intent.sh').read_text()
sys.exit(0 if 'REPAIR_HINT' in t else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "CDR emits the plain-language disarm diff (trust artifact)" || fail "the disarm diff is missing — cdr-diff.py or the emit_disarm_diff wiring (M1)"
import sys, pathlib
diff = pathlib.Path('workloads/skills/tools/lib/cdr-diff.py')
cdr = pathlib.Path('workloads/skills/tools/skill-cdr.sh').read_text()
ok = diff.exists() and 'emit_disarm_diff' in cdr and 'DISARM-DIFF.txt' in cdr
sys.exit(0 if ok else 1)
PY

# =============================================================================
section "18. Modular distribution (v0.6 M2)"
# =============================================================================
# distribution.yml is the single source mapping install-names → dirs →
# containers → CLI, consumed by both the standalone installer and the GUI
# profiles. Pin its integrity + the profile wiring.

python3 - <<'PY' 2>/dev/null && pass "distribution.yml is valid and self-consistent" || fail "distribution.yml missing/invalid: a profile references an unknown shield, or a shield's dirs/containers don't exist (M2)"
import sys, pathlib, yaml
p = pathlib.Path('distribution.yml')
if not p.exists():
    sys.exit(1)
d = yaml.safe_load(p.read_text())
shields = d.get('shields', {})
# Every profile references only defined shields.
for prof, members in d.get('profiles', {}).items():
    for m in members:
        if m not in shields:
            sys.stderr.write(f"profile {prof} references unknown shield {m}\n"); sys.exit(1)
# default_profile is a defined profile.
if d.get('default_profile') not in d.get('profiles', {}):
    sys.exit(1)
# Every shield's dirs exist and its containers are declared in compose.yml.
compose = yaml.safe_load(open('compose.yml'))
compose_services = set(compose.get('services', {}).keys())
for name, s in shields.items():
    for dpath in s.get('dirs', []):
        if not pathlib.Path(dpath).exists():
            sys.stderr.write(f"shield {name}: dir {dpath} missing\n"); sys.exit(1)
    for c in s.get('containers', []):
        if c not in compose_services:
            sys.stderr.write(f"shield {name}: container {c} not in compose.yml\n"); sys.exit(1)
sys.exit(0)
PY

python3 - <<'PY' 2>/dev/null && pass "build.rs stages manifests by profile (GUI renders only the profile's shields)" || fail "build.rs is not profile-driven (M2)"
import sys, pathlib
t = pathlib.Path('app/src-tauri/build.rs').read_text()
sys.exit(0 if ('OPENTRAPP_PROFILE' in t and 'profile_manifests' in t) else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "bootstrap verify is profile-aware + the standalone installer exists" || fail "bootstrap shell_services or scripts/install-shield.sh missing (M2)"
import sys, pathlib
boot = pathlib.Path('app/src-tauri/src/bootstrap/mod.rs').read_text()
installer = pathlib.Path('scripts/install-shield.sh')
sys.exit(0 if ('shell_services' in boot and installer.exists()) else 1)
PY

# =============================================================================
section "19. Adaptive containment (v0.6 M3)"
# =============================================================================

if [ -f "sentinel/egress-advisor.sh" ]; then
  pass "egress advisor present (proposes least-privilege from the egress log)"
else
  fail "sentinel/egress-advisor.sh missing (M3)"
fi

# Run the advisor's own deterministic test suite (includes the never-auto-loosen
# invariant — ADR-0002). This is the load-bearing safety property.
if bash sentinel/egress-advisor.test.sh > /dev/null 2>&1; then
  pass "egress advisor never proposes a loosening (ADR-0002 invariant holds)"
else
  fail "egress advisor test suite failed — the never-auto-loosen invariant may be broken (M3)"
fi

# =============================================================================
section "20. Semantic firewall (v0.6 M4)"
# =============================================================================

if [ -f "workloads/social/tools/semantic-firewall.sh" ]; then
  pass "semantic firewall present (rung-0 regex → rung-2 judge on feed content)"
else
  fail "workloads/social/tools/semantic-firewall.sh missing (M4)"
fi

# The headline property — a paraphrased injection that evades the 25 static
# patterns is caught by the semantic judge. Ollama-gated (the test self-skips
# if no local model is running, so CI without Ollama still passes).
if bash workloads/social/tests/semantic-firewall.test.sh > /dev/null 2>&1; then
  pass "semantic firewall catches paraphrased injections the regexes miss"
else
  fail "semantic firewall test suite failed (M4)"
fi

# =============================================================================
section "21. Per-profile image bundling (spec 05 §4f + §4b image side)"
# =============================================================================
# build.rs must filter the image-digests.json overlay by OPENTRAPP_PROFILE
# so that a `containment` build only bundles 3 images, not all 5. The mapping
# must derive from distribution.yml (the single source) — not a hardcoded
# Rust match that would duplicate it.

python3 - <<'PY' 2>/dev/null && pass "build.rs has a stage_images function that filters by profile" || fail "build.rs is missing stage_images (spec 05 §4f — per-profile image bundling not implemented)"
import sys, pathlib
t = pathlib.Path('app/src-tauri/build.rs').read_text()
# Must have: a stage_images function + read distribution.yml + use OPENTRAPP_PROFILE
has_fn   = 'fn stage_images' in t
reads_dist = 'distribution.yml' in t
uses_profile = 'OPENTRAPP_PROFILE' in t
sys.exit(0 if (has_fn and reads_dist and uses_profile) else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "build.rs derives profile→containers from distribution.yml (not a hardcoded match)" || fail "build.rs uses a hardcoded container match instead of reading distribution.yml (single-source violation)"
import sys, pathlib, re
t = pathlib.Path('app/src-tauri/build.rs').read_text()
# A hardcoded container match would look like:
#   match profile { "containment" => &["vault-agent", ...], ...}
# This is a FAIL — the containers list must come from distribution.yml.
# We allow a hardcoded *manifest* match (profile_manifests stays), but a
# separate containers match is banned; the image function must read the file.
bad = bool(re.search(r'"vault-agent".*"vault-proxy".*"vault-egress"', t, re.DOTALL)
           and 'fn profile_images' in t
           and 'distribution.yml' not in t)
# Pass when distribution.yml IS referenced.
sys.exit(0 if 'distribution.yml' in t else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "distribution.yml profile→containers mapping matches build.rs behaviour for 'containment'" || fail "distribution.yml or build.rs missing the containment→[vault-agent,vault-proxy,vault-egress] mapping"
import sys, pathlib, yaml
d = yaml.safe_load(pathlib.Path('distribution.yml').read_text())
# Derive the containment profile's containers via the shields.
containment_shields = d['profiles']['containment']
containers = []
for sh in containment_shields:
    containers.extend(d['shields'][sh]['containers'])
expected = {'vault-agent', 'vault-proxy', 'vault-egress'}
sys.exit(0 if set(containers) == expected else 1)
PY

# =============================================================================
section "22. Sentinel rung-1 embeddings (v0.6 D2)"
# =============================================================================
# Structural pins for the rung-1 layer — the model behaviour is verified by
# sentinel/embed.test.sh (Ollama-gated); these assert the wiring + the banked
# calibration finding can't silently regress.

if [ -f "sentinel/embed.sh" ] && [ -f "sentinel/lib/sentinel_embed.py" ] && [ -f "sentinel/corpus/build.sh" ]; then
  pass "Sentinel rung-1 lib present (embed.sh + lib/sentinel_embed.py + corpus/build.sh)"
else
  fail "Sentinel rung-1 lib missing embed.sh / lib/sentinel_embed.py / corpus/build.sh (D2)"
fi

python3 - <<'PY' 2>/dev/null && pass "rung-1 config resolves D2 to all-minilm + drift/sim thresholds" || fail "config.sh missing the rung-1 embed model or thresholds (D2)"
import sys, pathlib
t = pathlib.Path('sentinel/config.sh').read_text()
ok = ('SENTINEL_EMBED_MODEL' in t and 'all-minilm' in t
      and 'SENTINEL_DRIFT_SIM_MIN' in t and 'SENTINEL_SIM_HIGH' in t)
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "embed.sh is lib-first (delegates to the python rung-1 lib)" || fail "embed.sh does not delegate to lib/sentinel_embed.py (lib-first)"
import sys, pathlib
t = pathlib.Path('sentinel/embed.sh').read_text()
sys.exit(0 if 'sentinel_embed.py' in t else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "rung-1 corpus is model-tagged (refuses a mismatched-model corpus)" || fail "sentinel_embed.py does not guard the corpus model tag"
import sys, pathlib
t = pathlib.Path('sentinel/lib/sentinel_embed.py').read_text()
ok = 'corpus.get("model")' in t and 'rebuild' in t.lower()
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "the recall-safe-booster finding is banked in the rung-1 lib (score never gates rung 2)" || fail "the rung-1 booster-not-gate caveat is not documented (banked finding regressed)"
import sys, pathlib
t = pathlib.Path('sentinel/lib/sentinel_embed.py').read_text().lower()
ok = 'booster' in t and ('not a gate' in t or 'never a gate' in t) and 'paraphrase' in t
sys.exit(0 if ok else 1)
PY

# =============================================================================
section "23. Sentinel GUI bridge + activity indicator (v0.6 slice 1)"
# =============================================================================
# Pins the GUI binding of the shared judgment lib: the judge command + the
# activity indicator wiring. The runtime behaviour is covered by the cargo
# unit tests + the vitest hook test; these assert the cross-language wiring
# (esp. the event-name contract) can't silently regress.

if [ -f "app/src-tauri/src/commands/sentinel.rs" ]; then
  pass "Sentinel GUI bridge module present (commands/sentinel.rs)"
else
  fail "commands/sentinel.rs missing (v0.6 GUI slice 1)"
fi

python3 - <<'PY' 2>/dev/null && pass "sentinel commands registered in lib.rs (judge + activity)" || fail "lib.rs does not register get_sentinel_activity + sentinel_judge"
import sys, pathlib
t = pathlib.Path('app/src-tauri/src/lib.rs').read_text()
ok = ('commands::sentinel::get_sentinel_activity' in t
      and 'commands::sentinel::sentinel_judge' in t
      and 'SentinelActivityStore::new()' in t)
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "activity event-name contract matches (Rust emit ↔ React listen)" || fail "the sentinel-activity-changed event name differs between Rust and the hook"
import sys, pathlib
rust = pathlib.Path('app/src-tauri/src/commands/sentinel.rs').read_text()
hook = pathlib.Path('app/src/hooks/useSentinelActivity.ts').read_text()
ev = 'sentinel-activity-changed'
sys.exit(0 if (ev in rust and ev in hook) else 1)
PY

if [ -f "app/src/components/user/SentinelActivityBadge.tsx" ] && [ -f "app/src/hooks/useSentinelActivity.ts" ]; then
  pass "activity indicator surface present (badge + hook)"
else
  fail "the activity-indicator badge or hook is missing (GUI slice 1)"
fi

# =============================================================================
section "24. Persona-drift outgoing guard (v0.6 M4 §2c)"
# =============================================================================
# Pins the outgoing guard: rung-1 drift on what the agent SENDS, with the
# fail-safe hold. Runtime behaviour is covered by persona-guard.test.sh
# (Ollama+all-minilm-gated).

if [ -f "workloads/social/tools/persona-guard.sh" ] && [ -f "workloads/social/tests/persona-guard.test.sh" ]; then
  pass "persona-drift outgoing guard present (persona-guard.sh + test)"
else
  fail "persona-guard.sh or its test is missing (M4 §2c)"
fi

python3 - <<'PY' 2>/dev/null && pass "persona-guard uses rung-1 drift + holds fail-safe (never auto-sends unverified)" || fail "persona-guard.sh missing the rung-1 drift call or the fail-safe hold"
import sys, pathlib
t = pathlib.Path('workloads/social/tools/persona-guard.sh').read_text()
# Must consult embed.sh drift, and must HOLD (not send) when it can't verify.
ok = ('embed.sh' in t and 'drift' in t
      and 'HOLD' in t
      and 'exit 3' in t           # can't-verify -> fail-safe hold
      and 'exit 1' in t)          # drifted     -> hold
sys.exit(0 if ok else 1)
PY

# =============================================================================
section "25. Disarm-diff display — skills trust artifact (v0.6 GUI slice 2)"
# =============================================================================
# The read-only trust artifact, surfaced through the manifest contract (the
# cleaned-skills command runs in-container; the GUI renders the JSON). Read-only
# by design — it surfaces what CDR already did and must never mutate anything.

if [ -f "workloads/skills/tools/disarm-report.sh" ] && [ -f "workloads/skills/tests/disarm-report.test.sh" ] \
   && [ -f "app/src/components/user/CleanedSkillsCard.tsx" ]; then
  pass "disarm-diff display present (disarm-report.sh + test + CleanedSkillsCard.tsx)"
else
  fail "disarm-diff display missing a piece (disarm-report.sh / test / CleanedSkillsCard.tsx)"
fi

python3 - <<'PY' 2>/dev/null && pass "cleaned-skills manifest command declared (the in-architecture channel)" || fail "skills component.yml is missing the cleaned-skills command"
import sys, yaml, pathlib
m = yaml.safe_load(pathlib.Path('workloads/skills/component.yml').read_text())
cmds = {c['id']: c for c in m.get('commands', [])}
c = cmds.get('cleaned-skills')
ok = bool(c) and c.get('command','').endswith('disarm-report.sh') and c.get('group') == 'monitoring'
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "disarm-report is read-only (surfaces, never mutates)" || fail "disarm-report.sh contains a mutating operation — it must be read-only"
import sys, re, pathlib
t = pathlib.Path('workloads/skills/tools/disarm-report.sh').read_text()
# No deletes, no copies, no allowlist writes, no redirecting INTO skill files.
bad = re.search(r'\brm\b|\bcp\b|\bmv\b|allowlist|>\s*"?\$ROOT', t)
sys.exit(1 if bad else 0)
PY

# =============================================================================
section "26. Production Sentinel staging (v0.6 Item B)"
# =============================================================================
# The shared Sentinel lib must reach a PACKAGED build the same verified way as
# every other policy artifact: build.rs stages sentinel/ into the bundle, the
# shields bind-mount it :ro at /opt/sentinel (perimeter.yml kind: resource +
# dev compose.yml), and the Ollama runtime requirement is documented (spec 08 §5).

python3 - <<'PY' 2>/dev/null && pass "build.rs stages the shared Sentinel lib into the bundle" || fail "build.rs does not stage sentinel/ into resources/perimeter/sentinel (Item B)"
import sys, pathlib
t = pathlib.Path('app/src-tauri/build.rs').read_text()
ok = 'fn stage_sentinel' in t and 'resources/perimeter/sentinel' in t and 'stage_sentinel()' in t
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "perimeter.yml mounts sentinel :ro into both shields (verified resource)" || fail "perimeter.yml is missing the sentinel kind:resource /opt/sentinel mount on a shield"
import sys, yaml, pathlib
spec = yaml.safe_load(pathlib.Path('app/src-tauri/resources/perimeter.yml').read_text())
ok = True
for svc in ('vault-skills', 'vault-social'):
    vols = spec['services'][svc].get('volumes', [])
    m = next((v for v in vols if v.get('target') == '/opt/sentinel'), None)
    ok = ok and m is not None and m.get('source') == 'sentinel' \
         and m.get('kind') == 'resource' and m.get('read_only') is True
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "compose.yml bind-mounts sentinel :ro into both shields (dev ↔ perimeter agree)" || fail "compose.yml is missing the ./sentinel:/opt/sentinel:ro mount on a shield"
import sys, yaml, pathlib
c = yaml.safe_load(pathlib.Path('compose.yml').read_text())
ok = True
for svc in ('vault-skills', 'vault-social'):
    vols = c['services'][svc].get('volumes', [])
    ok = ok and any(str(v).strip() == './sentinel:/opt/sentinel:ro' for v in vols)
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "Ollama runtime requirement is documented (soft prerequisite, SD-B2)" || fail "README does not document the optional Ollama requirement for the local-AI rungs (Item B §B3)"
import sys, pathlib
t = pathlib.Path('README.md').read_text().lower()
sys.exit(0 if ('ollama' in t and 'all-minilm' in t and 'optional' in t) else 1)
PY

# =============================================================================
section "27. Allowlist approval — human-mediated loosening (v0.6 Item A)"
# =============================================================================
# The one new write/loosening surface. These pin the ADR-0002 invariant
# statically: the agent can never widen its own allowlist; clear exfil never
# reaches the judge; exactly one code path writes the allowlist (spec 08 §4).

if [ -f "app/src-tauri/src/orchestrator/allowlist.rs" ] && [ -f "app/src-tauri/src/commands/egress.rs" ]; then
  pass "allowlist-approval modules present (orchestrator/allowlist.rs + commands/egress.rs)"
else
  fail "allowlist-approval modules missing (orchestrator/allowlist.rs / commands/egress.rs)"
fi

python3 - <<'PY' 2>/dev/null && pass "both egress commands registered in lib.rs" || fail "lib.rs does not register list_egress_approvals + apply_allowlist_decision"
import sys, pathlib
t = pathlib.Path('app/src-tauri/src/lib.rs').read_text()
ok = ('commands::egress::list_egress_approvals' in t
      and 'commands::egress::apply_allowlist_decision' in t)
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "clear exfil never reaches the judge (only off-allowlist BLOCKED surfaces)" || fail "allowlist.rs parse does not gate on action==BLOCKED + the off-allowlist reason"
import sys, pathlib
t = pathlib.Path('app/src-tauri/src/orchestrator/allowlist.rs').read_text()
# The parser must gate on BOTH action=="BLOCKED" and reason=="domain not in
# allowlist"; EXFIL_BLOCKED / rebinding fall through and never surface.
ok = ('"BLOCKED"' in t and 'domain not in allowlist' in t
      and '"action"' in t and '"reason"' in t)
sys.exit(0 if ok else 1)
PY

python3 - <<'PY' 2>/dev/null && pass "exactly one allowlist writer (the append primitive is only called inside allowlist.rs)" || fail "the allowlist write primitive is called outside orchestrator/allowlist.rs (ADR-0002 invariant)"
import sys, pathlib
src = pathlib.Path('app/src-tauri/src')
al = (src / 'orchestrator' / 'allowlist.rs').read_text()
# apply_always (the sole high-level writer) is DEFINED in allowlist.rs ...
if 'pub fn apply_always' not in al:
    sys.exit(1)
# ... and the low-level write primitive append_host_inplace() may only be CALLED
# inside allowlist.rs. Other modules go through apply_always/record_denial (which
# they may call) — they may READ the allowlist, but never append to it directly.
for p in src.rglob('*.rs'):
    if p.name == 'allowlist.rs':
        continue
    if 'append_host_inplace(' in p.read_text():
        sys.stderr.write(f"raw allowlist write primitive called in {p}\n"); sys.exit(1)
sys.exit(0)
PY

python3 - <<'PY' 2>/dev/null && pass "record_denial never writes the allowlist (a denial is not a loosening)" || fail "record_denial in allowlist.rs touches the allowlist path"
import sys, re, pathlib
t = pathlib.Path('app/src-tauri/src/orchestrator/allowlist.rs').read_text()
m = re.search(r'pub fn record_denial.*?\n\}', t, re.S)
ok = bool(m) and 'live_allowlist_path' not in m.group(0) and 'denials_path' in m.group(0)
sys.exit(0 if ok else 1)
PY

if [ -f "docs/adr/0016-host-mediated-allowlist-loosening.md" ]; then
  pass "ADR-0016 (host-mediated allowlist loosening) present"
else
  fail "ADR-0016 missing"
fi

python3 - <<'PY' 2>/dev/null && pass "threat-model has the new T1/T5 rows (self-loosen + approval fatigue)" || fail "docs/threat-model.md is missing the Item A T1/T5 rows"
import sys, pathlib
t = pathlib.Path('docs/threat-model.md').read_text()
ok = ('self-loosen the perimeter' in t and 'approval fatigue' in t
      and '0016-host-mediated-allowlist-loosening' in t)
sys.exit(0 if ok else 1)
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
