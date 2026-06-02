# Release notes — v0.6.0 (A tiny local AI makes AI safe)

v0.6.0 makes the project's thesis — *"uses AI to make AI safe"* — real and
demonstrable rather than aspirational. A tiny, local judgment layer (**Sentinel**)
now watches the gray zone the static defences miss, across all three concerns:
runtime containment, the skill supply chain, and the agent-social feed. The
everyday judge is local and cheap, so it can be consulted constantly; the
powerful, privacy-spending option is always a deliberate, visible choice.

## What changed

### Sentinel — a local AI judgment ladder

A shared escalation ladder all three concerns consult (ADR-0015): static rung 0
→ embeddings rung 1 (`all-minilm`) → a tiny local LLM judge rung 2
(`qwen2.5-coder:3b`) → a rare, human-triggered rung 3. Cheap rungs handle the
common case; the expensive ones only fire when the cheap ones genuinely can't
resolve a case. The judge is load-on-demand and injection-hardened, and a
malformed answer always escalates — never a silent allow.

A **tiered-model** finding underlies it: give the bigger model only to the role
whose mistakes you can't otherwise catch. The CDR parser stays on the leaner
`qwen2.5-coder:1.5b` (its failures are schema-detectable and retry-recoverable);
the judge gets `qwen2.5-coder:3b` (its failures are not self-checking).

### Adaptive containment — explained, one-tap allowlist decisions

An off-allowlist request is no longer a blunt block. Sentinel judges the
gray-zone hosts and surfaces the allow-leaning ones as **pending approvals** with
a plain-language reason; you tap "Allow always" or "Block". By construction, only
the human can ever loosen the perimeter — the agent has no path to do so, the
judge only recommends, and clear exfiltration is hard-blocked and never reaches
the judge (ADR-0002, ADR-0016).

### The skills cleanroom — a judge second opinion + a disarm diff

The 87-pattern scanner stays as the cheap pre-filter, and the local judge is now
a **fail-safe second opinion** on what the scanner would auto-allow: it can only
tighten a clean verdict, never loosen a quarantine, catching a novel or
paraphrased skill-level threat the regexes miss. CDR is now reliable
(retry-with-repair, quarantine-never-silent), and a plain-language **disarm diff**
shows exactly what was removed from a skill before it was rebuilt.

### The semantic firewall — live, on AT Protocol

The social shield is un-parked behind a protocol-adapter abstraction and a first
**live** network adapter for **AT Protocol (Bluesky)** (ADR-0017): the rung-2
judge catches paraphrased injections the 25 static patterns miss, and an
outgoing **persona-drift** guard holds a post that no longer matches the agent's
own voice. Reads use the public AppView (no auth); the leg is opt-in and the
perimeter never auto-participates.

### Modular distribution + the `skills` rename

A single `distribution.yml` drives profile-based image bundling and a standalone
per-shield installer, so a user can install only what they want (ADR-0014). The
`forge` workload is renamed to `skills` throughout (`vault-skills`,
`openagent-skills`).

---

## Breaking changes

None for end users. Existing credentials and settings carry forward. The
container `vault-forge` is renamed `vault-skills`; a fresh perimeter is brought
up on first launch.

## New runtime requirement (optional)

The local-AI rungs need [Ollama](https://ollama.com/) reachable on the host with
`qwen2.5-coder:1.5b`, `qwen2.5-coder:3b`, and `all-minilm` pulled. Without it the
fast static defences still run; the AI rungs degrade fail-safe (hold for review)
rather than judging automatically.

## Known issues

- The Sentinel rung-2/rung-1 features require Ollama as above; they are inert
  (fail-safe) without it.
- The live social adapter is opt-in and validated against AT Protocol; other
  networks (Mastodon, Nostr) are future adapters behind the same contract.

## Upgrade path

Standard auto-update — the Tauri updater will prompt in-app. To update manually,
download the installer for your platform from the assets below and run it over
the existing installation.

## Full commit range

`git log --oneline v0.5.0..v0.6.0` — the v0.6 reassessment: M0–M4 (rename,
Sentinel judge lib + CDR fix, modular distribution, adaptive containment,
semantic firewall), rung-1 embeddings, the GUI Sentinel bridge + activity
indicator, persona-drift, the disarm-diff display, and the completion items —
production Sentinel staging, host-mediated allowlist approval, the live AT
Protocol adapter, and the judge-as-second-opinion on skill auto-allow.
