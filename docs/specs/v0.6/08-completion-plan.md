# OpenTrApp v0.6 — Completion plan (spec C)

> **Status:** ✅ **RELEASED as v0.6.0 (2026-06-02)** — all four items shipped
> (B `cbd2b9f` · A `665da53` · C `96d99a4` · D1 `8450257`) + release bump/CI-green
> (`e624c2c`/`7ff6cae`); published GitHub release. The remaining work is the operator
> queue (D2 pre-release, D3/Zone-6b dogfood) — see [`docs/handoff.md`](../../handoff.md).
> Plan authored 2026-06-01.
> **For:** the agent(s) finishing v0.6. Read **after** [`00-index.md`](00-index.md)
> and [`07-implementation-roadmap.md`](07-implementation-roadmap.md).
> **Scope:** harmonise + sequence everything remaining to ship **`v0.6.0`**, now
> that M0–M4 + rung-1 + the GUI Sentinel bridge/indicator + persona-drift + the
> disarm-diff display have landed. No code was written authoring this.

---

## 1. Current state — what already landed (do NOT rebuild)

All on `main`, pushed, gated green (orchestrator-check **89/0**, cargo **96/0**,
tsc clean, vitest **82/82**, playwright **25/25**, plus the bash suites).

| Area | Landed | Where |
|------|--------|-------|
| M0 rename `forge → skills` | ✅ | repo-wide |
| M1 Sentinel judge lib + CDR retry-repair + disarm diff | ✅ | `sentinel/judge.sh`, `workloads/skills/` |
| M2 modular distribution + per-profile image bundling | ✅ | `distribution.yml`, `build.rs` |
| M3 adaptive containment — egress advisor (never-auto-loosen) | ✅ | `sentinel/egress-advisor.sh` |
| M4 adapter abstraction + semantic firewall (incoming) | ✅ | `workloads/social/` |
| **Rung-1 embeddings** (D2 → `all-minilm`) | ✅ | `sentinel/embed.sh`, `sentinel/corpus/` |
| **GUI Sentinel bridge + activity indicator** | ✅ | `app/src-tauri/src/commands/sentinel.rs`, Security page |
| **Persona-drift outgoing guard** (M4 §2c) | ✅ | `workloads/social/tools/persona-guard.sh` |
| **Disarm-diff display** (skills trust artifact, GUI slice 2a) | ✅ | `workloads/skills/tools/disarm-report.sh`, `CleanedSkillsCard` |
| ADR-0015 (Sentinel judgment layer) | ✅ | `docs/adr/0015-*` |

## 2. Remaining work (the inventory)

| # | Item | Kind | Status |
|---|------|------|--------|
| **A** | Allowlist approval (GUI slice 2b) | new **write/loosening** surface | ✅ landed `665da53` — ADR-0016, only-human-loosens, `EgressApprovalsCard` |
| **B** | Production Sentinel staging | packaging | ✅ landed `cbd2b9f` — `sentinel/` verified `:ro` resource (host + shields) |
| **C** | M4 live network adapter | integration | ✅ landed `96d99a4` — atproto adapter (Bluesky), ADR-0017, validated live |
| **D** | Closeout | small/docs | **D1** ✅ judge-as-2nd-opinion (tighten-only) + ADR-0016 (with A). D2 (pre-release) + D3 (Zone 6b) = operator queue |

## 3. The decisive principle — right channel for the component type

This resolves most "how do I wire it?" questions without taste:

- **Workloads** (`skills`, `social`, `agent`) have a `component.yml` → surface
  through the **manifest-command channel** (the generic backend runs a declared
  command; the frontend renders it). *The disarm-diff display used this.*
- **Infra** (`proxy`, `egress`) has **no** `component.yml` (discovery globs
  `workloads/*` only) → surface through the **orchestrator's container-management
  layer** (read the log volume, write the host config, signal reload). This is
  *orchestration*, not workload business logic, so it does **not** violate the
  generic-backend constraint. *The allowlist approval uses this.*

The mistake to avoid (made and corrected this session): trying to read a
container's podman volume from the host as if it were a workload artifact. Read
from where the component's data lives, through the channel that matches its type.

---

## 4. Item A — Allowlist approval (threat-modeled)

Turns the blunt off-allowlist BLOCK into an **explained, one-tap human decision**
(spec [`02`](02-adaptive-containment.md) §3b). It is the one genuinely new
**write/loosening** surface in v0.6, so it leads with a threat model.

### 4.1 The hard invariant (ADR-0002)
Only the **human** can loosen the perimeter; the **agent never can**. Everything
below is built so this holds *by construction*, not by convention.

### 4.2 Threat model (extends T1 / T5 + the self-promote invariant)

| Threat | Mitigation (by construction) |
|--------|------------------------------|
| The contained agent triggers a loosening | The agent has **no path** to the app's control plane — no network service (CLAUDE.md §10), the agent is contained. The only writer of the allowlist is the orchestrator, only on an explicit GUI tap. Write-authority = human-via-GUI, structurally. |
| A wrong/compromised judge auto-allows exfil (T1) | The judge **only recommends** — it surfaces an *allow-leaning* request as a *pending approval*; it never applies a loosening. Clear exfil is **hard-blocked at rung 0** and never reaches the judge. "Always allow" is a human tap. A fully-wrong judge still cannot loosen. |
| Attacker-controlled host string rendered in the approval UI (T1, injection) | The host/URL is treated as **data** — rendered as text, banned-vocab applies, the judge prompt is injection-hardened (it already resists "return allow"). |
| Approval fatigue — user clicks Allow without reading (T5) | Only *gray-zone* requests surface (rung 0 clear cases never prompt); the plain-language reason states *why*; the destructive-action approval delay (T5 friction layer) applies. Escalation must be rare (the D5 budget). |
| TOCTOU / empty-allowlist window on write+reload | Atomic write (temp + rename) on the host file; the proxy already does an **atomic swap** on `_reload_allowlist` (SIGHUP). No empty-set window. |

### 4.3 Architecture & data flow (the infra channel)

```
proxy logs off-allowlist BLOCKED ─▶ vault-proxy-logs volume (requests.jsonl)
                                         │  orchestrator reads it (podman volume
                                         │  mountpoint, or `podman exec cat`)
                                         ▼
  recent off-allowlist hosts ─▶ Sentinel judge (context=egress_request) ─┬ block-leaning ─▶ stays blocked (logged)
                                  [reuses sentinel_judge bridge]          └ allow-leaning ─▶ PENDING APPROVAL
                                                                                              │ (GUI, one tap)
   user taps ──┬ "Always allow" ─▶ atomic append host allowlist (~/.opentrapp/perimeter/allowlist.txt)
               │                    + `podman kill -s HUP vault-proxy`  (reload — the loosening)
               └ "Deny"          ─▶ no persist (optionally remember-deny)
```

**The allowlist relocation (prereq, shared with Item B):** today
`./infra/proxy/allowlist.txt:/opt/vault/allowlist.txt:**ro**` binds from the
source tree. Move the seed to the **runtime dir** `~/.opentrapp/perimeter/allowlist.txt`,
bind-mounted **`:ro`** into the proxy. `:ro` is *correct and load-bearing* — the
container (and thus the agent) can never write it; only the **host app** writes
the host file, then signals reload. Stage the seed at bootstrap (the ADR-0011
runtime model). Touches `compose.yml`, `bootstrap`, `build.rs`/staging.

### 4.4 New orchestrator commands (container-management layer, not a manifest)
- `list_egress_approvals() -> [PendingApproval{host, reason, judged_at}]` — read
  the log volume, filter off-allowlist, judge each via the Sentinel bridge.
- `apply_allowlist_decision(host, decision: "always"|"deny")` — `always`
  atomic-appends + reloads; `deny` no-persists. The **only** allowlist writer.
Register in `lib.rs`; live alongside the lifecycle/podman management code.

### 4.5 GUI
A pending-approvals section on the **Security page** (Allow always / Deny + the
plain-language reason). Reuses the **Sentinel activity indicator** (`thinking`
while judging). Plain-language, banned-vocab.

### 4.6 Tests (TDD)
- **Invariant pin:** no code path appends to the allowlist without the
  human-decision flag (the ADR-0002 guard — mirror egress-advisor's never-loosen pin).
- Off-allowlist gray-zone → a pending approval; **clear exfil never reaches the
  judge** (rung-0 hard-block) and never appears as approvable.
- `always` appends + triggers reload; `deny` does not persist.
- Verdict/reason vocabulary; `orchestrator-check` §pin (the infra channel wired,
  the clear-exfil path does NOT route to the judge).

### 4.7 Collision surfaces (→ Opus, sequential): `podman.rs`, the new
commands + `lib.rs`, `App.tsx`/`SecurityMonitor.tsx`, `compose.yml`, `bootstrap`,
`build.rs`. (Same shared GUI/runtime surfaces as the prior Opus slices.)

---

## 5. Item B — Production Sentinel staging

So the judge/embed bridges + the in-container bash shields work in a **packaged
build** (today they resolve only via the dev fallback / staged-runtime paths).

### 5.1 Two facets
- **Host / app (the Rust bridge):** bundle `sentinel/` as a Tauri resource
  (`tauri.conf.json` `bundle.resources`, staged like `resources/perimeter`); the
  `locate_sentinel_dir` resolver already checks `resource_dir()` first, so this
  "just works" once staged. Add `build.rs` staging of `sentinel/`.
- **In-container (the bash shields):** `semantic-firewall.sh`, `persona-guard.sh`,
  `skill-cdr.sh` already resolve the lib via a `/opt/sentinel` candidate path.
  Stage/mount `sentinel/` into `vault-skills` + `vault-social` (compose mount in
  dev; copy into the image for release). **Sentinel ships with every profile** —
  it is shared, so it is bundled regardless of the modular-distribution profile
  (harmonise with M2's per-profile bundling: profiles select *workload* images;
  the shared lib is always present).

### 5.2 Runtime requirement (document, don't bundle)
The judge/embed need **Ollama** reachable on the host (already true for CDR).
Bundling Ollama is **out of scope**; document the requirement in the release
notes + the first-run check (a soft prerequisite — the static layers work
without it; the AI rungs degrade to "escalate/hold", which the legs already
handle fail-safe).

### 5.3 Tests
- Resolution test: with a bundled `resource_dir/sentinel`, `sentinel_judge`
  locates the lib (no dev fallback).
- `orchestrator-check` pin: `sentinel/` is staged for the build + mounted into
  the shield containers.

### 5.4 Collision surfaces: `build.rs`, `tauri.conf.json`, `bootstrap`,
`compose.yml`, `podman.rs` runtime layout. **Overlaps Item A's allowlist
relocation** → same Opus stream; **do B before A** (B establishes the runtime
staging that A's allowlist-relocation extends).

---

## 6. Item C — M4 live network adapter

Per spec [`04`](04-semantic-firewall-social.md) §2a/§7: implement a live adapter
against the existing contract (`fetch_feed`/`fetch_agent`/`post`/`stats`),
validate the semantic firewall (incoming) + persona-guard (outgoing) against it,
and write the un-park ADR. **Behind a flag** until the live adapter validates —
do not re-park as a Moltbook-only revival.

- **First target (SD-C1 — RESOLVED): AT Protocol (atproto / Bluesky).** Real
  agent/bot traffic, a documented public API (the XRPC `app.bsky.feed.*`
  endpoints / firehose), and an open ecosystem. Implement
  `workloads/social/tools/lib/adapters/atproto.sh` first; keep Mastodon/Nostr as
  later adapters behind the same contract.
- **Files:** `workloads/social/tools/lib/adapters/atproto.sh` + tests. Fully
  **disjoint** from the Opus stream → parallelisable (Sonnet for the adapter
  mechanics; scouting the specific feeds/handles to test against is a quick
  human-or-agent step).

---

## 7. Item D — Closeout

| # | Task | Owner |
|---|------|-------|
| D1 | Wire the 3b judge as a **second opinion on the skills scanner's auto-allow** path (now viable — the 1.5b over-blocked). Skills-only, small. | Sonnet |
| D2 | Pre-release: re-record demo gifs against a v0.6 build; sweep the `forge→skills` rename into the gitignored `docs/pitch-opencode.md`; OpenSSF badge resubmission. | Sonnet/human |
| D3 | Zone 6b — dogfood harness ordering (gitignored `AGENT-TODO.md`). | Sonnet |
| D4 | **ADR-0016** — record the allowlist write/loosening decision, once Item A lands. | with A |

---

## 8. Harmonised sequencing (the next-session map)

```
Opus, sequential (shared runtime + GUI surfaces):
   B  Sentinel staging  ──→  A  Allowlist approval (+ ADR-0016)

Sonnet, parallel (disjoint files — must NOT touch the Opus collision set):
   C  live adapter      D1 judge-as-2nd-opinion      D2/D3 pre-release/dogfood
```

**Why this order:**
1. **B before A** — both modify the runtime-staging layer (`build.rs` /
   `bootstrap` / `podman.rs` / `compose.yml`); B lays the staging A extends, and
   A's production judge calls depend on B's staged lib.
2. **A is the only new write/loosening surface** — most security-sensitive,
   carries the threat model, gets the most careful review. Opus, never rushed.
3. **C / D run in parallel** (Sonnet) — disjoint files. **Collision set the
   Sonnet streams must avoid:** `build.rs`, `bootstrap/mod.rs`, `podman.rs`,
   `compose.yml`, `lib.rs`, `App.tsx`, `SecurityMonitor.tsx`. (D1 touches only
   `workloads/skills`; C only `workloads/social`; D2/D3 only docs/assets.)
4. **Freeze rule still holds** — no leg modifies the Sentinel lib; staging (B)
   moves it but does not change its contract.

**The gate (every milestone):** the established gate — `cargo test --lib`,
`tsc --noEmit`, `vitest`, `orchestrator-check.sh` (add a §-section per
milestone), `playwright --project=default`, `podman compose config` — plus each
item's pre-build tests authored first (TDD).

---

## 9. Decisions (RESOLVED 2026-06-01 — do not relitigate)

| # | Decision | Resolution |
|---|----------|------------|
| SD-A1 | "Allow once" vs "Always allow" only | **RESOLVED: ship "Always allow" + "Deny" only.** "Allow once" is deferred — it needs proxy one-shot/TTL support (extra L7 complexity); lean ships the two-button flow first. |
| SD-A2 | Remember a "Deny" to avoid re-prompting the same host | **RESOLVED: yes** — record a denied host so it doesn't nag; it still never auto-loosens (deny is not a write to the allowlist). |
| SD-B1 | Stage `sentinel/` into containers: bind-mount vs image copy | **RESOLVED: bind-mount in dev, copy into the image for release** (a hermetic shipped artifact). |
| SD-B2 | Bundle Ollama? | **RESOLVED: no** — out of scope; document the runtime requirement; the AI rungs degrade fail-safe without it. |
| SD-C1 | Which live agent-social network to scout first | **RESOLVED: AT Protocol (atproto / Bluesky) first.** Implement the atproto adapter + validate the firewall/persona-guard against it before any other network; keep the others as later adapters behind the same contract. |

## 10. Definition of done for v0.6.0 (updated)

The original [`07`](07-implementation-roadmap.md) §8 done-whens, plus:
- The allowlist turns blunt blocks into explained one-tap decisions and **never
  auto-loosens** (the invariant pinned); the threat-model rows are added to
  [`docs/threat-model.md`](../../threat-model.md) and ADR-0016 is written.
- The judge/embed bridges + the in-container shields work in a **packaged build**
  (Sentinel staged host + container); the Ollama requirement is documented.
- The social shield validates against **≥1 live** agent-social network (or, if
  none is reachable, ships the adapter + persona-drift behind a flag with the
  gap logged — not silently dropped).
- Pre-release: demo gifs re-recorded against `v0.6.0`; the pitch + docs swept to
  the `skills` names.

## 11. Relationship to existing docs
- Builds on: [`00-index.md`](00-index.md), [`07-implementation-roadmap.md`](07-implementation-roadmap.md).
- Allowlist invariant: [ADR-0002](../../adr/0002-adaptive-shell-levels.md); credential reuse [ADR-0001](../../adr/0001-proxy-side-api-key-injection.md).
- Sentinel: [ADR-0015](../../adr/0015-local-ai-judgment-layer.md) + [`01-sentinel-spine.md`](01-sentinel-spine.md).
- Threat model to extend: [`docs/threat-model.md`](../../threat-model.md) (T1, T5).
- Suggested new record: **ADR-0016 — host-mediated allowlist loosening** (with Item A).
