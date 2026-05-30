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
