# ADR-0014 — Monorepo dev-home + modular distribution + `openagent-*` naming

**Status:** Proposed — 2026-05-31 (part of the v0.6 reassessment; see [`docs/specs/v0.6/`](../specs/v0.6/00-index.md))
**Relationship to [ADR-0013](0013-monorepo-consolidation.md):** extends, does **not** revert. The monorepo stays the development home; this ADR adds the modular *distribution* layer ADR-0013 left unbuilt.
**Companion spec:** [`docs/specs/v0.6/05-modular-distribution.md`](../specs/v0.6/05-modular-distribution.md)

---

## Context

ADR-0013 collapsed the three submodules into one monorepo (`workloads/` +
`infra/`) for development ergonomics — and was right to: the submodules
co-shipped in lockstep, had zero external consumers, and the submodule tax
bought nothing.

But the collapse optimised for *maintainability* and inadvertently undercut a
property that was a deliberate **product** feature from the project's origin:
**modularity of distribution.** The app grew as three standalone CLI tools
(skill scanner, social feed scanner, agent container), each independently
useful, with the GUI added later purely as an optional bundling layer for
non-technical users. The value proposition for technical users was *"download
one lean tool for one purpose and drive it via CLI"* — and the broader bet was
that people are more likely to adopt a small focused tool than a large opaque
app claiming to do five things.

A 2026-05-31 audit established the key fact: **the tools are still modular at the
code level.** Each `workloads/{forge,social,agent}` runs standalone via its own
Makefile CLI with zero cross-tool imports; the manifest-driven GUI renders a
dashboard for whatever tools are present. The *only* thing monolithic is
distribution: `app/src-tauri/build.rs` bundles all three manifests + all five
container images into one AppImage, and the bootstrap brings up all five
containers. There is no way to install just one tool.

So the problem ("giant bloated app, install all to use 1/5th") is a
**distribution-layer** problem, not a repo-structure problem. It does not
require reverting ADR-0013.

## Decision

**Keep the monorepo as the single development home; add a modular distribution
layer so each tool is independently installable.** This is the "one repo, many
independently-installable artifacts" pattern (Babel, Cargo, Next.js). Three
parts:

### 1. Three distribution modes
- **Standalone CLI per shield** — install one tool + its container image(s) + a
  thin CLI wrapper. No GUI, no other tools, no five-container AppImage.
- **GUI with install-profiles** — the desktop app installs a chosen profile
  (`containment` / `containment+skills` / `containment+social` / `all`) and
  renders only the present tools' dashboards (the manifest discovery already
  supports this).
- **Monorepo** — clone/edit/build, unchanged (the ADR-0013 dev experience).

### 2. `openagent-*` naming for the standalone shields
The three standalone-installable shields carry a distribution/marketing family
name; the shared judgment layer (Sentinel) does **not** (it fails the
standalone-use test — nobody installs it alone).

| Install name | Internal dir(s) | Standalone CLI |
|--------------|-----------------|----------------|
| `openagent-containment` | `workloads/agent` + `infra/{proxy,egress}` | run an agent inside the perimeter |
| `openagent-skills` | `workloads/skills` *(renamed from `forge`, SD1)* | scan + CDR-rebuild skills |
| `openagent-social` | `workloads/social` | scan agent-social feeds |

- The `openagent-` prefix is a **distribution identity only** — it never appears
  on internal modules. Internal directory names stay `agent` / `skills` /
  `social` / `proxy` / `egress` (no `openagent-` prefix).
- A name ≠ its directory is acceptable; the mapping lives in a root
  `distribution.yml` (the single source for both the standalone installers and
  the GUI profiles).
- Sentinel is `sentinel` — an internal shared library, no `openagent-*` name.

### 3. A standalone-test rule for what earns a distribution name
Refines ADR-0013's lifecycle test into the rule the maintainer articulated:
**if you can download, install, and use a part standalone-usefully, it earns a
distribution name + a standalone install path; if not, it stays an unnamed
internal module.** Crucially, this is a *distribution* property, not a *repo*
property — earning a name does **not** mean earning a separate repository. The
standalone shields are named and independently installable while still living in
the one monorepo.

## Consequences

### Positive
- **Adoption surface widens.** A technical user installs `openagent-skills`
  alone and gets a lean CLI scanner — the "small focused tool" adoption path,
  without the GUI or the other four containers.
- **The GUI becomes honestly optional.** Non-technical users get the visual
  layer with a profile; it is never a prerequisite for using a shield.
- **No reversal cost.** ADR-0013's dev ergonomics stay; no submodules return; no
  cross-repo coordination tax. The modularity is recovered at the layer where it
  was actually missing.
- **Independent release boundary, already granular.** CI already tags each
  container image independently in GHCR; standalone installs pull the per-image
  tag, so a shield ships a fix without rebuilding the GUI.
- **The naming fight resolves cleanly.** Distribution names where standalone use
  exists; internal modules elsewhere; Sentinel unnamed.

### Negative
- **A distribution layer to build + maintain:** `distribution.yml`, the
  profile-driven `build.rs`, bootstrap profile selection, per-tool installers +
  CLI wrappers, per-tool landing/docs. (Spec'd in `05-modular-distribution.md`.)
- **Two binding levels for Sentinel.** Because shields install standalone,
  Sentinel must be a shared library callable from a bare CLI, with the GUI as a
  consumer — not a GUI-only service (see [`docs/specs/v0.6/01-sentinel-spine.md`](../specs/v0.6/01-sentinel-spine.md) §5).
- **A name-vs-directory indirection** (`openagent-skills` ↔ `workloads/forge`).
  Mitigated by `distribution.yml` as the single source.

### Risks accepted
- **The standalone-test rule could be over-applied** to re-extract tools into
  separate repos later. Guard: this ADR explicitly scopes the rule to
  *distribution naming + install paths*, not repository topology. Re-extraction
  to a separate repo remains governed by ADR-0013's "only if a real second
  consumer appears, and then via pinned OCI/package, never a submodule."

## Alternatives considered and rejected

- **Re-extract to separate repos.** Maximum separation + independent OSS
  communities, but re-introduces the multi-repo coordination cost ADR-0013
  removed, for a benefit (independent contributor communities) not yet in
  evidence. Rejected; the standalone-install need is met by modular distribution
  without it.
- **Leave distribution monolithic, market the tools as "capabilities" inside one
  app.** Cheapest, but does not deliver the "install one lean tool" experience
  that is the actual product differentiator. Rejected.
- **A `vault-sentinel` container as the shared judge.** Would break the
  standalone-CLI path (a CLI tool can't depend on a running container to judge a
  line) and add a sixth container. Rejected in favour of lib-first Sentinel.

## Sub-decisions (resolved 2026-05-31)
- **SD1 — RESOLVED:** rename `workloads/forge` → `workloads/skills` (and
  `vault-forge` → `vault-skills`, id `forge` → `skills`) for 1:1 consistency
  (dir = container = id = install root `openagent-skills`). "Cleanroom" remains
  the CDR capability name.
- **SD2 — RESOLVED:** `openagent-containment` (not `-runtime`).

## Cross-references
- [ADR-0013](0013-monorepo-consolidation.md) — the monorepo this extends.
- [`docs/specs/v0.6/05-modular-distribution.md`](../specs/v0.6/05-modular-distribution.md) — the implementation spec.
- [`docs/specs/v0.6/00-index.md`](../specs/v0.6/00-index.md) — the v0.6 spec index (D7/D8 record these decisions).
- A later ADR (suggested ADR-0015) should record the Sentinel judgment-layer
  decision once the spine lands.
