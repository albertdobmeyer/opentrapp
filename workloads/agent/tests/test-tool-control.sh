#!/usr/bin/env bash
# Test: Tool Control System — config generation and security enforcement
#
# These tests run on the HOST only (no container needed).
# They validate that the config generator produces correct, secure output.
set -uo pipefail

VAULT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CORE="$VAULT_DIR/scripts/tool-control-core.py"
MANIFEST="$VAULT_DIR/config/tool-manifest.yml"

PASS=0
FAIL=0

echo "=== Tool Control Tests ==="
echo ""

check() {
    local desc="$1"
    shift
    printf "  %-55s " "$desc"
    local output
    output=$("$@" 2>&1) && exit_code=0 || exit_code=$?
    if [ $exit_code -eq 0 ]; then
        echo "PASS"
        PASS=$((PASS + 1))
    else
        echo "FAIL"
        echo "       $output" | head -5
        FAIL=$((FAIL + 1))
    fi
}

check_fail() {
    local desc="$1"
    shift
    printf "  %-55s " "$desc"
    local output
    output=$("$@" 2>&1) && exit_code=0 || exit_code=$?
    if [ $exit_code -ne 0 ]; then
        echo "PASS (rejected as expected)"
        PASS=$((PASS + 1))
    else
        echo "FAIL (should have been rejected)"
        echo "       $output" | head -3
        FAIL=$((FAIL + 1))
    fi
}

# --- Prerequisites ---
echo "  Prerequisites:"
check "Python3 available" python3 --version
check "PyYAML available" python3 -c "import yaml"
check "Manifest parses" python3 -c "import yaml; yaml.safe_load(open('$MANIFEST'))"
check "Core script compiles" python3 -c "import py_compile; py_compile.compile('$CORE', doraise=True)"
echo ""

# --- Preset: Hard Shell ---
echo "  Hard Shell preset:"
HARD=$(python3 "$CORE" --manifest "$MANIFEST" --preset hard --output config 2>&1)

check "Hard: valid JSON" python3 -c "import json; json.loads('''$HARD''')"

check "Hard: profile = minimal" python3 -c "
import json
c = json.loads('''$HARD''')
assert c['tools']['profile'] == 'minimal', f'got {c[\"tools\"][\"profile\"]}'
"

check "Hard: exec.security = deny" python3 -c "
import json
c = json.loads('''$HARD''')
assert c['tools']['exec']['security'] == 'deny', f'got {c[\"tools\"][\"exec\"][\"security\"]}'
"

check "Hard: no safeBins" python3 -c "
import json
c = json.loads('''$HARD''')
assert 'safeBins' not in c['tools']['exec'], 'safeBins should not be present'
"

check "Hard: elevated = false" python3 -c "
import json
c = json.loads('''$HARD''')
assert c['tools']['elevated']['enabled'] == False
"

check "Hard: sandbox = off" python3 -c "
import json
c = json.loads('''$HARD''')
assert c['agents']['defaults']['sandbox']['mode'] == 'off'
"
echo ""

# --- Preset: Split Shell ---
echo "  Split Shell preset:"
SPLIT=$(python3 "$CORE" --manifest "$MANIFEST" --preset split --output config 2>&1)

check "Split: valid JSON" python3 -c "import json; json.loads('''$SPLIT''')"

check "Split: profile = coding" python3 -c "
import json
c = json.loads('''$SPLIT''')
assert c['tools']['profile'] == 'coding', f'got {c[\"tools\"][\"profile\"]}'
"

check "Split: exec.security = allowlist" python3 -c "
import json
c = json.loads('''$SPLIT''')
assert c['tools']['exec']['security'] == 'allowlist'
"

check "Split: exec.ask = always" python3 -c "
import json
c = json.loads('''$SPLIT''')
assert c['tools']['exec']['ask'] == 'always'
"

check "Split: exec.host = gateway" python3 -c "
import json
c = json.loads('''$SPLIT''')
assert c['tools']['exec']['host'] == 'gateway'
"

check "Split: safeBins count matches manifest" python3 -c "
import json, yaml
c = json.loads('''$SPLIT''')
m = yaml.safe_load(open('$MANIFEST'))
expected = len(m['presets']['split']['safeBins'])
actual = len(c['tools']['exec']['safeBins'])
assert actual == expected, f'got {actual}, manifest says {expected}'
"

check "Split: safeBins match safeBinProfiles" python3 -c "
import json
c = json.loads('''$SPLIT''')
bins = set(c['tools']['exec']['safeBins'])
profs = set(c['tools']['exec']['safeBinProfiles'].keys())
assert bins == profs, f'mismatch: bins-profs={bins-profs} profs-bins={profs-bins}'
"

check "Split: rm NOT in safeBins" python3 -c "
import json
c = json.loads('''$SPLIT''')
assert 'rm' not in c['tools']['exec']['safeBins'], 'rm must not be in safeBins'
"

check "Split: elevated = false" python3 -c "
import json
c = json.loads('''$SPLIT''')
assert c['tools']['elevated']['enabled'] == False
"

check "Split: workspaceOnly = true" python3 -c "
import json
c = json.loads('''$SPLIT''')
assert c['tools']['fs']['workspaceOnly'] == True
"
echo ""

# --- Preset: Soft Shell (F11 — alsoAllow for tools with empty profiles) ---
echo "  Soft Shell preset:"
SOFT=$(python3 "$CORE" --manifest "$MANIFEST" --preset soft --output config 2>&1)

check "Soft: valid JSON" python3 -c "import json; json.loads('''$SOFT''')"

check "Soft: profile = coding" python3 -c "
import json
c = json.loads('''$SOFT''')
assert c['tools']['profile'] == 'coding', f'got {c[\"tools\"][\"profile\"]}'
"

check "Soft: exec.security = allowlist" python3 -c "
import json
c = json.loads('''$SOFT''')
assert c['tools']['exec']['security'] == 'allowlist'
"

check "Soft: exec.ask = on-miss" python3 -c "
import json
c = json.loads('''$SOFT''')
assert c['tools']['exec']['ask'] == 'on-miss', f'got {c[\"tools\"][\"exec\"][\"ask\"]}'
"

# F11 fix: tools with profiles:[] in OpenClaw must be alsoAllow'd explicitly.
check "Soft: alsoAllow contains web_fetch (F11)" python3 -c "
import json
c = json.loads('''$SOFT''')
assert 'web_fetch' in c['tools'].get('alsoAllow', []), f'alsoAllow={c[\"tools\"].get(\"alsoAllow\")}'
"

check "Soft: alsoAllow contains web_search (F11)" python3 -c "
import json
c = json.loads('''$SOFT''')
assert 'web_search' in c['tools'].get('alsoAllow', [])
"

check "Soft: alsoAllow contains cron (F11)" python3 -c "
import json
c = json.loads('''$SOFT''')
assert 'cron' in c['tools'].get('alsoAllow', [])
"

check "Soft: alsoAllow contains canvas (F11)" python3 -c "
import json
c = json.loads('''$SOFT''')
assert 'canvas' in c['tools'].get('alsoAllow', [])
"

check "Soft: alsoAllow contains message (F11)" python3 -c "
import json
c = json.loads('''$SOFT''')
assert 'message' in c['tools'].get('alsoAllow', [])
"

check "Soft: alsoAllow tools are subset of enabled_tools" python3 -c "
import json, yaml
c = json.loads('''$SOFT''')
m = yaml.safe_load(open('$MANIFEST'))
also = set(c['tools'].get('alsoAllow', []))
enabled = set(m['presets']['soft']['enabled_tools'])
assert also.issubset(enabled), f'alsoAllow has tools not in enabled_tools: {also - enabled}'
"

check "Soft: alsoAllow does not overlap deny list" python3 -c "
import json
c = json.loads('''$SOFT''')
also = set(c['tools'].get('alsoAllow', []))
deny = set(c['tools']['deny'])
assert not (also & deny), f'alsoAllow overlaps deny: {also & deny}'
"

check "Soft: gateway not in alsoAllow (NEVER)" python3 -c "
import json
c = json.loads('''$SOFT''')
assert 'gateway' not in c['tools'].get('alsoAllow', [])
"

check "Soft: bash not in alsoAllow (NEVER)" python3 -c "
import json
c = json.loads('''$SOFT''')
assert 'bash' not in c['tools'].get('alsoAllow', [])
"

check "Hard: no alsoAllow field (or empty)" python3 -c "
import json
c = json.loads('''$HARD''')
also = c['tools'].get('alsoAllow', [])
assert also == [], f'unexpected alsoAllow in Hard: {also}'
"

check "Split: no alsoAllow field (or empty)" python3 -c "
import json
c = json.loads('''$SPLIT''')
also = c['tools'].get('alsoAllow', [])
assert also == [], f'unexpected alsoAllow in Split: {also}'
"

check "Manifest: soft preset has also_allow field (F11)" python3 -c "
import yaml
m = yaml.safe_load(open('$MANIFEST'))
assert 'also_allow' in m['presets']['soft'], 'soft preset missing also_allow'
assert isinstance(m['presets']['soft']['also_allow'], list)
"
echo ""

# --- NEVER-enable enforcement ---
echo "  NEVER-enable enforcement:"
check_fail "gateway cannot be enabled" \
    python3 "$CORE" --manifest "$MANIFEST" --preset split --enable gateway --output config

check_fail "nodes cannot be enabled" \
    python3 "$CORE" --manifest "$MANIFEST" --preset split --enable nodes --output config

check_fail "bash cannot be enabled" \
    python3 "$CORE" --manifest "$MANIFEST" --preset split --enable bash --output config

check_fail "Unknown tool rejected" \
    python3 "$CORE" --manifest "$MANIFEST" --preset split --enable nonexistent_tool --output config
echo ""

# --- Invariant enforcement ---
echo "  Invariant enforcement across presets:"
for preset in hard split; do
    CFG=$(python3 "$CORE" --manifest "$MANIFEST" --preset "$preset" --output config 2>&1)

    check "$preset: elevated always false" python3 -c "
import json
c = json.loads('''$CFG''')
assert c['tools']['elevated']['enabled'] == False
"

    check "$preset: sandbox always off" python3 -c "
import json
c = json.loads('''$CFG''')
assert c['agents']['defaults']['sandbox']['mode'] == 'off'
"

    check "$preset: gateway.mode always local" python3 -c "
import json
c = json.loads('''$CFG''')
assert c['gateway']['mode'] == 'local'
"

    check "$preset: workspaceOnly always true" python3 -c "
import json
c = json.loads('''$CFG''')
assert c['tools']['fs']['workspaceOnly'] == True
"

    check "$preset: whatsapp disabled" python3 -c "
import json
c = json.loads('''$CFG''')
assert c['channels']['whatsapp']['enabled'] == False
"

    check "$preset: no askFallback (Zod rejects)" python3 -c "
import json
c = json.loads('''$CFG''')
assert 'askFallback' not in c['tools']['exec'], 'askFallback must not be present'
"
done
echo ""

# --- Idempotency ---
echo "  Idempotency:"
SPLIT_A=$(python3 "$CORE" --manifest "$MANIFEST" --preset split --output config 2>&1)
SPLIT_B=$(python3 "$CORE" --manifest "$MANIFEST" --preset split --output config 2>&1)
check "Split generated twice is identical" python3 -c "
a = '''$SPLIT_A'''
b = '''$SPLIT_B'''
assert a == b, 'configs differ'
"
echo ""

# --- Risk score ---
echo "  Risk scores:"
check "Hard risk score = 0.0" python3 -c "
import json
r = json.loads('''$(python3 "$CORE" --manifest "$MANIFEST" --preset hard --output risk 2>&1)''')
assert r['risk_score'] == 0.0, f'got {r[\"risk_score\"]}'
"

check "Split risk score in 0.1-0.3 range" python3 -c "
import json
r = json.loads('''$(python3 "$CORE" --manifest "$MANIFEST" --preset split --output risk 2>&1)''')
assert 0.1 <= r['risk_score'] <= 0.3, f'got {r[\"risk_score\"]}'
"
echo ""

# --- Allowlist ---
echo "  Proxy allowlist:"
HARD_AL=$(python3 "$CORE" --manifest "$MANIFEST" --preset hard --output allowlist 2>&1)
SPLIT_AL=$(python3 "$CORE" --manifest "$MANIFEST" --preset split --output allowlist 2>&1)

check "Hard: 3 base domains" python3 -c "
lines = [l for l in '''$HARD_AL'''.strip().split('\n') if l and not l.startswith('#')]
assert len(lines) == 3, f'got {len(lines)}: {lines}'
"

check "Split: 3 base domains" python3 -c "
lines = [l for l in '''$SPLIT_AL'''.strip().split('\n') if l and not l.startswith('#')]
assert len(lines) == 3, f'got {len(lines)}: {lines}'
"
echo ""

# --- NEVER-safebins in manifest ---
echo "  Manifest integrity:"
check "rm in never_safebins" python3 -c "
import yaml
m = yaml.safe_load(open('$MANIFEST'))
assert 'rm' in m['never_safebins'], 'rm must be in never_safebins'
"

check "Interpreters in never_safebins" python3 -c "
import yaml
m = yaml.safe_load(open('$MANIFEST'))
for interp in ['sh', 'bash', 'node', 'python', 'python3']:
    assert interp in m['never_safebins'], f'{interp} missing from never_safebins'
"

check "gateway in never_enable" python3 -c "
import yaml
m = yaml.safe_load(open('$MANIFEST'))
assert 'gateway' in m['never_enable']
"

check "nodes in never_enable" python3 -c "
import yaml
m = yaml.safe_load(open('$MANIFEST'))
assert 'nodes' in m['never_enable']
"

check "bash in never_enable" python3 -c "
import yaml
m = yaml.safe_load(open('$MANIFEST'))
assert 'bash' in m['never_enable']
"

check "bash in deny list (split preset)" python3 -c "
import json
c = json.loads('''$(python3 "$CORE" --manifest "$MANIFEST" --preset split --output config 2>&1)''')
assert 'bash' in c['tools']['deny'], 'bash not in deny list'
"
echo ""

# --- Results ---
echo "==========================="
echo "Results: $PASS passed, $FAIL failed"
echo ""

if [ $FAIL -gt 0 ]; then
    echo "[!] TOOL CONTROL TESTS FAILED"
    exit 1
else
    echo "[+] ALL TOOL CONTROL TESTS PASSED"
fi
