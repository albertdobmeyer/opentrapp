# Spec: Engagement Level Presets

**Date:** 2026-04-07
**Phase:** Pioneer Phase 6
**Gap:** Handoff Gap 5 — Three engagement levels documented but not enforced
**Pattern:** Mirrors vault's tool-control.sh preset system

---

## Problem

The safe-participation-guide documents three engagement levels (Observer, Researcher, Participant) with different rate limits, scanning policies, and automation rules. But nothing enforces these — the user manually edits `.env` and hopes they set the right values. A misconfig (e.g., Level 1 user accidentally setting high rate limits) undermines the safety model.

## Solution

Create a preset system that configures pioneer for a specific engagement level with one command, validates the config matches the level, and exposes level-switching as GUI buttons via `component.yml`.

---

## Presets

### Observer (Level 1)

Read-only. No API key. No interaction capability.

```env
# Engagement Level: observer (Level 1 — read-only)
MOLTBOOK_API_BASE=https://api.moltbook.com
MOLTBOOK_API_KEY=
AGENT_HANDLE=
RATE_LIMIT_POSTS_PER_HOUR=0
RATE_LIMIT_COMMENTS_PER_HOUR=0
RATE_LIMIT_VOTES_PER_HOUR=0
FEED_SCAN_ENABLED=false
ENGAGEMENT_LEVEL=observer
DATA_DIR=./data
INJECTION_PATTERNS=./config/injection-patterns.yml
FEED_ALLOWLIST=./config/feed-allowlist.yml
```

**Rationale:**
- No API key → can only use unauthenticated endpoints (read-only)
- All rate limits = 0 → no posting even if posting tools are added later
- `FEED_SCAN_ENABLED=false` → not needed for pure observation (no content processing pipeline)
- `ENGAGEMENT_LEVEL=observer` → new field for level detection

### Researcher (Level 2)

Registered identity. Controlled interaction. Mandatory feed scanning.

```env
# Engagement Level: researcher (Level 2 — controlled interaction)
MOLTBOOK_API_BASE=https://api.moltbook.com
MOLTBOOK_API_KEY=
AGENT_HANDLE=
RATE_LIMIT_POSTS_PER_HOUR=5
RATE_LIMIT_COMMENTS_PER_HOUR=10
RATE_LIMIT_VOTES_PER_HOUR=20
FEED_SCAN_ENABLED=true
ENGAGEMENT_LEVEL=researcher
DATA_DIR=./data
INJECTION_PATTERNS=./config/injection-patterns.yml
FEED_ALLOWLIST=./config/feed-allowlist.yml
```

**Rationale:**
- API key and handle left blank → user fills in their own
- Conservative rate limits matching the safe-participation-guide
- `FEED_SCAN_ENABLED=true` → all incoming content must be scanned before processing
- Researcher sees and acts on scan results

### Participant (Level 3)

Full interaction with guardrails. Higher rate limits. Managed allowlist.

```env
# Engagement Level: participant (Level 3 — full interaction)
MOLTBOOK_API_BASE=https://api.moltbook.com
MOLTBOOK_API_KEY=
AGENT_HANDLE=
RATE_LIMIT_POSTS_PER_HOUR=10
RATE_LIMIT_COMMENTS_PER_HOUR=25
RATE_LIMIT_VOTES_PER_HOUR=50
FEED_SCAN_ENABLED=true
ENGAGEMENT_LEVEL=participant
DATA_DIR=./data
INJECTION_PATTERNS=./config/injection-patterns.yml
FEED_ALLOWLIST=./config/feed-allowlist.yml
```

**Rationale:**
- Higher rate limits — still conservative by platform standards
- Feed scanning mandatory (same as researcher)
- User should have a retraction plan documented before switching to this level

---

## New Config Field: ENGAGEMENT_LEVEL

Add `ENGAGEMENT_LEVEL` to `.env`. Values: `observer`, `researcher`, `participant`.

- Used by `verify.sh` to detect current level and apply level-specific checks
- Used by `engagement-control.sh` to confirm current state
- Used by `identity-checklist.sh` to show level-appropriate advice
- Default if not set: treat as `observer` (safest assumption)

---

## Scripts

### `scripts/engagement-control.sh`

The preset switcher. Mirrors vault's `tool-control.sh` UX.

**Modes:**
- `--level observer|researcher|participant` — select a level
- `--dry-run` — show what the config would look like
- `--apply` — write the preset to `config/.env`, preserving user values (API key, handle)
- `--status` — show current engagement level and config summary

**Apply behavior:**
1. Read current `config/.env` to capture user-specific values (API_KEY, AGENT_HANDLE)
2. Write preset values to `config/.env`
3. Re-inject user-specific values (don't overwrite their API key)
4. Run `verify.sh` to validate the result

**Dry-run behavior:**
1. Show the preset config that would be written
2. Show what's different from current config
3. Show level-specific guidelines (from safe-participation-guide)

**Status behavior:**
1. Detect `ENGAGEMENT_LEVEL` from current `.env`
2. Show rate limits, scan status, key presence
3. Flag mismatches (e.g., level says "observer" but API key is set)

### No container involvement

Unlike vault's tool-control.sh which manages container config, pioneer presets are purely host-side `.env` management. No containers, no restart, no rebuild.

---

## Makefile Targets

```makefile
observer:     ## Switch to Level 1 (read-only, no API key)
researcher:   ## Switch to Level 2 (registered, controlled interaction)
participant:  ## Switch to Level 3 (full interaction with guardrails)
level-status: ## Show current engagement level and config
```

---

## component.yml Commands

Add to the `lifecycle` group:

```yaml
- id: set-observer
  name: Observer Mode
  description: "Level 1: Read-only observation. No API key needed, no interaction."
  group: lifecycle
  type: action
  danger: safe
  command: "bash scripts/engagement-control.sh --level observer --apply"
  output:
    format: ansi
    display: checklist
  available_when:
    - ready
  sort_order: 20

- id: set-researcher
  name: Researcher Mode
  description: "Level 2: Registered identity with controlled interaction. Feed scanning required."
  group: lifecycle
  type: action
  danger: caution
  command: "bash scripts/engagement-control.sh --level researcher --apply"
  output:
    format: ansi
    display: checklist
  available_when:
    - ready
  sort_order: 30

- id: set-participant
  name: Participant Mode
  description: "Level 3: Full interaction with rate limits. Requires retraction plan."
  group: lifecycle
  type: action
  danger: caution
  command: "bash scripts/engagement-control.sh --level participant --apply"
  output:
    format: ansi
    display: checklist
  available_when:
    - ready
  sort_order: 40

- id: level-status
  name: Engagement Status
  description: Show current engagement level and configuration summary
  group: monitoring
  type: query
  danger: safe
  command: "bash scripts/engagement-control.sh --status"
  output:
    format: ansi
    display: report
  available_when:
    - ready
    - not_setup
  sort_order: 20
```

---

## verify.sh Updates

Add level-specific checks after existing health checks:

```
8. Engagement Level
  - ENGAGEMENT_LEVEL is set and valid (observer|researcher|participant)
  - Observer: API key should be empty, all rate limits = 0
  - Researcher: Feed scanning enabled, rate limits within bounds (1-20 posts/hr)
  - Participant: Feed scanning enabled, rate limits within bounds (1-50 posts/hr)
  - Mismatch warnings (e.g., observer with API key set)
```

---

## Rate Limit Enforcement

**Current state:** Rate limits are config values only — the tools don't enforce them because there are no posting/commenting tools yet. The scanner and census are read-only operations.

**What the presets do now:** Set the correct defaults so that when posting tools are added, the rate limits are already in place. The identity checklist already validates rate limit ranges.

**Future work:** When posting tools are added, they MUST read `RATE_LIMIT_*` from `.env` and enforce them. This is documented as a contract — presets set the values, tools enforce them.

---

## Security Implications

**Low risk.** This feature:
- Only modifies host-side `.env` files (no containers, no secrets in motion)
- Preserves existing API keys during preset switching (doesn't overwrite)
- Defaults to safest level (observer) if `ENGAGEMENT_LEVEL` is missing
- Does not add new capabilities — only configures existing ones

**The dangerous transition:** Going from observer → researcher requires providing an API key. Going from researcher → participant requires higher rate limits. Both transitions are intentional and require explicit `--apply`.

---

## Test Plan

1. **Preset files valid:** All three presets source correctly in bash
2. **Dry-run output:** Each level shows correct config diff
3. **Apply preserves user values:** Set API key, apply different level, key survives
4. **Round-trip:** observer → researcher → participant → observer, verify each state
5. **Status detection:** Correctly identifies current level from `.env`
6. **Verify adapts:** Level-specific checks pass for each level
7. **Identity checklist adapts:** Shows level-appropriate advice
8. **Default safety:** Missing ENGAGEMENT_LEVEL treated as observer
9. **Mismatch detection:** Observer with API key set → warning

---

## Files to Create/Modify

| Action | File |
|--------|------|
| Create | `config/observer.env` |
| Create | `config/researcher.env` |
| Create | `config/participant.env` |
| Create | `scripts/engagement-control.sh` |
| Modify | `Makefile` — add 4 targets |
| Modify | `component.yml` — add 4 commands |
| Modify | `scripts/verify.sh` (will be rewritten from Makefile inline) |
| Modify | `config/.env.example` — add ENGAGEMENT_LEVEL field |
| Create | `tests/tools/test-engagement-control.sh` |

---

*This spec covers Pioneer Phase 6. Implementation should follow the test plan for verification.*
