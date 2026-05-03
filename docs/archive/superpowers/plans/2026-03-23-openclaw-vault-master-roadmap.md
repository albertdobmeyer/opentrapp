# OpenClaw-Vault Master Roadmap

> **For agentic workers:** Each phase below has its own implementation plan. Execute plans in order — each phase depends on the previous one being complete and verified.

**Goal:** Transform openclaw-vault from a static developer sandbox into a proven, multi-gear security harness that any non-technical user can trust.

**Architecture:** Six-layer defense (container + proxy + tool policy + app restrictions + exec controls + hardening config). Three gears (Manual / Semi-Auto / Full-Auto). GUI-driven via Lobster-TrApp manifest system.

**Spec:** `docs/superpowers/specs/2026-03-23-openclaw-vault-security-harness-design.md`

---

## Phase 0: Fix Pre-Existing Bugs

**Plan:** `docs/superpowers/plans/2026-03-23-phase0-bug-fixes.md`

**Why first:** We can't build on a broken foundation. These bugs exist independent of the redesign and must be fixed before any new work.

| Task | Risk if Skipped |
|------|----------------|
| Fix `test-network-isolation.sh` (uses `wget` stripped from image) | Security test suite gives false confidence |
| Fix proxy container name in `component.yml` (`openclaw-proxy` -> `vault-proxy`) | GUI proxy-logs button silently fails |
| Make `anthropic-version` header configurable in vault-proxy.py | Proxy breaks when Anthropic updates API version |
| Update stale docs (TODO.md, vision-and-status.md) | Contributors confused by wrong information |

**Exit criteria:** All 12 existing tests pass. All 15 verify.sh checks pass. component.yml references correct container names.

---

## Phase 1: Verify OpenClaw Compatibility

**Plan:** Written after Phase 0 is complete.

**Why:** Open Questions 4 and 8 from the spec MUST be answered before we can build gear configs. Everything downstream depends on knowing the actual config format and Node version requirements.

| Task | Open Question Answered |
|------|----------------------|
| Verify OpenClaw `@2026.2.17` works on Node 20 (or determine Node 22+ is required) | OQ4: Node version |
| Determine if OpenClaw `--config` accepts YAML, JSON, or both | OQ8: Config format |
| Map our YAML config keys to OpenClaw's official JSON key paths | OQ8: Config alignment |
| Investigate Gateway WebSocket API for runtime reconfiguration | OQ1: Gear switching |
| Document actual OpenClaw behavior when sandbox mode is set but Docker socket is absent | Spec Layer 4 validation |

**Exit criteria:** We know the exact config format, Node version, and runtime reconfiguration capabilities. A document captures all findings with evidence.

**How:** This phase requires actually running OpenClaw inside the vault container and testing its behavior. This is where we install it on our laptop for the first time.

---

## Phase 2: Formalize Gear 1 (Manual)

**Plan:** Written after Phase 1 findings are incorporated.

**Why:** Gear 1 is 90% of what the current vault already does. Formalizing it means: generating the correct OpenClaw config (in the right format, per Phase 1 findings), writing gear-specific verification tests, and ensuring every Layer 1-6 control is properly configured and tested.

| Task | Purpose |
|------|---------|
| Create Gear 1 OpenClaw config profile (correct format per Phase 1) | Layer 3-6 configuration |
| Create Gear 1 allowlist template (LLM APIs only, no raw.githubusercontent.com) | Layer 2 configuration |
| Write Gear 1-specific verification tests (beyond the 15-point check) | Prove Gear 1 claims |
| Create `scripts/switch-gear.sh manual` command | Gear switching mechanism |
| Update `component.yml` with gear-switching commands | Lobster-TrApp integration |
| Test Gear 1 end-to-end on our laptop | Prove the thesis for Manual mode |

**Exit criteria:** Gear 1 is running on our laptop. Agent cannot exec, cannot read files, cannot reach non-API domains, requires approval for every action. All gear-specific tests pass.

---

## Phase 3: Monitoring Implementation

**Plan:** Written after Phase 2 is verified.

**Why:** Monitoring is required for ALL gears. Before we build Gear 2/3 (which grant more access), we need the ability to see what the agent is doing. Without monitoring, the user has no visibility — which violates the thesis.

| Task | Purpose |
|------|---------|
| Implement `monitoring/network-log-parser.py` (parse proxy JSONL) | Network traffic visibility |
| Implement `monitoring/session-report.sh` (summarize agent actions) | Session audit trail |
| Implement `monitoring/activity-feed.sh` (real-time structured log) | Live monitoring |
| Update `component.yml` monitoring commands | GUI integration |
| Test monitoring output renders correctly in Lobster-TrApp | End-to-end verification |

**Exit criteria:** User can see what the agent did in plain language via the GUI. Network traffic, tool usage, and blocked attempts are all visible.

---

## Phase 4: Gear 2 (Semi-Auto) — Capability System

**Plan:** Written after Phase 3 is verified.

**Why:** This is where the vault goes from "locked-down research tool" to "useful AI assistant." Gear 2 introduces per-capability tool grants, selective host mounts, and messaging channel integration.

| Task | Purpose |
|------|---------|
| Design capability toggle system (config format, switching mechanism) | Core Gear 2 architecture |
| Implement messaging capability (Telegram credentials, channel tools) | First real capability |
| Implement web browsing capability (sandboxed browser, domain config) | Second capability |
| Implement file workspace capability (tmpfs, file transfer in/out) | Third capability |
| Implement file access capability (specific host directory mounts) | Fourth capability |
| Create Gear 2 compose templates (per-capability mount variations) | Layer 1 configuration |
| Create Gear 2 allowlist templates (per-capability domain sets) | Layer 2 configuration |
| Create Gear 2 OpenClaw config profiles | Layers 3-6 configuration |
| Write Gear 2-specific verification tests | Prove Gear 2 claims |
| Implement protected resources checks for Gear 2 | Prove driver seat holds |
| Create `scripts/switch-gear.sh semi` command | Gear switching |
| Solve credential persistence (persistent volume for Telegram/WhatsApp) | OQ3 |
| Test Gear 2 end-to-end on our laptop | Prove the thesis for Semi-Auto |

**Exit criteria:** User can grant specific capabilities via GUI. Agent can use granted tools but cannot access protected resources. Monitoring shows all activity.

---

## Phase 5: Gear 3 (Full-Auto) — Broad Autonomy with Driver Seat

**Plan:** Written after Phase 4 is verified.

**Why:** The most permissive gear and the hardest to prove safe. Must demonstrate that even with broad access, the protected resources (root, SSH keys, passwords, keyrings, etc.) remain inaccessible.

| Task | Purpose |
|------|---------|
| Create Gear 3 compose template (broad host mounts minus protected resources) | Layer 1 |
| Create Gear 3 allowlist (broad, blocking ClawHub + internal networks) | Layer 2 |
| Create Gear 3 OpenClaw config (full profile, deny gateway + sessions_spawn) | Layers 3-6 |
| Implement exfiltration threshold per gear (OQ9) | Layer 2 tuning |
| Implement agent self-modification prevention (OQ10) | Layer 4 hardening |
| Write Gear 3 protected resources test suite (attempt every driver-seat resource) | Prove driver seat holds at maximum autonomy |
| Create `scripts/switch-gear.sh full` command | Gear switching |
| Test Gear 3 end-to-end | Prove the thesis for Full-Auto |

**Exit criteria:** Agent operates with broad autonomy. Protected resources are verified inaccessible. Monitoring captures all activity. User can downshift to Gear 1 at any time.

---

## Phase 6: Documentation & Release Preparation

**Plan:** Written after Phase 5 is verified.

**Why:** The vault's README must match what it actually does. The component.yml must expose all gears. The thesis must be stated clearly for non-technical users.

| Task | Purpose |
|------|---------|
| Rewrite README (new thesis, new target audience, gear system docs) | Public documentation |
| Remove Path B (docker-sandbox-setup.sh) | Cleanup per spec section 6.4 |
| Remove/archive Phase 2 VM isolation stubs | Cleanup |
| Update component.yml with all gear commands, states, and monitoring | Lobster-TrApp integration |
| Capture screenshots for README | Visual documentation |
| Update CLAUDE.md with new architecture | Developer documentation |
| Final security audit of entire repo | Pre-release confidence |
| Make repos public again | Release |

**Exit criteria:** README accurately describes a working, tested, multi-gear security harness. All four repos are ready for public consumption.

---

## Dependency Graph

```
Phase 0 (Bug Fixes)
    |
Phase 1 (Verify OpenClaw Compatibility)
    |
Phase 2 (Gear 1 — Manual)
    |
Phase 3 (Monitoring)
    |
Phase 4 (Gear 2 — Semi-Auto)
    |
Phase 5 (Gear 3 — Full-Auto)
    |
Phase 6 (Documentation & Release)
```

Every phase is strictly sequential. Each phase's plan incorporates learnings from the previous phase. No phase is planned in detail until the previous one is complete — because each phase answers open questions that affect later phases.

---

## Timeline Expectations

No time estimates. Each phase is done when its exit criteria are met. The critical path is: Phase 0 -> Phase 1 -> Phase 2 -> test on our laptop. If Gear 1 doesn't work on our laptop, everything stops until it does.
