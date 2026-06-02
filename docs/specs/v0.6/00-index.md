# OpenTrApp v0.6 — Specification Index

> **Status:** ✅ **v0.6.0 RELEASED** (2026-06-02) — M0–M4 + rung-1 + GUI Sentinel
> surfaces + Items B/A/C/D shipped; published GitHub release (`/releases/latest`). See
> [`08-completion-plan.md`](08-completion-plan.md) + [`docs/handoff.md`](../../handoff.md).
> Remaining = operator queue (demo gifs, OpenSSF badge, Zone 6b). Spec authored 2026-05-31.
> **For:** the agent(s) implementing v0.6. Read this file first.
> **Scope:** two pillars — a shared AI judgment layer (Sentinel) and a modular
> distribution layer. No code was written while authoring these specs.
> **Version:** ships as **`v0.6.0`** (current shipped: `v0.5.0`). "v6" was
> shorthand for v0.6. The code version bump is a release-time step, not part of
> these specs.
> **First implementation step is [`06-naming-consistency-sweep.md`](06-naming-consistency-sweep.md)**
> (rename `forge → skills`) — done before any feature work.

---

## 1. What v0.6 is — two pillars

**Pillar A — Sentinel (make the USP true).** v0.6 makes "**uses AI to make AI
safe**" literally true by adding **Sentinel** — a tiny local AI that quietly
watches another AI's behaviour in real time, judges the gray zone the static
defences miss, explains its decisions in plain language, and escalates to a
powerful model only when the user deliberately asks it to.

**Pillar B — Modular distribution (make it lean to adopt).** v0.6 lets a user
install only what they want — one standalone shield via CLI, or the GUI with a
profile — instead of a five-container "install all to use 1/5th" app. The tools
were always modular *in code*; Pillar B adds the modular *distribution* that
ADR-0013's monorepo collapse left unbuilt. See
[`05-modular-distribution.md`](05-modular-distribution.md) + [ADR-0014](../../adr/0014-monorepo-modular-distribution.md).

The two pillars reinforce each other: because shields install standalone,
Sentinel is built as a **shared library** each shield embeds (not a GUI-only
service), which keeps both the everyday judgment lean *and* the tools genuinely
independent.

## 2. Why (the gap v0.6 closes)

A 2026-05-31 ground-truth audit found the USP is ~90% aspirational today:

| Concern | AI today | Reality |
|---------|----------|---------|
| Containerisation | none | shell level = user toggle; allowlist = 4 hardcoded domains; egress = static nftables; orchestrator = deterministic |
| Forge (skills) | one slice | 87-pattern scanner is pure regex; a local model (`qwen2.5-coder:1.5b`) is used only in one CDR stage |
| Social | none | 25 static regex patterns, parked, Moltbook-coupled |

The static layers are good (fast, cheap, auditable). What's missing is
**judgment** — catching the paraphrased/novel attack, adapting without a human
editing a regex, and *explaining* a decision. v0.6 adds exactly that, without
bloat.

## 3. The spine: Sentinel's escalation ladder

One shared judgment layer all three concerns consult. Each rung is rarer,
slower, costlier than the one below. Cheap rungs handle ~95%.

| Rung | What | When | Cost | Memory |
|-----:|------|------|------|--------|
| **0 — Static** | regex / allowlist / nftables (today's defences) | always, instant | free | ~0 |
| **1 — Embeddings** | similarity / anomaly / drift | constantly, background | ~free | ~45 MB (`all-minilm`) |
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

## 4. The three concerns / shields in v0.6

Each concern is both a **Sentinel leg** (Pillar A) and a **standalone-
installable shield** (Pillar B). The `openagent-*` name is the install/marketing
identity; the internal dir name stays short.

| Shield (install name) | Internal | Tagline | Leg spec |
|-----------------------|----------|---------|----------|
| `openagent-containment` | `workloads/agent` + `infra/{proxy,egress}` | "least-privilege, discovered not configured" | [`02-adaptive-containment.md`](02-adaptive-containment.md) |
| `openagent-skills` | `workloads/skills` *(renamed from `forge`)* | "anything that can't survive being described is gone" | [`03-cleanroom-skills.md`](03-cleanroom-skills.md) |
| `openagent-social` | `workloads/social` | "read the agent-web without becoming a vector" | [`04-semantic-firewall-social.md`](04-semantic-firewall-social.md) |
| *(internal, no install name)* | `app/src-tauri/src/sentinel/` | the shared judge | [`01-sentinel-spine.md`](01-sentinel-spine.md) |

> **Naming sweep (v0.6 implementation work, SD1 resolved):** rename `workloads/forge` →
> `workloads/skills`, container `vault-forge` → `vault-skills`, component id
> `forge` → `skills`. "Cleanroom" stays the *capability* name (the CDR pipeline);
> "skills" is the canonical identifier. Legacy `forge` / `openskill-forge`
> references (README, `forge-spotlight.md`, the pitch) get swept in the same pass.

**Sentinel gets no `openagent-*` name** — it fails the standalone-use test
(nobody installs it alone; it only judges fragments for the shields). The
`openagent-` prefix is a *distribution* identity, never an internal-module
prefix — internal dirs stay `agent`/`skills`/`social`/`proxy`/`egress`. Full
naming canon: [`05-modular-distribution.md`](05-modular-distribution.md) §2.

## 5. Build sequencing

Full execution plan — milestones, dependency graph, the first PR, the gate,
parallelism, and release staging — is in
[`07-implementation-roadmap.md`](07-implementation-roadmap.md). The shape:

```
M0 Naming sweep (gate, 06)  →  M1 Sentinel lib + skills (01 + 03)  →  ┌ M2 distribution (05)
                                                                      ├ M3 containment (02)
                                                                      └ M4 social (04)  [parallel]
```

- **M0 first** — the `forge → skills` rename ([`06`](06-naming-consistency-sweep.md)),
  on clean ground, so feature work builds on final names.
- **M1 next** — the Sentinel shared library proven on the skills leg (it already
  has the local model + the ZONE-4a bug the spine fixes). Foundation for all legs.
- **M2 / M3 / M4 in parallel** after M1 freezes the lib.

> **M0–M4 + rung-1 + the GUI Sentinel bridge/indicator + persona-drift + the
> disarm-diff display have LANDED** (on `main`, gated). The harmonised plan for
> the work remaining to ship `v0.6.0` — allowlist approval (threat-modeled),
> production Sentinel staging, the M4 live adapter, and closeout — is
> [`08-completion-plan.md`](08-completion-plan.md). Read it for the next session.

## 6. The anti-bloat contract (non-negotiable)

These constraints keep v0.6 lean. Any implementation that violates one needs a
new decision, not a workaround:

1. **One shared judgment layer, not three.** Build Sentinel once, as a shared
   library both the standalone CLIs and the GUI consume.
2. **Static-first, always.** Rungs 1–3 only run on what rung 0 can't resolve.
3. **Tiny default, load-on-demand.** Rung 2 unloads when idle. Rung 1
   embeddings (`all-minilm`, ~45 MB) are the lightest layer — Ollama loads the
   model on call and unloads it after its keep-alive window.
4. **Rung 3 never auto-fires.** No surprise slowdown, no surprise cost.
5. **No new dependency tier.** Ollama is already in the stack; embeddings are
   a small addition; the rung-3 cloud call reuses the agent's key + proxy.
6. **Coexists with the user's real agent.** Sentinel runs *next to*
   OpenClaw/opencode, never competing for the RAM the user needs.
7. **Install only what you use.** No user installs a container they don't need;
   the GUI is an *optional* layer over standalone-capable shields, never a
   prerequisite (Pillar B).

## 7. Glossary

- **Sentinel** — the shared judgment layer; an internal module/library, not a
  product (no `openagent-*` name). Code name only; the USP is the capability.
- **Rung** — one tier of the escalation ladder (0 static → 3 big judge).
- **Verdict** — Sentinel's structured output for a case: allow / block /
  escalate, plus a plain-language reason.
- **Disarm diff** — the human-readable before/after a skill goes through CDR.
- **Persona drift** — divergence between an agent's outgoing content and its
  established task/voice.
- **The silo** — OpenTrApp's containment promise: *untrusted content never
  reaches the host unfiltered; credentials never leak.* (NOT "no cloud LLM" —
  the agent already calls one.)

## 8. Decisions

### Resolved
| # | Decision | Resolution |
|---|----------|------------|
| D1 | Name for "Sentinel" / the tiny-AI USP | **No separate product/marketing name.** Sentinel is an internal shared module/library (fails the standalone-use test). `sentinel` is the internal code name only; the USP is the *capability* ("a tiny local AI makes AI safe"), not a sub-brand. Renaming the code name is low-stakes. |
| D7 | Modularity model | **Monorepo + modular distribution** (not separate repos; ADR-0013 stays). See [`05`](05-modular-distribution.md) + ADR-0014. |
| D8 | Standalone-shield naming | **`openagent-*` family** — `openagent-containment` / `openagent-skills` / `openagent-social`. Distribution identity only. |
| SD1 | `forge` vs `skills` internally | **Rename to `skills`** — `workloads/forge` → `workloads/skills`, `vault-forge` → `vault-skills`, id `forge` → `skills`. Implementation sweep in the modular-distribution leg. |
| SD2 | `containment` vs `runtime` | **`openagent-containment`** — the product is about containment; "runtime" undersells the three-container fence. |
| D2 | Embedding model (rung 1) | **`all-minilm`** (all-MiniLM-L6-v2, ~45 MB, 384-dim, Apache-2.0) — runs via the same local Ollama, no second runtime. Built + calibrated. Finding banked: `drift` (vs the agent's own voice) is the reliable signal and may gate; `score` (similarity to a small known-bad corpus) is a recall-safe **booster** that catches near-duplicates but misses novel paraphrases, so it must NOT suppress rung 2. See [`01`](01-sentinel-spine.md) §4. |
| D3 | Rung-2 judge model | **`qwen2.5-coder:3b`** (~1.9 GB) — empirically the smallest local model with adequate gray-zone precision: allows a benign documented command (5/5), blocks exfil, resists judge-injection. The 1.5b over-blocked; 3b resolves it and fits alongside the agent. The **parser** (CDR describe) stays on the leaner 1.5b — parser failures are schema-detectable + retry-recoverable, judgment is not. (M1 + the bigger-model re-test.) |
| D4 | Sentinel lib packaging | **bash lib callable from a bare CLI** — `sentinel/judge.sh` reads a JSON request on stdin, writes a Verdict on stdout, against local Ollama. No GUI/Rust-app needed (lib-first). (M1.) |
| D6 | Version | **Ships as `v0.6.0`** (current shipped: v0.5.0; "v6" was shorthand). Code version bump is a release-time step. |

### Open (resolve during implementation)
| # | Decision | Owner | Where |
|---|----------|-------|-------|
| D5 | "Confirmed edge case" threshold (alert-fatigue floor; default `SENTINEL_ESCALATE_BELOW=0.35`, tunable) | implementer | [`01`] §escalation |

The v0.6-completion sub-decisions are **resolved** (2026-06-01): SD-A1 Always+Deny
(defer allow-once), SD-A2 remember-deny, SD-B1 bind-mount dev / image-copy release,
SD-B2 no-bundle-Ollama, **SD-C1 AT Protocol first**. See [`08`](08-completion-plan.md) §9.

## 9. Relationship to existing docs

- Architecture this builds on: [`docs/perimeter-explained.md`](../../perimeter-explained.md), [`docs/trifecta.md`](../../trifecta.md)
- Invariant Sentinel must honour: [ADR-0002](../../adr/0002-adaptive-shell-levels.md) (agent cannot self-promote privilege)
- Credential model rung-3-cloud reuses: [ADR-0001](../../adr/0001-proxy-side-api-key-injection.md)
- The CDR origin the Cleanroom leg extends: [ADR-0003](../../adr/0003-content-disarm-reconstruction.md)
- The monorepo this builds on (and does NOT revert): [ADR-0013](../../adr/0013-monorepo-consolidation.md)
- The modular-distribution + naming decision record: [ADR-0014](../../adr/0014-monorepo-modular-distribution.md)
- A further ADR should record the Sentinel judgment-layer decision once the
  spine lands (suggested: ADR-0015 — local-AI judgment layer).
