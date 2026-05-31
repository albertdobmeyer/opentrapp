# Spec: Security Certificate System (Phase 2)

**Date:** 2026-04-02
**Phase:** 2 (per docs/roadmap.md)
**Depends on:** Phase 1 (Housekeeping) — completed
**Blocks:** Phase 3 (CDR) — CDR output includes a certificate

## Purpose

Build the bridge between forge and vault. When a skill passes the full security pipeline, the forge produces a machine-readable clearance report (certificate) that the vault's `install-skill.sh` can validate before accepting the skill. Without this, skill installation requires manual review or blind trust.

## Design Decisions (Resolved)

| Decision | Resolution | Rationale |
|----------|-----------|-----------|
| GPG signing | Deferred (note in TODO.md) | SHA-256 proves integrity; GPG adds complexity for identity proof that isn't needed yet |
| Export format | Directory (`exports/<name>/`) | Simple, inspectable, directly feeds vault's install-skill.sh |
| Test gate | Required (4-gate: lint+scan+verify+test) | Matches publish pipeline rigor; strongest trust guarantee |
| Skill versioning | Required in frontmatter (`version: 1.0.0`) | Self-contained; certifier refuses without it |

## Certificate Format (clearance-report.json)

```json
{
  "forge_version": "1.0.0",
  "skill": "api-dev",
  "version": "1.0.0",
  "certified_at": "2026-04-02T18:00:00Z",
  "scan": {
    "status": "PASS",
    "critical": 0,
    "high": 0,
    "medium": 0,
    "pattern_count": 87,
    "pattern_version": "2026.04"
  },
  "verify": {
    "verdict": "VERIFIED",
    "total_lines": 509,
    "safe_lines": 509,
    "suspicious_lines": 0,
    "malicious_lines": 0
  },
  "test": {
    "status": "PASS",
    "assertions": 7,
    "failures": 0
  },
  "checksum": "sha256:101f9a313cff52ef047071d66b75b3b041efdcd57837634ebc8fb16b24b9fd93"
}
```

### Vault Compatibility

The vault's `install-skill.sh` (already implemented at `opencli-container/scripts/install-skill.sh:156-213`) validates:
- `scan.status == "PASS"`
- `scan.critical == 0`
- `verify.verdict == "VERIFIED"`
- `checksum` starts with `sha256:` — computes SHA-256 of SKILL.md and compares

All four fields are present in our format. The additional fields (`forge_version`, `test`, `pattern_version`, line counts) are ignored by the vault but provide audit trail for humans and future tooling.

## New Files

### tools/skill-certify.sh

Runs the full 4-gate pipeline on a single skill and generates `clearance-report.json`.

**Usage:**
```bash
bash tools/skill-certify.sh <skill-name>
# Output: skills/<skill-name>/clearance-report.json
```

**Pipeline:**
1. **Gate 1 — Lint.** Run `skill-lint.sh` on the skill. Block on failure.
2. **Gate 2 — Scan.** Run `skill-scan.sh --json` on the skill. Capture JSON output. Block if `blocked > 0`.
3. **Gate 3 — Verify.** Run `skill-verify.sh --json --trust` on the skill. Capture JSON output. Block if verdict is not `VERIFIED`.
4. **Gate 4 — Test.** Run `skill-test.sh` on the skill. Capture pass/fail counts. Block on any failure.
5. **Generate certificate.** Extract scan summary and verify data from captured JSON. Compute SHA-256 of SKILL.md. Extract `version` from frontmatter. Write `clearance-report.json`.

**Frontmatter version extraction:** Uses the existing `frontmatter.sh` library (or python3 YAML parsing) to read `version` from SKILL.md. Refuses to certify if `version` is missing or not in semver format (`X.Y.Z`).

**JSON generation:** Uses `json_escape` from `common.sh` (moved there as part of this spec — see "Pre-existing bug fix" below).

### tools/skill-export.sh

Packages a certified skill for vault consumption.

**Usage:**
```bash
bash tools/skill-export.sh <skill-name>
# Output: exports/<skill-name>/SKILL.md + clearance-report.json + .trust
```

**Steps:**
1. Check if `skills/<name>/clearance-report.json` exists and is fresh (checksum matches current SKILL.md). If stale or missing, run `skill-certify.sh` first.
2. Create `exports/<name>/` directory.
3. Copy `skills/<name>/SKILL.md`, `skills/<name>/clearance-report.json`, and `skills/<name>/.trust` into the export directory.
4. Print the export path and instructions for vault installation.

**Staleness check:** Compare `checksum` field in clearance-report.json against current SHA-256 of SKILL.md. If mismatch, re-certify.

## Modified Files

### tools/lib/common.sh — Add json_escape

Move `json_escape()` from `skill-scan.sh` to `common.sh` so both the scanner, verifier, and certifier can use it. Remove the duplicate from `skill-scan.sh` and have it use the shared version.

This also fixes a pre-existing bug: the verifier's `--json` mode calls `json_escape` which doesn't exist in its scope (only defined in `skill-scan.sh`).

### tools/skill-scan.sh — Remove json_escape

Remove the local `json_escape()` definition (now in `common.sh`).

### All 25 skills — Add version to frontmatter

Add `version: 1.0.0` to the YAML frontmatter of all 25 SKILL.md files. Example:

```yaml
---
name: api-dev
version: 1.0.0
description: Build, test, and debug REST/GraphQL APIs...
metadata: {...}
---
```

### tools/skill-lint.sh — Add version check

Add a linter check that `version` field exists in frontmatter and matches semver format. This is a WARN (not FAIL) for now to avoid breaking the lint gate retroactively — the certifier enforces it as a hard requirement separately.

### Makefile — Add targets

```makefile
certify: ## Generate security certificate (SKILL=name)
	@bash $(TOOLS_DIR)/skill-certify.sh "$(SKILL)"

certify-all: ## Generate certificates for all skills
	@for dir in $(SKILLS_DIR)/*/; do \
		skill=$$(basename "$$dir"); \
		bash $(TOOLS_DIR)/skill-certify.sh "$$skill" || exit 1; \
	done

export: ## Certify + package for vault transfer (SKILL=name)
	@bash $(TOOLS_DIR)/skill-export.sh "$(SKILL)"
```

### component.yml — Add commands

Add `certify` and `export` commands in the operations group for the OpenTrApp GUI.

### .gitignore — Add exports/

The `exports/` directory is generated output, not committed.

### TODO.md — Update

Mark Phase 2 items as done. Add note about deferred GPG signing.

## Directories

### exports/ (generated, gitignored)

```
exports/
└── api-dev/
    ├── SKILL.md
    ├── clearance-report.json
    └── .trust
```

### certificates/ (NOT used)

The design doc mentions a `certificates/` directory. We store certificates inside the skill directory instead (`skills/<name>/clearance-report.json`). This keeps the certificate co-located with its skill and simplifies staleness checks. No separate certificates/ directory needed.

## End-to-End Flow

```
# Certify a single skill
make certify SKILL=api-dev
# → Lint: PASS
# → Scan: PASS (0 findings)
# → Verify: VERIFIED (509/509 safe)
# → Test: PASS (7/7 assertions)
# → Certificate: skills/api-dev/clearance-report.json

# Export for vault
make export SKILL=api-dev
# → Certificate valid (checksum matches)
# → Exported to exports/api-dev/

# Install in vault (from opentrapp root)
cd components/opencli-container
bash scripts/install-skill.sh ../openskill-forge/exports/api-dev/ \
  --clearance ../openskill-forge/exports/api-dev/clearance-report.json
# → Clearance: PASS (scan clean, verified, checksum valid)
# → Skill 'api-dev' installed to workspace/skills/api-dev/
```

## Verification Plan

1. **Unit:** `make certify SKILL=api-dev` produces valid JSON certificate with correct checksums
2. **Unit:** `make export SKILL=api-dev` creates exports/api-dev/ with 3 files
3. **Integration:** Vault's `install-skill.sh` accepts the exported skill + certificate (requires running vault)
4. **Regression:** `make check-all` still passes (168 tests, 10/10 self-test)
5. **Staleness:** Modify a SKILL.md, re-run certify, verify checksum updates
6. **Rejection:** Run certify on a skill that fails scan — verify it blocks and produces no certificate
7. **Version gate:** Remove version from frontmatter, run certify — verify it refuses

## Out of Scope

- GPG signing (deferred — note added to TODO.md for future consideration)
- JSON schema file for clearance report (nice-to-have, Phase 5)
- Tarball/archive export format
- Batch export (`make export-all`)
- Certificate expiration/rotation
