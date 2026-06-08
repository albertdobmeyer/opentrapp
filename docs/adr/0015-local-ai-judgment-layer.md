# ADR-0015 — Local-AI judgment layer (Sentinel)

**Status:** Accepted — M1 shipped (rung-2 judge lib + rung-1 embeddings landed; GUI rung-3 UX is a later milestone)
**Companion spec:** [`docs/specs/v0.6/01-sentinel-spine.md`](../specs/v0.6/01-sentinel-spine.md)
**Cross-references:** [ADR-0001](0001-proxy-side-api-key-injection.md) · [ADR-0002](0002-adaptive-shell-levels.md) · [ADR-0003](0003-content-disarm-reconstruction.md) · [ADR-0013](0013-monorepo-consolidation.md) · [ADR-0014](0014-monorepo-modular-distribution.md)

---

## Context

Through v0.5.0 the project's USP — "uses AI to make AI safe" — was approximately
90 % aspirational. A 2026-05-31 audit established the gap:

| Concern | AI today | Reality |
|---------|----------|---------|
| Containerisation | none | shell level = user toggle; egress = static nftables |
| Skills (forge) | one slice | 87-pattern scanner is pure regex; a local model is used only in one CDR stage |
| Social | none | 25 static regex patterns, parked |

The static layers are good — fast, cheap, auditable. What is missing is **judgment**:
catching the paraphrased / novel attack, adapting without a human editing a regex,
and *explaining* a decision. The gap is the "gray zone": cases the static rules flag
as suspicious but cannot decisively classify. Closing it requires a semantic layer.

Two constraints must hold:

1. **No always-on model.** The judgment layer coexists with the user's real agent
   on a 7–8 GB machine. A model that loads eagerly would compete for the RAM the
   agent needs.
2. **No cloud-only path.** An LLM API call on every ambiguous fragment would
   introduce latency, cost, and a privacy risk for every static-layer hit, making the
   tool unusable in high-traffic scenarios.

An escalation ladder with cheap-by-default rungs resolves both.

## Decision

Add **Sentinel** — a tiny local-AI judgment layer that closes the gray zone the
static defences miss — as the shared judgment layer for all three concerns (skills,
containment, social).

### 1. The escalation ladder

Sentinel is structured as a four-rung ladder where each rung is rarer, slower, and
costlier than the one below. Cheap rungs handle ~95 % of cases.

```
caller ── content + context ──▶ Rung 0 (static, caller-side)
                                   │ pass → return allow
                                   │ clear-hit → return block
                                   │ ambiguous ▼
                                Rung 1 (embeddings, all-minilm ~45 MB)
                                   │ confident → return verdict
                                   │ uncertain ▼
                                Rung 2 (tiny LLM, load-on-demand)
                                   │ confident → return verdict
                                   │ still-uncertain → mark edge case ▼
                                Rung 3 (human-first, user-triggered)
                                   (a) local-big   (b) cloud-fragment   (c) human decides
```

| Rung | What | When | Memory |
|-----:|------|------|--------|
| **0 — Static** | regex / allowlist / nftables (existing defences) | always, instant | ~0 |
| **1 — Embeddings** | similarity to known-bad corpus; anomaly / drift | background, constant | `all-minilm` ~45 MB; loaded on call |
| **2 — Tiny LLM** | semantic judgment on ambiguous flagged cases | only when rung 0/1 are uncertain | load-on-demand; unloads after idle window |
| **3 — Big judge** | confirmed edge case rung 2 cannot resolve | **user-triggered, never auto-fires** | heavy; local-big or cloud-fragment |

Rungs 0–2 run automatically. The rung 2 → 3 transition does **not** auto-run; it
produces an `escalate` Verdict that the concern leg surfaces to the user. Rung 3 is
always human-first.

### 2. Rung 3 — three options, never automatic

When a leg surfaces an `escalate` Verdict the user picks one of:

- **(a) local-big:** pause the agent via `pause_perimeter` (`lifecycle.rs`), load a
  bigger local model, judge, unload, `resume_perimeter`. Slow, free, fully private.
- **(b) cloud-fragment:** send only the minimal `fragment_to_send` to a cloud model
  through the existing `vault-proxy`, reusing the API key already in the proxy
  (ADR-0001) — no new credential, no new egress path. The user sees the exact bytes
  that will leave before sending and gives explicit consent. Privacy-max users disable
  this option in config.
- **(c) human decides:** show rung 2's reasoning and the evidence; the user rules.
  Always available; the default if both model options are declined.

### 3. The model-tiering principle (load-bearing finding from M1)

Give the bigger model only to the role whose mistakes you cannot otherwise catch.

The tiny model plays two roles in the stack. As a **parser** (the CDR "describe"
step: untrusted skill → structured intent JSON) its failure mode is *invalid JSON*,
which is schema-detectable and retry-recoverable — the validator catches it and the
model is re-prompted with the error. As a **judge** (gray-zone allow/block) its
failure mode is a *plausible wrong verdict*, which is not self-checking; a confident
wrong answer is indistinguishable from a right one.

Therefore:

- The **parser** defaults to `qwen2.5-coder:3b` (`CDR_MODEL`) as of 2026-06-08
  (revised — see note). Parser failures are schema-detectable and retry-recoverable,
  so the smaller 1.5b also works and remains a supported lean override; the default
  was raised to 3b for higher first-pass rebuild fidelity (the 1.5b drifts more) and
  so a full install shares **one** coder model with the judge instead of carrying
  both 1.5b and 3b.
- The **judge** runs on `qwen2.5-coder:3b` (`SENTINEL_MODEL`). This is the smallest
  local model with adequate gray-zone precision, as established by M1 testing: the
  1.5b over-blocked a benign `curl` documentation example; the 3b allowed it 5/5
  while still blocking exfiltration and resisting judge-injection. Spending model
  size on the judge buys precision that cannot be obtained any other way.

Both are env-overridable. A larger machine can point the judge at a 7b/14b for
greater precision; a smaller machine can drop **either** to 1.5b — the parser's
failure mode is caught by schema validation, and a 1.5b judge over-blocks (flagged
content surfaces for review rather than auto-allowing). The CDR rebuild can also be
pointed at a model you already run (Ollama-native or OpenAI-compatible) via
`config/cdr.conf`, so 3b is a default, not a forced download.

**Revision (2026-06-08):** the parser default moved from 1.5b → 3b at the owner's
request, prioritising rebuild fidelity and a single shared coder model over the
small per-run RAM saving. This partially adopts the "single model for both" option
the original draft rejected (below); the rejection's logic still holds (1.5b is
*sufficient* for the parser because its failures are recoverable), so 1.5b stays a
first-class, documented override for memory-constrained machines. The trade is
honest: 3b uses ~2.5 GB resident per CDR run vs ~0.8 GB for 1.5b.

### 4. Lib-first (D4 — resolved)

Sentinel is a **shared library**, not a GUI-only service. This is required by the
modular distribution constraint (ADR-0014): a standalone `openagent-skills` install
has no GUI and no Tauri runtime, yet still needs rungs 0–2. The judgment logic
cannot live only behind Tauri commands.

The shape:

- **The shared lib** (`sentinel/`) — portable bash/Python assets: the rung-2 judge
  prompt + Verdict schema + Ollama-call helper, the escalation contract, and the
  rung-1 corpus helpers. A standalone CLI calls `sentinel/judge.sh` directly against
  local Ollama with no parent app. Rungs 0–2 need nothing but the tool, the lib, and
  Ollama.
- **The GUI is one consumer of the lib.** `app/src-tauri/src/sentinel/` orchestrates
  the shared lib for GUI mode, exposes Tauri commands + activity-indicator events,
  and owns the rich rung-3 UX (visual banner, `pause_perimeter` consent dialog).
- **Standalone CLIs** get a text-prompt rung 3 — `escalate? [local-big / cloud /
  decide]` — the same contract, a terminal UX instead of a GUI banner.

A `vault-sentinel` container was considered and rejected: it would add a sixth
container, require a model runtime inside it plus IPC, and break the standalone-CLI
path (a CLI tool cannot depend on a running container to judge a line).

### 5. Acceptable-on-host rationale

The judge runs on the host (or in the calling tool's process). It reads only
*already-untrusted* fragments that the static layer has already flagged. It never
executes those fragments — the injection-hardened system prompt instructs the model
to treat the fragment as content to *analyze*, not instructions to *obey*. An attempt
to manipulate the judge is itself treated as a danger signal.

This satisfies the project's "no untrusted content on the host filesystem" constraint
(CLAUDE.md §10): the fragments are already in memory as flagged signals, not executed
code, and the judge produces a structured Verdict that the calling layer acts on.

### 6. No `openagent-*` name (D1 — resolved)

Sentinel fails the standalone-use test (ADR-0014 §3): nobody installs it alone; it
only judges fragments for the shields. The `openagent-` prefix is a distribution
identity, never an internal-module prefix. Sentinel is `sentinel` — an internal
shared library, no product name.

## Consequences

### Positive

- **The USP is now literally true.** "Uses AI to make AI safe" describes a real
  running component, not an aspiration.
- **Gray-zone coverage without bloat.** Rungs 0–2 cost ~0 at rest; rung 2 loads
  only when the static layer genuinely cannot resolve a case, and unloads after an
  idle window.
- **All three concerns share one judgment layer.** No duplicated prompts, no
  per-concern model management. The escalation contract is a single interface.
- **Injection resistance is structural.** The judge prompt is hardened against the
  exact attack vector it watches for; a fragment that tries to flip the verdict
  becomes evidence against itself.
- **Rung-3 cloud is scoped + consented.** Only the minimal fragment leaves;
  it travels through the existing `vault-proxy`; the user sees the exact bytes before
  they go. No new credential, no new egress path.
- **Privacy-max users are not coerced.** Cloud is an opt-in rung-3 option; the
  everyday path is fully local.

### Negative

- **Model management overhead.** Rung 2 adds a load-on-demand lifecycle (Ollama
  `keep_alive` / unload on idle). The activity indicator must expose the rung state
  so the user never wonders why their machine got busy.
- **Two binding levels to maintain.** The lib-first design means every non-trivial
  Sentinel change must be tested against both the standalone-CLI path and the
  Tauri-command path.
- **Rung 1 (embeddings) landed (D2 — resolved: `all-minilm`).** A calibration
  finding constrains its use: `drift` (an outgoing post vs the agent's own voice) is a
  reliable signal that may gate, but `score` (similarity to a small known-bad corpus)
  is a recall-safe *booster* — it catches near-duplicates yet misses novel paraphrases,
  so a low/clean similarity must never suppress the rung-2 judge. See spec [`01`](../specs/v0.6/01-sentinel-spine.md) §4.
- **Alert fatigue is a real risk.** Rung-3 escalations must be rare; the escalation
  floor (D5 — default `SENTINEL_ESCALATE_BELOW=0.35`) must be tuned against real
  traffic to stay below alert-fatigue threshold.

### Risks accepted

- **Wrong verdicts are possible.** The 3b model can mis-judge a novel gray-zone
  case. Mitigations: the static layer blocks clear-bad cases before rung 2 sees
  them; the escalation path surfaces uncertain cases to the human; rung-3 options
  allow a more capable model on hard cases. The residual risk is accepted; the
  alternative (no judgment layer) leaves the full gray zone uncovered.
- **Ollama dependency.** Rung 2 requires Ollama to be running. The lib emits an
  `escalate` Verdict on `ECONNREFUSED` and the caller decides its
  fail-closed/fail-open policy. Rung 0 static defences are unaffected.

## Alternatives considered and rejected

- **A `vault-sentinel` container.** Would require a sixth container, a model runtime
  inside it, and IPC. Breaks the standalone-CLI path; a CLI tool cannot depend on a
  running container. Rejected in favour of lib-first.
- **Cloud-only judgment (always call the agent's API).** Too expensive and too slow
  to call on every ambiguous static hit; introduces a privacy risk for every rung-0
  flag. Rejected; cloud is rung-3, scoped, and consented.
- **Single model for both parser and judge.** Originally rejected: using 1.5b
  everywhere under-powers the judge (it over-blocked the benign gray zone in M1
  testing), and the tiered split was the measured finding. **Partially adopted
  2026-06-08:** the parser default was raised to 3b (matching the judge) for rebuild
  fidelity and one shared model. The original rejection's core point still stands —
  1.5b is *sufficient* for the parser because its failures are schema-caught — so
  1.5b remains a documented override; what changed is the default's weighting toward
  fidelity over a small RAM saving. The judge is still never dropped below 3b by
  default (its mis-verdicts are not recoverable).
- **Per-concern judgment layers.** Each concern (skills, containment, social) could
  embed its own model and prompts. Rejected: three separate model lifecycles, three
  prompt codebases to keep injection-hardened, three times the RAM cost at rung 2.
  The shared-lib design preserves modularity without duplicating the cost.

## Cross-references

- [ADR-0001](0001-proxy-side-api-key-injection.md) — the proxy-side API key rung-3 cloud reuses (no new credential).
- [ADR-0002](0002-adaptive-shell-levels.md) — the agent-cannot-self-promote invariant Sentinel must honour.
- [ADR-0003](0003-content-disarm-reconstruction.md) — the CDR origin from which the rung-2 Ollama integration and the injection-hardened prompt were generalised.
- [ADR-0013](0013-monorepo-consolidation.md) — the monorepo this builds on.
- [ADR-0014](0014-monorepo-modular-distribution.md) — the modular-distribution constraint that forces the lib-first design.
- [`docs/specs/v0.6/01-sentinel-spine.md`](../specs/v0.6/01-sentinel-spine.md) — the full mechanical spec (escalation thresholds, interface, model layer, activity indicator, configuration, test strategy).
- [`docs/specs/v0.6/00-index.md`](../specs/v0.6/00-index.md) §8 — resolved decisions D1, D3, D4 (no product name, judge model, lib packaging).
- [`sentinel/README.md`](../../sentinel/README.md) — what shipped in M1.
