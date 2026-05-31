# Modular distribution — Pillar B (spec)

> Part of [OpenTrApp v0.6](00-index.md). Pairs with [ADR-0014](../../adr/0014-monorepo-modular-distribution.md).
> Resolves the "giant bloated app — install all to use 1/5th" problem **without
> reverting the monorepo** (ADR-0013 stays).

---

## 1. The problem this solves

A 2026-05-31 audit found the three tools are **already modular at the code
level** — each runs standalone via its own Makefile CLI with zero cross-tool
imports, and the manifest-driven GUI renders a dashboard for whatever tools are
present. **The only thing monolithic is distribution:** `app/src-tauri/build.rs`
bundles all three manifests + all five container images into one AppImage, and
the bootstrap brings up all five containers. There's no way to install just one
tool.

Pillar B adds the missing distribution layer so:
- a **tech-savvy user** installs *one* lean shield and drives it via CLI;
- a **non-technical user** installs the GUI with a *profile* (only the tools
  they want);
- nobody installs five containers to use one.

This is the "one repo, many independently-installable artifacts" pattern (Babel,
Cargo, Next.js). Modularity lives at the **distribution** layer, not the repo
layer.

## 2. Naming canon (`openagent-*` family)

The three **standalone-installable shields** carry distribution/marketing
names. **Sentinel does not** — it fails the standalone-use test (it only judges
fragments for the shields; nobody installs it alone). The `openagent-` prefix is
a *distribution* identity, never an internal-module prefix.

| Install name | Internal dir(s) | Container(s) | Standalone CLI |
|--------------|-----------------|--------------|----------------|
| `openagent-containment` | `workloads/agent` + `infra/proxy` + `infra/egress` | vault-agent, vault-proxy, vault-egress | run any CLI agent inside the perimeter |
| `openagent-skills` | `workloads/skills` *(renamed from `forge`)* | vault-skills *(renamed from `vault-forge`)* | scan + CDR-rebuild skills |
| `openagent-social` | `workloads/social` | vault-social | scan agent-social feeds |
| *(internal, no install name)* | `app/src-tauri/src/sentinel/` + the shared lib | — | the shared judge |

**Rules:**
- **Internal directory names stay** `agent` / `skills` / `social` / `proxy` /
  `egress` — no re-churn after ADR-0013. Distribution-name → internal mapping
  lives in the distribution manifest (§4); a name ≠ its dir is fine.
- **No `openagent-` prefix inside the monorepo.** `workloads/openagent-skills`
  would be double-namespaced. The prefix appears only on the installable
  artifact + its docs/landing.
- The family identity (these are open-agent safety tools) is expressed by the
  prefix on the artifacts + the README, not by the directory tree.

### Sub-decisions (resolved 2026-05-31)
- **SD1 — RESOLVED: rename to `skills`.** `workloads/forge` → `workloads/skills`,
  container `vault-forge` → `vault-skills`, component id `forge` → `skills`,
  install name `openagent-skills`. Full 1:1 consistency (dir = container = id =
  install root). "Cleanroom" remains the capability name for the CDR pipeline.
  **Implementation sweep** — see [`06-naming-consistency-sweep.md`](06-naming-consistency-sweep.md)
  for the full file list, order, and tests. (The v0.6 spec docs have already
  been swept: dir `forge`→`skills` in the canon, file `03-cleanroom-skills.md`.)
- **SD2 — RESOLVED: `openagent-containment`.** "Runtime" undersells that it's a
  three-container fence; the product is about containment.

## 3. The three distribution modes

### Mode 1 — standalone CLI per shield (tech-savvy)
Install one shield + its container image(s) + a thin CLI wrapper. No GUI, no
other shields, no five-container AppImage.

```
# illustrative — exact mechanism in §5
curl -sSL https://opentrapp.com/install/skills | sh
skills scan ./my-skill/SKILL.md
skills cdr ./my-skill/
```

- `openagent-skills` → a `skills` CLI over `workloads/skills`'s Makefile targets.
- `openagent-social` → a `social` CLI over `workloads/social`.
- `openagent-containment` → installs the agent+proxy+egress trio + the perimeter
  compose subset; the CLI brings the fence up/down around the user's agent.

### Mode 2 — GUI with install-profiles (non-technical)
The desktop app installs a **profile** — only the chosen tools' containers +
manifests. The GUI renders only the present tools' dashboards (already works —
the manifest discovery returns only what's bundled).

| Profile | Containers brought up |
|---------|----------------------|
| `containment` (default/minimum) | vault-agent, vault-proxy, vault-egress |
| `containment+skills` | + vault-skills |
| `containment+social` | + vault-social |
| `all` | all five |

Profile is chosen at install (or changed later in Preferences → re-runs the
bootstrap for the new set).

### Mode 3 — monorepo (developers)
Clone, edit, build — unchanged. The dev experience ADR-0013 optimized for.

## 4. What to build

### 4a. A distribution manifest
A single source of truth mapping install-names → internal dirs → containers →
CLI entrypoint. Drives both the standalone installers and the GUI profiles, so
the mapping isn't duplicated.

```yaml
# distribution.yml (new, repo root)
shields:
  openagent-containment:
    dirs: [workloads/agent, infra/proxy, infra/egress]
    containers: [vault-agent, vault-proxy, vault-egress]
    cli: containment
    standalone: true
  openagent-skills:
    dirs: [workloads/skills]
    containers: [vault-skills]
    cli: skills
    standalone: true
  openagent-social:
    dirs: [workloads/social]
    containers: [vault-social]
    cli: social
    standalone: true
profiles:
  containment:        [openagent-containment]
  containment+skills: [openagent-containment, openagent-skills]
  containment+social: [openagent-containment, openagent-social]
  all:                [openagent-containment, openagent-skills, openagent-social]
```

### 4b. Decouple `build.rs`
Today `app/src-tauri/build.rs` (lines ~18–32) hardcodes `STAGED_MANIFESTS =
["agent", "forge", "social"]` and stages all of them. Make the staged set
**profile-driven** (env var / cargo feature read from `distribution.yml`), so a
`containment` build bundles only the agent manifest + the three containment
images. Default profile = `all` (preserves today's behaviour).

### 4c. Bootstrap profiles
The bring-up set in `app/src-tauri/src/bootstrap/mod.rs` (`SHELL_SERVICES`, which
this session edited) becomes the **profile's** container set, not the hardcoded
four. `containment` brings up egress+proxy+agent only; skills/social start only
if their profile includes them. The existing single-flight guard +
idempotency (Zone 2) carry over.

### 4d. Per-tool standalone installers + CLI wrappers
For each `standalone: true` shield: an install script that (1) fetches the
tool's dir (from a release tarball or a thin git sparse-checkout), (2) pulls its
GHCR image by digest (CI already tags per-image), (3) drops a CLI wrapper on
PATH that shells to the tool's Makefile/tools. The tools are bash/python, so a
script + image is the lean fit — no packaging runtime needed.

### 4e. Per-tool landing/docs
Each shield gets a crisp standalone README + a landing section so it's
discoverable as a lean tool, not buried. `openagent-skills` already has
`skills-spotlight.md` + `workloads/skills/README.md` — replicate the shape for
containment + social.

### 4f. Independent release boundary
Standalone installs pull the per-image GHCR tag, so a shield ships a fix as a
new image tag **without rebuilding the GUI**. Document the per-shield version in
each shield's README; the GUI's "about" lists the bundled shield versions.

## 5. Interfaces to existing code

| Existing | Change |
|----------|--------|
| `app/src-tauri/build.rs` | staged manifest/image set becomes profile-driven (read `distribution.yml`) |
| `app/src-tauri/src/bootstrap/mod.rs` | bring-up set = the profile's containers, not hardcoded |
| `compose.yml` | unchanged (defines all five); profiles select a subset to start |
| CI (`.github/workflows/ci.yml`) | already tags per-image in GHCR; add per-shield standalone-install artifact publishing |
| `workloads/*/Makefile` | the standalone CLI wrappers shell to these (unchanged) |
| GUI manifest discovery (`discovery.rs`) | unchanged — already renders only present manifests |

## 6. Tests (pre-build / TDD)

- **Profile bring-up:** `containment` profile starts exactly agent+proxy+egress;
  skills/social are not started. Assert the container set.
- **GUI renders the profile:** with only the agent manifest bundled, the GUI
  shows only the containment dashboard, no skills/social tiles, no errors.
- **Standalone runs without the parent:** the `skills` CLI wrapper scans a
  fixture with no GUI/Rust-app present (only `workloads/skills` + its image +
  local Ollama for Sentinel rungs).
- **distribution.yml is the single source:** a check (orchestrator-check.sh §17,
  new) that every profile references defined shields, every shield's `dirs`
  exist, every `containers` entry is in `compose.yml`, and the install-names
  match the `openagent-*` canon.
- **Default profile = all:** a build with no profile set bundles all three
  (no behaviour change for existing users).

## 7. Done-when

- A user can install `openagent-skills` alone and scan a skill via CLI with no
  GUI; a user can install the GUI with a `containment` profile and see only the
  containment dashboard; the monorepo dev build still produces the full app; and
  `distribution.yml` is the single source mapping names → dirs → containers →
  CLI. ADR-0014 records the decision.

## 8. Relationship to Sentinel (Pillar A)

Because shields install standalone (no parent app), Sentinel must be a **shared
library** each shield embeds — not a GUI-only service. The standalone
`openagent-skills` CLI calls Sentinel's rung-0/1/2 helpers against local Ollama
directly (the way `forge/tools/lib/cdr-intent.sh` already does). See the
lib-first refinement in [`01-sentinel-spine.md`](01-sentinel-spine.md) §5.
