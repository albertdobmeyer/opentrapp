# ADR-0025 — Standalone Skill Firewall: scan-on-host scope & claim boundary

**Status:** Accepted — scopes the standalone delivery of the skill scanner + CDR (Phase 2 of the
top-level roadmap). Realizes the standalone `openagent-skills` direction ratified in
[ADR-0014](0014-monorepo-modular-distribution.md), within the structure of
[ADR-0024](0024-product-structure-three-concerns.md).

**Cross-references:** [ADR-0003](0003-content-disarm-reconstruction.md) (CDR pipeline) ·
[ADR-0006](0006-four-container-topology.md) (why the scanner is isolated) ·
[ADR-0014](0014-monorepo-modular-distribution.md) (modular distribution) ·
[ADR-0024](0024-product-structure-three-concerns.md) (three concern sub-apps) ·
[CLAUDE.md §9 (no untrusted content on host) / §11 (gate the claim)](../../CLAUDE.md)

---

## Context

USP-2 — the skill scanner + Content-Disarm-Reconstruction — runs **isolated in `vault-skills`**
inside the perimeter, where the agent cannot tamper with it ([ADR-0006](0006-four-container-topology.md))
and untrusted skill content never touches the host ([CLAUDE.md §9](../../CLAUDE.md)).

The opencode mission needs a **lower-activation-energy** offering than "adopt a five-container
perimeter": opencode's plugin/skill/MCP ecosystem has **zero vetting**, and a single offline
`skill scan <path>` that a user runs as an `opencode plugin install` pre-hook is a far smaller ask.
The scanner is already host-runnable — bash + `python3` stdlib, a runtime-computed `REPO_ROOT`, no
container assumption (the self-test is 10/10 on the host). So the standalone delivery is mostly a
thin entrypoint. The open question this ADR answers: a standalone scanner runs **on the host**, which
sits in tension with the §9 promise "no untrusted content on the host." We scope that honestly rather
than let the standalone tool quietly erode the perimeter's headline claim.

## Decision

1. **Ship the scanner/CDR as a standalone `skill` CLI** (`workloads/skills/skill`) runnable on the host
   with **no perimeter**:
   - **Tier A — `skill scan` / `skill verify`:** the 87-pattern blocklist + zero-trust line classifier.
     **Fully offline — no model, no network.**
   - **Tier B — `skill cdr`:** the model-backed rebuild. **Bring-your-own model** (`config/cdr.conf`:
     a local Ollama model or any OpenAI-compatible endpoint). Optional; Tier A is the wedge.
   - **Exit-code contract:** `0` clean · `1` finding (blocks an install when used as a pre-hook) ·
     `2` usage. Same code, two delivery modes (isolated in `vault-skills`; or standalone on the host) —
     one source of truth, no fork.

2. **The claim boundary (the honest part).** The standalone tool **reads and pattern-matches text — it
   does not execute the skill.** Its guarantee is *"vet (and optionally rebuild) a skill **before** an
   agent loads it,"* **not** *"no untrusted content ever touches the host."* The stronger §9 guarantee —
   untrusted content is processed only inside a container, never on the host — **remains the full
   perimeter's (the Vault).** Each artifact claims **only what it delivers** ([CLAUDE.md §11](../../CLAUDE.md)):
   - *Standalone Skill Firewall:* "an offline supply-chain check you run before installing a skill/plugin."
   - *The Vault:* "untrusted skill content is downloaded, scanned, and rebuilt **inside an isolated
     container the agent can't tamper with**, never on your host."

3. **The clearance report (`.trust`, SHA-256) is a portable trust artifact** ([ADR-0003](0003-content-disarm-reconstruction.md))
   — a standalone or non-perimeter consumer can verify a skill against it.

## Consequences

**Positive**
- The **opencode adoption wedge**: a single offline CLI is a credible "yes" where the full perimeter is
  not. USP-2 becomes **independently true and independently adoptable** — the strongest mission lever.
- No code fork: the standalone CLI dispatches to the same `tools/skill-*.sh` the perimeter uses.

**Negative / cost (stated honestly)**
- Tier A reads untrusted **text** on the host. That is materially lower-risk than *executing* a skill
  (it never runs the content; bash + `python3` stdlib, pattern-matching only) — but it is **not zero**
  (a pathological input could stress the parser). Users who want the strong "nothing untrusted on the
  host" property route through the Vault. This trade-off is the whole subject of this ADR; it must be
  stated wherever the standalone tool is offered, not buried.
- Two delivery modes mean a **claim-scoping discipline**: the standalone tool must never be marketed
  with the perimeter's guarantee.

## Alternatives considered

- **Only ship inside the perimeter (no standalone).** Rejected — that is precisely the
  high-activation-energy ask opencode's terminal users will not take; the standalone scanner is the wedge.
- **Claim the standalone tool gives the full "no untrusted content on host" guarantee.** Rejected — it is
  false (scanning reads text on the host). Scope honestly per §11.
- **Fork a separate standalone codebase.** Rejected — one source of truth; the same scripts run in both
  modes.

## What this ADR does NOT decide

- The user-facing **tool→guard naming sweep** ("Workshop" → "Skill Firewall" in the GUI/GLOSSARY, behind
  the 28-term ban test) — a separate, careful change.
- The eventual **`opentrapp skill …` top-level command** wiring (depends on the GUI demotion / `opentrapp`
  rename — [ADR-0022](0022-daemon-control-surface.md) / Phase 3).
