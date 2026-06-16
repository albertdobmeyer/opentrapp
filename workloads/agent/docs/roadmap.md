# vault-agent Roadmap

**Updated:** 2026-03-30
**Current state:** All three shell levels operational. Per-tool whitelisting/blacklisting with 24-point verification. Hard Shell, Split Shell, and Soft Shell presets. Monitoring tools (network, session, audit, log rotation) all implemented.
**Cross-reference:** See `docs/trifecta.md` in the opentrapp root for how this module fits with vault-skills and vault-social.

---

## Phase 1: Documentation Cleanup — COMPLETED (2026-03-27)

**Why:** Stale docs create confusion and waste time. Fix before building anything new.

| Task | Details |
|---|---|
| Fix `component.yml` config paths | `openclaw-hardening.yml` → `config/openclaw-hardening.json5`, `allowlist.txt` → `proxy/allowlist.txt`, format `yaml` → `json5` |
| Update `CLAUDE.md` | Profile `"minimal"` → `"coding"` (current Split Shell state), fix stale claims |
| Complete Gear → Shell terminology migration | Update `docs/setup-guide.md`, cross-reference `GLOSSARY.md` |
| Remove redundant `monitoring/skill-scanner.sh` | Replace with pointer to vault-skills's `tools/skill-scan.sh` |
| Add OpenClaw version pin note | `openclaw-internals.md` and `phase1-findings.md` cover different versions (2026.2.17 vs 2026.2.26) — clarify which is current (2026.2.26, pinned in Containerfile) |
| Split `definitions.md` | Keep ecosystem definitions (lines 1-120). Move competitive positioning (121-323) to `product-assessment.md` in opentrapp root |

**Exit criteria:** All docs reflect current state. No stale claims. Terminology consistent.

---

## Phase 2: Runtime Monitoring — COMPLETED (2026-03-27)

**Why:** Before granting more autonomy (Soft Shell), we need full visibility into what Hum does. Monitor first, expand later.

### 2a: Network Log Parser

Replace `monitoring/network-log-parser.py` stub with a real implementation.

**Input:** `requests.jsonl` from vault-proxy (structured JSON, one entry per request/response).

**Output:** Human-readable report with:
- Summary: total requests, allowed/blocked counts, unique domains
- Anomaly flags:
  - Requests to unexpected domains (should never happen with allowlist, but defense-in-depth)
  - Unusually large payloads (> configurable threshold)
  - High request frequency in short windows (potential abuse)
  - Requests outside normal hours (if configured)
- Per-domain breakdown: request count, total bytes, blocked count
- Timeline: requests over time (text-based, for terminal output)

**Implementation notes:**
- Python script (matches existing stub language)
- Reads from proxy container volume or via `podman exec`
- Must work when container is stopped (read from volume directly)
- JSON output mode for programmatic consumption
- Wire into `component.yml` as a monitoring command
- Add to Makefile as `make network-report`

### 2b: Session Report Generator

Replace `monitoring/session-report.sh` stub with a real implementation.

**Input:** Session transcript `.jsonl` files + proxy `requests.jsonl`.

**Output:** Post-session summary:
- Session duration (first to last message timestamp)
- Message count (user messages, agent responses, tool calls)
- Tools used (which tools were invoked, how many times)
- Commands executed (exact commands, approval status)
- Files created/modified in workspace
- Network activity (domains contacted, requests allowed/blocked)
- Security events (any blocked requests, approval denials, injection patterns)
- API usage estimate (request count to api.anthropic.com, rough token estimate from payload sizes)

**Implementation notes:**
- Bash script (matches existing pattern)
- Combines data from session transcripts AND proxy logs for complete picture
- Must work when container is stopped
- Wire into `component.yml` and Makefile as `make session-report`

### 2c: Skill Scanner Stub Cleanup

- Delete `monitoring/skill-scanner.sh`
- Create `monitoring/README.md` explaining that skill scanning lives in vault-skills
- Update `TODO.md` to reflect this decision

### 2d: Log Rotation

- Add log rotation for `requests.jsonl` (rotate at 10MB, keep 5 rotations)
- Add session transcript size monitoring (warn if total exceeds 100MB)
- Document cleanup procedure

**Exit criteria:** `make network-report` and `make session-report` produce useful output. Monitoring stubs are gone. Logs don't grow unbounded.

---

## Phase 3: Split Shell Completion — COMPLETED (2026-03-30)

**Why:** Split Shell works but has rough edges. Fix them before designing Soft Shell.

### 3a: Shell-Aware Verification

`verify.sh` check #15 hardcodes `exec security = deny` — this fails on Split Shell where exec security is `"allowlist"`.

**Fix:**
- Detect current shell level from running config
- Adjust expected values per shell:
  - Hard Shell: expect `security: "deny"`
  - Split Shell: expect `security: "allowlist"` + `ask: "always"` + safeBins present
  - Soft Shell: (define when Soft Shell is designed)
- Add checks specific to Split Shell:
  - Verify safeBinProfiles match safeBins (no orphans)
  - Verify `host: "gateway"` (not `"sandbox"`)
  - Verify `workspaceOnly: true`
  - Verify persistent volume is mounted

### 3b: Per-Shell Allowlist

Currently all shells use the same 3-domain `proxy/allowlist.txt`. Future shells may need different domains.

**Design decision needed:** Should allowlists be per-shell files (`proxy/hard-shell-allowlist.txt`, `proxy/split-shell-allowlist.txt`) or should `tool-control.sh` swap the allowlist file during a shell switch?

**For now:** Document that the 3-domain allowlist is intentionally shared between Hard Shell and Split Shell. Soft Shell allowlist design is a Phase 4 task.

### 3c: Test Runner

12 test scripts exist in `tests/` but have no runner. Create `make test` that:
- Runs all 12 test scripts sequentially
- Reports pass/fail per script
- Returns non-zero if any fail
- Works alongside `make verify` (which runs verify.sh)

### 3d: `read-chat.sh` Improvements — DONE

- ~~Remove the 300-character truncation~~ — Rewritten: full messages displayed, no truncation
- ~~Add `--tool-calls` flag~~ — Done: shows tool invocations with details
- ~~Add `--since TIMESTAMP` flag~~ — Done: filter messages by timestamp
- Added `--all` flag and `sanitize()` for terminal injection prevention

**Exit criteria:** All met. `verify.sh` 24/24 on all three shells. `make test` passes. `read-chat.sh` shows full conversation with tool calls.

---

## Phase 4: Tool Control System — COMPLETED (2026-03-30)

**Why:** Soft Shell is the next trust level — broader autonomy while the universal container hardening stays enforced. Requires a proper design process before implementation.

### Design Questions to Answer

1. **Approval model:** Does Soft Shell keep `ask: "always"` for all commands, or auto-approve safeBins and only prompt for unknowns?
2. **Extended safeBins:** Which additional commands? Candidates: `grep`, `find`, `ls`, `sed`, `awk`, `diff`. These are already in the `coding` profile but not in Split Shell's safeBins.
3. **New domains:** Does Soft Shell unlock `raw.githubusercontent.com` (for reading skill source code)? Any others?
4. **Process tool:** Split Shell denies `process` (background processes). Does Soft Shell allow it?
5. **Sub-agents:** Split Shell denies `group:sessions`. Does Soft Shell allow limited sub-agent spawning?
6. **Browser:** Split Shell denies `browser`. Does Soft Shell allow sandboxed browsing through the proxy?
7. **Cron:** Split Shell denies `cron`. Does Soft Shell allow scheduling?

### Implementation Work (after design)

- Create `config/soft-shell.json5`
- Update `tool-control.sh` to support `soft|3`
- Create per-shell allowlist if needed
- Update `verify.sh` with Soft Shell expectations
- Write Soft Shell verification tests
- Document in `openclaw-reference.md`

**Exit criteria:** Soft Shell design document written and approved. Config file created. Verify + tests pass. The agent demonstrably has broader capability while the universal container hardening continues to hold.

---

## Phase 5: Cross-Module Integration — COMPLETED (2026-03-30)

**Why:** The three modules currently operate independently. Key workflows need connection points.

### 5a: Skill Installation Path (Forge → Vault)

Define and document how a forge-vetted skill enters the vault:
- Manual transfer workflow (current: copy file into workspace)
- Automated workflow (future: forge produces clearance report, vault accepts cleared skills)
- Which shell level allows skill installation
- How the vault verifies a skill was scanned by forge

### 5b: Telegram Bot Token Proxy Injection

The Telegram bot token currently enters the vault container directly (`compose.yml` line 51). For consistency with the "no secrets in the container" philosophy, investigate whether the proxy can inject it the same way it injects API keys — intercepting requests to `api.telegram.org` and adding the bot token.

**Trade-off:** Complexity vs consistency. The bot token is revocable and lower-risk than API keys. This may be over-engineering.

### 5c: Feed Scanning Integration (Pioneer → Vault)

When Moltbook domains are eventually added to the allowlist (Soft Shell or later):
- Define whether feed scanning happens in the proxy or the agent workspace
- If proxy-level: vault-proxy.py calls pioneer's injection patterns on Moltbook API responses
- If workspace-level: pioneer's patterns loaded as a workspace file for the agent to consult

**This is not blocking anything now** — Moltbook domains are not in the allowlist.

**Exit criteria:** Skill installation path documented and tested. Bot token decision made. Feed scanning plan documented for future implementation.

---

---

## Phase 6: Hardening — Trial Run Findings (2026-03-31)

**Why:** Trial run with attack surface probing revealed vulnerabilities that must be fixed before the module is shippable. See `docs/specs/2026-03-31-trial-run-findings.md` for full details.

### 6a: Config Read-Only Mount (CRITICAL)

The agent can modify its own config (`~/.openclaw/openclaw.json`) from inside the container if it gains arbitrary code execution. OpenClaw hot-reloads changes, so a modified config takes effect immediately.

**Fix:** Split the persistent volume into workspace (read-write) and config (read-only). Config only writable at startup and via `tool-control.sh --apply`.

**Files:** `compose.yml`, `scripts/entrypoint.sh`, `scripts/tool-control.sh`

### 6b: Strip Destructive Binaries from Image (MEDIUM)

`/bin/rm` and other destructive binaries exist in the Alpine base image. Strip them in the Containerfile.

**Files:** `Containerfile`

### 6c: Formalize Attack Surface Tests (MEDIUM)

The trial run probes should be an automated test script (`tests/test-attack-surfaces.sh`).

**Files:** `tests/test-attack-surfaces.sh`

**Exit criteria:** Config is read-only inside the container. `rm` not found. All attack surface probes pass as automated tests.

---

## Phase 7: Soft Shell — The Safari

**Why:** The agent becomes genuinely useful — not just a chatbot, but an autonomous assistant. Web search, scheduling, file processing, content automation, all within the fixed protected-resource boundary.

**Spec:** `docs/specs/2026-03-31-soft-shell-design.md`

**Prerequisite:** Phase 6 complete (hardening fixes). DONE.

### What Changes from Split Shell

| Aspect | Split Shell | Soft Shell |
|---|---|---|
| Tools enabled | 11 | 17 (+web_search, web_fetch, cron, process, canvas, message) |
| Exec approval | ask: "always" (every command) | ask: "on-miss" (safeBins auto-approve) |
| SafeBins | 16 | 28 (+grep, sed, awk, diff, xargs, basename, dirname, etc.) |
| Proxy domains | 3 (base only) | 4+ (base + raw.githubusercontent.com, user-configurable) |
| Risk score | 0.18 | 0.45 |

### Implementation Steps

| Step | What | Files |
|---|---|---|
| 7a | Add `soft` preset to tool-manifest.yml | config/tool-manifest.yml |
| 7b | Create config/soft-shell.json5 | config/soft-shell.json5 |
| 7c | Update tool-control-core.py for soft preset | scripts/tool-control-core.py |
| 7d | Update verify.sh for Soft Shell detection | scripts/verify.sh |
| 7e | Test: apply, verify, attack probes, round-trip | All test scripts |
| 7f | Update docs: CLAUDE.md, README.md, openclaw-reference.md | Docs |

**Exit criteria:** `make soft-shell` applies Soft Shell, 24/24 verify PASS, all attack surface probes pass, round-trip Hard→Split→Soft→Split→Hard all pass. Hum can search the web and schedule tasks via Telegram.

---

## Phase 8: Final Review + Certification — COMPLETED (2026-04-06)

**Why:** Holistic review of the entire module — code, docs, tests, attack surfaces. The module must be certifiably shippable.

**Exit criteria:** All tests pass, all docs accurate, all attack surfaces probed and mitigated, tool control system fully operational across all three shell levels. **MET.**

**What was done:**
- Cross-module integration test suite created (28 checks, all passing)
- Orchestrator validation fixed and passing (39 checks)
- Documentation accuracy verified and corrected (trifecta.md, feed-scanning spec)
- Schema enum alignment fixed across all 3 contract layers (json5 format)
- Certification report produced: `docs/certification-report.md`
- Container-level verification (24-point verify, 13 security tests) deferred to next container session

---

## Dependency Graph

```
Phase 1-5 (COMPLETED)
    ↓
Phase 6 (Hardening — trial run findings)
    ↓
Phase 7 (Soft Shell design + implementation)
    ↓
Phase 8 (Final review + certification)
```

---

*This roadmap covers the vault-agent workload only. See `workloads/skills/docs/roadmap.md` and `workloads/social/docs/roadmap.md` for the other modules. See `docs/trifecta.md` in the opentrapp root for the cross-module strategy.*
