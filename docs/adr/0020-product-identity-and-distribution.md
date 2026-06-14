# ADR-0020 — Product identity & distribution: a CLI/daemon with projected viewers

**Status:** Accepted — *direction* ratified 2026-06-14; implementation **staged** (see
[ADR-0021](0021-danger-gated-agentic-control-plane.md) — security spine, and
[ADR-0022](0022-daemon-control-surface.md) — control surface / de-Tauri). **Honest current state:**
OpenTrApp ships *today* as a Tauri 2 desktop app; only `opentrapp-core` / `opentrapp-daemon` are
already Tauri-free ([ADR-0019](0019-headless-daemon-gui-viewer-split.md)). This ADR records the
**TARGET** identity the project is moving toward — **not** a completed migration. Nothing here claims
the CLI/registry/MCP surfaces exist yet.
**Cross-references:** [ADR-0008](0008-tauri-over-electron.md) (Tauri choice — *refined*) ·
[ADR-0014](0014-monorepo-modular-distribution.md) (modular distribution) ·
[ADR-0019](0019-headless-daemon-gui-viewer-split.md) (daemon-is-the-product — *extended*) ·
[ADR-0002](0002-adaptive-shell-levels.md) · [ADR-0016](0016-host-mediated-allowlist-loosening.md) ·
[CLAUDE.md §1/§5/§11](../../CLAUDE.md)

---

## Context

Four pressures converged and forced an explicit statement of *what OpenTrApp is as a distributable
artifact* — a question the codebase had answered only implicitly, and increasingly inconsistently:

1. **The desktop-app framing (ADR-0008) is straining.** Tauri's Linux webview pulls
   `webkit2gtk` + GTK3, which surfaces 19 upstream `unmaintained`/`unsound` OpenSSF advisories.
   They are **non-exploitable, viewer-only, and not in the security boundary** (`cargo tree -i`
   proves every one routes through the `opentrapp` GUI crate; the daemon/core are clean), but
   Scorecard counts them and cannot read our `deny.toml` acceptance — reputationally relevant
   before approaching opencode or forming an org.
2. **ADR-0019 already split the product.** The five-container perimeter is owned by a headless,
   Tauri-free **daemon**; the GUI is a *disposable projection*. "The daemon is the product" is
   already true in the backend; the desktop-app identity is a legacy of how *one* projection ships.
3. **The audience is developers running CLI agents.** The mission is to be the recommended
   containment for opencode. For that audience, `brew install` / `cargo install` / `curl | sh` is
   more credible and natural than a downloaded `.dmg`, and Scorecard's **Packaging** check wants a
   *registry-native publish* — which native installers + a hand-rolled GHCR image push do not
   satisfy (hence the persistent Packaging `?`).
4. **A new requirement crystallized: OpenTrApp should be *agent-operable*.** A user's **host agent**
   (e.g. Claude Code) should be able to install, configure, run, and observe OpenTrApp. This is
   *agent orchestrating an agent*: the **external, trusted operator** drives OpenTrApp, which
   **contains** the internal agent. This is the inverse of — and must never be confused with —
   exposing controls to the *contained* agent (a security inversion; see Decision).

## Decision

OpenTrApp's product identity is:

> **A registry-installable CLI/daemon orchestrator + signed container images, with a web GUI and an
> optional MCP adapter as thin, on-demand projections of the same manifest-driven daemon.**

Ratified as six tenets (the canonical nutshell):

1. **The daemon is the product.** Headless, Tauri-free, owns the perimeter lifetime. CLI + GUI + MCP
   are thin projections over **one** manifest-driven command set (extends ADR-0019).
2. **CLI-first, registry-native distribution.** The CLI/daemon ships via crates.io / Homebrew /
   `curl | sh`; images via GHCR. Developer- and agent-installable — and this is what makes the
   Scorecard Packaging check legible.
3. **Agent-operable ("agentic").** The same command surface is driven by humans, Claude Code,
   opencode, or any host agent. The *external* operator orchestrates the *internal* (contained) agent.
4. **Two agents, one inviolable rule.** The **external host agent** is a *trusted operator*
   (legitimate control). The **internal contained agent never controls its own cage**
   (ADR-0002/0016).
5. **Danger-gated control plane (security non-negotiable).** Read/safe operations are freely
   agent-operable; **boundary-weakening** operations (manifest `danger: high` — loosen allowlist,
   pause perimeter, edit egress policy) are **human-gated even when an agent asks.** This is what
   keeps "agentic" from meaning "trivially disarmable by a prompt-injected host agent." Specified in
   ADR-0021.
6. **The GUI is a disposable, on-demand config projection** for non-technical users — not a
   competing daily-use surface (the user lives in Telegram / opencode / the terminal), not a desktop
   app, not the product.

**Explicit non-identities:**

- **Not fundamentally a desktop app.** The Tauri desktop app is the *current delivery* of one
  projection, being phased out (the de-Tauri work, ADR-0022). The identity is the daemon + images.
- **Not an MCP server for the *contained* agent.** Exposing perimeter controls to the agent inside
  the cage is a security inversion. The codebase already encodes this instinct — the skill-scanner
  CDR deliberately refuses MCP/tool-use so the analysis model stays capability-free. An MCP adapter,
  *if* built, serves the **external** host operator only, under the danger-gated rule (tenet 5).

## Consequences

**Positive**
- A single, coherent answer that unifies four previously-tangled questions (de-Tauri, packaging,
  the agentic mission, the security model). Each downstream decision now has a north star.
- Registry-native distribution makes Packaging Scorecard-legible *and* makes the tool installable by
  the exact audience (developers + their agents) the mission targets.
- The de-Tauri path (ADR-0022) removes the 19 GTK3 advisories from the shipped binary *genuinely*
  (not relocated), making the best-practices claim defensible end-to-end.
- "Agent-operable" turns OpenTrApp into a native citizen of agent ecosystems — an adoption strategy,
  not just a feature.

**Negative / cost (stated honestly)**
- This is a *direction*, not a finished state. Until ADR-0022 lands, OpenTrApp remains a Tauri
  desktop app and the Packaging/Vulnerabilities Scorecard checks are unchanged.
- The agentic control plane is a **new attack surface on the containment itself** (a prompt-injected
  host agent could try to weaken the perimeter). This is *the* risk the identity introduces, and it
  is the entire subject of ADR-0021 — the identity is only safe *with* the danger-gated invariant.
- A CLI/daemon + on-demand web GUI changes the install/UX story for the non-technical persona
  (background service + browser config panel instead of a native window). ADR-0022 owns that design.

## Alternatives considered

- **Extract the GUI into a separate `OpenTrApp-GUI` repo.** Rejected: optics-only — it *relocates*
  the advisories to the GUI repo rather than removing them, doesn't change the installed binary,
  reverses the deliberate ADR-0013 monorepo consolidation on the same failed "lifecycle test," and
  risks reading as score-gaming.
- **Stay a desktop app.** Rejected as the *identity*: Scorecard-illegible packaging, Tauri/GTK3-bound,
  and less credible to the developer/agent audience the mission targets. (Tauri is not "wrong" — it
  was a sound choice per ADR-0008; it is simply not the *product*, only one projection's delivery.)
- **Expose OpenTrApp as an MCP server for the contained agent.** Rejected: security inversion.

## What this ADR does NOT decide (staged)

- The **danger-gated control-plane security model** + the agentic threat model → **ADR-0021** (must
  be ratified before any control surface is built).
- The **daemon control surface** (CLI · on-demand loopback web GUI / de-Tauri · optional MCP
  adapter) and the loopback-server security spec → **ADR-0022**.
- **Packaging mechanics** (crates.io / Homebrew / recognized release automation; recognized GHCR
  publish; the `openagent-*` modular artifacts per ADR-0014) → the Tier-3 distribution spec.
- The **C1 fallback** (track Tauri/wry GTK4 / webkitgtk-6.0 upstream; clears the advisories without
  the migration if it lands) is a no-regret parallel, not superseded by this ADR.
