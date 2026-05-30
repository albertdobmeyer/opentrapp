# Spec: Skill Installation Path (Forge → Vault)

**Date:** 2026-03-30
**Phase:** 5a (Cross-Module Integration)
**Security implications:** Skills are executable reference material — a malicious skill can instruct the agent to perform harmful actions. Every skill entering the vault must be vetted.

---

## Purpose

Define and implement how a openskill-forge-vetted skill enters the opencli-container. This is the most important integration gap between the three modules (per trifecta.md).

## Current State

- Skills are `SKILL.md` files with YAML frontmatter (name, description, metadata)
- Forge has: `make scan` (87-pattern security scan, JSON output), `make verify` (zero-trust line classification, JSON output)
- Vault has: no skill installation mechanism
- ClawHub domains are blocked in the proxy allowlist (intentional)
- Manual workaround: user copies a SKILL.md into the vault workspace via `podman cp`

## Skill Installation Policy

| Shell Level | Skill Installation | Rationale |
|---|---|---|
| Hard Shell | NOT ALLOWED | No file operations — conversation only |
| Split Shell | MANUAL ONLY | User reviews skill, copies via host-side tooling with clearance report |
| Soft Shell | MANUAL OR ASSISTED | User-initiated, agent may suggest skills but cannot install autonomously |

**Skills are NEVER auto-installed.** The agent cannot install skills from ClawHub (domains blocked). All skill installation is user-initiated from the host side.

## Manual Skill Transfer Workflow (Available Now)

This workflow works today with existing tools:

```
1. User obtains skill SKILL.md (from ClawHub, custom, or forge output)

2. User scans the skill with forge:
   cd components/openskill-forge
   make scan-one SKILL=<name>           # 87-pattern security scan
   make verify-skill SKILL=<name>       # zero-trust line classification

3. User reviews scan results:
   - CRITICAL findings → DO NOT INSTALL
   - HIGH findings → review manually before proceeding
   - Clean scan + VERIFIED verdict → safe to install

4. User copies skill into vault workspace:
   podman cp <skill-dir>/SKILL.md opencli-container:/home/vault/.openclaw/workspace/skills/<name>/SKILL.md

5. User verifies installation:
   podman exec opencli-container ls -la /home/vault/.openclaw/workspace/skills/

6. Agent can now reference the skill from its workspace
```

## Vault-Side Tooling (To Build)

### scripts/install-skill.sh

A host-side script that:
1. Accepts a path to a skill directory (containing SKILL.md)
2. Optionally accepts a clearance report (forge scan + verify JSON output)
3. Validates the skill format (YAML frontmatter present, SKILL.md exists)
4. If clearance report provided: validates it shows PASS/VERIFIED status
5. If no clearance report: warns the user and requires explicit confirmation
6. Copies SKILL.md into the vault workspace at `skills/<name>/SKILL.md`
7. Reports success and shows the installed skill's metadata

### What it does NOT do
- It does not download skills from ClawHub (domains are blocked)
- It does not run scan or verify (that's forge's job)
- It does not modify the agent's tool policy (skills are reference material, not tools)
- It does not execute anything inside the container

## Clearance Report Format (Coordination with Forge)

The forge side needs to implement `make export SKILL=<name>` (forge roadmap Phase 3) that produces:

```json
{
  "skill": "<name>",
  "version": "1.0.0",
  "exported_at": "2026-03-30T12:00:00Z",
  "scan": {
    "status": "PASS",
    "critical": 0,
    "high": 0,
    "medium": 0,
    "pattern_count": 87
  },
  "verify": {
    "verdict": "VERIFIED",
    "safe_lines_pct": 98.5,
    "suspicious_lines": 0,
    "malicious_lines": 0
  },
  "checksum": "sha256:<hash-of-SKILL.md>"
}
```

The vault's `install-skill.sh` validates this report before accepting the skill. If the checksum doesn't match the actual SKILL.md, installation is rejected.

## Implementation Steps

### Step 1: Document the manual workflow
- Add manual workflow to `docs/openclaw-reference.md` or a new `docs/skill-installation.md`
- No code changes

### Step 2: Build install-skill.sh
- Accept skill path and optional clearance report
- Validate format, warn if no report
- Copy to workspace via podman cp
- Report installation status

### Step 3: Add Makefile target
- `make install-skill SKILL=<path>`

### Step 4: Coordinate clearance report format with forge
- This is forge Phase 3 work — define the JSON format, implement `make export`
- Vault side: update install-skill.sh to validate the report when provided

## Security Implications

- Skills are reference material (markdown), not executable code — but they instruct the LLM
- A malicious skill could instruct the agent to perform harmful actions via social engineering
- The forge's 87-pattern scanner detects known malicious patterns
- The zero-trust verifier quarantines skills with unrecognizable content
- The vault's tool policy still applies — even if a skill says "run rm -rf /", the agent can't if rm is not in safeBins
- Defense-in-depth: forge scans → user reviews → vault tool policy → container exoskeleton

## Verification Plan

1. Manual workflow: copy a skill into the workspace, verify agent can read it
2. install-skill.sh: test with valid skill (accepts), test without clearance (warns), test with bad checksum (rejects)
3. Test that ClawHub domains remain blocked (agent cannot install skills autonomously)
