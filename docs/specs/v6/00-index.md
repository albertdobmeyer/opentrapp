# OpenTrApp v6 — Specification Index

> **Status:** Spec, ready for implementation. Authored 2026-05-31.
> **For:** the agent(s) implementing v6. Read this file first.
> **Scope:** all three concerns, one shared judgment layer. No code was
> written while authoring these specs — they describe what to build.

---

## 1. What v6 is, in one sentence

OpenTrApp v6 makes the project's "**uses AI to make AI safe**" claim literally
true by adding **Sentinel** — a tiny local AI that quietly watches another
AI's behaviour in real time, judges the gray zone the static defences miss,
explains its decisions in plain language, and escalates to a powerful model
only when the user deliberately asks it to.

## 2. Why (the gap v6 closes)

A 2026-05-31 ground-truth audit found the USP is ~90% aspirational today:

| Concern | AI today | Reality |
|---------|----------|---------|
| Containerisation | none | shell level = user toggle; allowlist = 4 hardcoded domains; egress = static nftables; orchestrator = deterministic |
| Forge (skills) | one slice | 87-pattern scanner is pure regex; a local model (`qwen2.5-coder:1.5b`) is used only in one CDR stage |
| Social | none | 25 static regex patterns, parked, Moltbook-coupled |

The static layers are good (fast, cheap, auditable). What's missing is
**judgment** — catching the paraphrased/novel attack, adapting without a human
editing a regex, and *explaining* a decision. v6 adds exactly that, without
bloat.

## 3. The spine: Sentinel's escalation ladder

One shared judgment layer all three concerns consult. Each rung is rarer,
slower, costlier than the one below. Cheap rungs handle ~95%.

| Rung | What | When | Cost | Memory |
|-----:|------|------|------|--------|
| **0 — Static** | regex / allowlist / nftables (today's defences) | always, instant | free | ~0 |
| **1 — Embeddings** | similarity / anomaly / drift | constantly, background | ~free | ~100 MB always-on |
| **2 — Tiny LLM** ⭐ | semantic judgment on flagged cases | only when 0/1 flag ambiguity | cheap, local | sub-1B, load-on-demand |
| **3 — Big judge** | confirmed edge case rung 2 can't crack | **user-triggered, never auto** | slow or paid | heavy |

Full mechanical detail: [`01-sentinel-spine.md`](01-sentinel-spine.md).

**Three load-bearing decisions (already made, do not relitigate):**
- **Rung 3 is human-first.** It never auto-fires. On a confirmed edge case the
  user picks: (a) bigger *local* model [pause the agent, stay private], (b)
  *cloud* verdict on just the flagged fragment [cents, reuses the agent's
  existing key], or (c) decide themselves.
- **The tiny budget is configurable.** Ship a sub-1B default that runs on a
  7–8 GB laptop *alongside* the user's real agent; let power users upgrade.
- **Activity is always visible.** The user must never wonder why their machine
  got busy. A Sentinel indicator shows the active rung.

## 4. The three concerns in v6

| Leg | Tagline | Spec |
|-----|---------|------|
| Adaptive Containment | "least-privilege, discovered not configured" | [`02-adaptive-containment.md`](02-adaptive-containment.md) |
| The Cleanroom (forge) | "anything that can't survive being described is gone" | [`03-cleanroom-forge.md`](03-cleanroom-forge.md) |
| The Semantic Firewall (social) | "read the agent-web without becoming a vector" | [`04-semantic-firewall-social.md`](04-semantic-firewall-social.md) |

## 5. Build sequencing

Build the **spine once** (during the forge leg, which already has the local
model and the ZONE-4a bug the spine fixes), then wire the other legs to it:

1. **[`03`] Cleanroom (forge)** — proves the static→tiny→human ladder
   end-to-end; ships the Sentinel service + disarm diff + activity indicator.
2. **[`02`] Adaptive Containment** — wires the persistent egress log to
   Sentinel; adds the propose-tightening loop.
3. **[`04`] Semantic Firewall (social)** — generalises the Moltbook adapter;
   adds persona-drift + semantic injection judgment.

Each leg is its own spec → plan → build unit. The spine is consumed by all
three.

## 6. The anti-bloat contract (non-negotiable)

These constraints keep v6 lean. Any implementation that violates one needs a
new decision, not a workaround:

1. **One shared judgment layer, not three.** Build Sentinel once.
2. **Static-first, always.** Rungs 1–3 only run on what rung 0 can't resolve.
3. **Tiny default, load-on-demand.** Rung 2 unloads when idle. Rung 1
   embeddings are the only always-resident AI (~100 MB).
4. **Rung 3 never auto-fires.** No surprise slowdown, no surprise cost.
5. **No new dependency tier.** Ollama is already in the stack; embeddings are
   a small addition; the rung-3 cloud call reuses the agent's key + proxy.
6. **Coexists with the user's real agent.** Sentinel runs *next to*
   OpenClaw/opencode, never competing for the RAM the user needs.

## 7. Glossary

- **Sentinel** — the shared judgment layer (working name; §Open decisions).
- **Rung** — one tier of the escalation ladder (0 static → 3 big judge).
- **Verdict** — Sentinel's structured output for a case: allow / block /
  escalate, plus a plain-language reason.
- **Disarm diff** — the human-readable before/after a skill goes through CDR.
- **Persona drift** — divergence between an agent's outgoing content and its
  established task/voice.
- **The silo** — OpenTrApp's containment promise: *untrusted content never
  reaches the host unfiltered; credentials never leak.* (NOT "no cloud LLM" —
  the agent already calls one.)

## 8. Open decisions (resolve during implementation, flagged per-leg)

| # | Decision | Owner | Where |
|---|----------|-------|-------|
| D1 | Public name for "Sentinel" / the tiny-AI USP | maintainer | affects pitch + UI |
| D2 | Embedding model (flavour, size, license) for rung 1 | implementer | [`01`] §model layer |
| D3 | Rung-2 default model (qwen2.5-coder:0.5b vs alternative) | implementer | [`01`] §model layer |
| D4 | Sentinel runtime location (host service vs container) | implementer | [`01`] §architecture |
| D5 | "Confirmed edge case" threshold (avoid alert fatigue) | implementer | [`01`] §escalation |
| D6 | Whether v6 maps to v0.6 or a later tag | maintainer | release |

## 9. Relationship to existing docs

- Architecture this builds on: [`docs/perimeter-explained.md`](../../perimeter-explained.md), [`docs/trifecta.md`](../../trifecta.md)
- Invariant Sentinel must honour: [ADR-0002](../../adr/0002-adaptive-shell-levels.md) (agent cannot self-promote privilege)
- Credential model rung-3-cloud reuses: [ADR-0001](../../adr/0001-proxy-side-api-key-injection.md)
- The CDR origin the Cleanroom leg extends: [ADR-0003](../../adr/0003-content-disarm-reconstruction.md)
- A new ADR should record the Sentinel decision once the spine lands
  (suggested: ADR-0014 — local-AI judgment layer).
