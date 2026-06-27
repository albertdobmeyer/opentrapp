# ADR-0021 — Danger-gated agentic control plane

**Status:** Accepted (2026-06-27) — the security model for the agent-operable control plane that
[ADR-0020](0020-product-identity-and-distribution.md) (tenet 5) requires; **implementation staged**
(see *Implementation status* below). This record decides the *authorization* model (what may
be done by whom, and how boundary-weakening is gated); the *transport* threat model of the loopback
server is deferred to [ADR-0022](0022-daemon-control-surface.md).
**Cross-references:** [ADR-0002](0002-adaptive-shell-levels.md) (the loosening invariant) ·
[ADR-0016](0016-host-mediated-allowlist-loosening.md) (the human-is-sole-writer pattern this
generalizes) · [ADR-0011](0011-zero-trust-self-sufficient-bootstrap.md) (self-healing staged
perimeter) · [ADR-0019](0019-headless-daemon-gui-viewer-split.md) (the daemon owns the perimeter) ·
[ADR-0020](0020-product-identity-and-distribution.md) · [threat-model.md](../threat-model.md)
(T1–T6) · [CLAUDE.md §11](../../CLAUDE.md)

---

## Context

ADR-0020 makes OpenTrApp **agent-operable**: the user's *host* agent (e.g. Claude Code) — an
**external operator**, outside the perimeter, running with the user's authority — should be able to
install, configure, run, and observe OpenTrApp. The external operator orchestrates the *internal*
(contained) agent.

The existing threat model already makes the perimeter **non-loosenable by the *contained* agent**
(T1; ADR-0002/0016): the agent has no call edge to the control plane, the *only* writer of an egress
loosening is the orchestrator on an explicit human tap, and the allowlist is read-only to the proxy
and self-healed from the signed bundle each launch (ADR-0011). That invariant is structural.

Agent-operability introduces a **new actor the existing model does not cover**: a host program we
*trust* (Claude Code), wielding the user's authority, that could be **prompt-injected** by content
it processes — the exact attack class OpenTrApp exists to blunt, now aimed one level up. If such an
injected host agent is handed a clean OpenTrApp control surface (`opentrapp allowlist add evil.com`,
`opentrapp pause`), the injection's *easiest* path to disabling containment is to call it. A security
tool that shipped a blessed, convenient "disable the security" verb for an injectable agent would be
self-defeating.

This ADR defines the authorization model that prevents that — honestly bounded against what is and
is not achievable when the operator has full host privileges.

## Threat model: T7 — Prompt-injected host operator

**Definition.** A *trusted, user-installed* host agent (Claude Code, opencode, a shell agent) that is
**prompt-injected** via content it reads (a malicious file, web page, tool output), and that has been
given — or can reach — an OpenTrApp control surface. The attacker controls the agent's *instructions*,
not the host OS. Goal: weaken or disable the perimeter protecting the contained agent.

**T7 is distinct from T4 (compromised host, out of scope).** In **T4**, the attacker controls the
host directly (rootkit, malicious binary) — game over, no container isolation can recover, explicitly
out of scope. In **T7**, the host program is *honest*; only the *content it reads* is hostile, and
its perimeter-affecting reach is (absent OpenTrApp control surfaces) only what any user-privileged
process has. T7 is addressable; T4 is not.

**The honest boundary (stated plainly, not flinched from).** A host agent runs with the user's full
privileges: it can read/write any user file, run any command, kill processes. A *fully* injected one
can therefore tamper with `~/.opentrapp` directly, kill the daemon, or edit `compose.yml` — i.e. it
can do anything T4 can do **without** OpenTrApp's control plane. **So the danger-gate does NOT claim
to stop a fully-injected, fully-privileged host agent — that residual is T4, out of scope and
inherited unchanged.**

**What the danger-gate DOES claim (the defensible, in-scope guarantee):**

> **OpenTrApp's agentic control plane is never an *amplifier*.** Adding agent-operability introduces
> **no new, easier boundary-weakening path** than the pre-existing T4 residual. Boundary-weakening
> through *any* OpenTrApp surface requires the *same* out-of-band human action whether the caller is
> a human or an agent — so the obvious, blessed weakening verb an injection payload would target is
> closed, and the only remaining path is raw host tampering (T4), which is higher-cost, more
> detectable, and not what a generic injection does.

STRIDE decomposition of T7's capabilities **via OpenTrApp surfaces** (the part we own):

| T7 capability (via an OpenTrApp surface) | STRIDE | Defense (this ADR) | Residual |
|---|---|---|---|
| Loosen the egress allowlist | E, T | No agent call edge to the allowlist writer; the sole writer is the human tap (ADR-0016), generalized to *every* surface incl. CLI/MCP | Raw file tampering = T4 (out of scope); mitigated for the *live* file by the ADR-0011 self-heal |
| Pause / stop the perimeter, or weaken the shell level toward Soft | E, D | `boundary_impact: weakening` → out-of-band human confirmation; not agent-auto-satisfiable | T4 (kill the daemon directly) |
| Change egress/proxy policy or the proxy CA | T, E | Same: weakening writer, human-gated, no agent edge | T4 |
| Read status / logs / run a scan / restart a container in-policy | (none — boundary-neutral) | Permitted to agents (its own operational `danger` UX still applies) | — |

## Decision

A **danger-gated control plane** built on three rules.

### 1. `boundary_impact` — a security axis distinct from operational `danger`

The manifest `danger` field (`safe | caution | destructive`) measures **operational** risk (data
loss, disruption) — it is necessary but **not** a boundary-security classification. A boundary
weakener (`add host to allowlist`) may be merely `caution`; a `destructive` op (`delete a quarantined
skill`) may not weaken the perimeter at all. So every control-plane operation additionally carries a
**`boundary_impact`**:

- **`neutral`** — operates *within* the perimeter; does not reduce its protection (status, logs,
  scans, in-policy restarts, the wizard's *validate* steps).
- **`weakening`** — reduces the perimeter's protection (loosen egress allowlist; pause/stop the
  boundary; loosen the shell level; edit egress/proxy/CA policy; disable a layer).

This is a security-review classification on the operation, defaulting to `weakening` when in doubt
(fail-closed), validated in CI like the schema-alignment checks.

### 2. The gate: neutral is agent-operable; weakening requires an out-of-band human confirmation

- `boundary_impact: neutral` → **agent-operable** (subject only to its own operational `danger`
  confirmation UX). Humans, Claude Code, opencode all drive these freely. This is the "agentic" win.
- `boundary_impact: weakening` → **never auto-applied from any agent-reachable path.** It requires a
  human confirmation **on a channel the (possibly injected) host agent does not control** — the GUI
  approval tap (ADR-0016's two-tap), or a confirmation pushed to the user's phone (Telegram).
  An agent may *request/recommend* a weakening (the Sentinel judge may explain the trade-off,
  ADR-0015); a human *decides*.

> **The out-of-band rule is load-bearing.** A *same-channel* "are you sure? type yes" prompt is
> **worthless** here: an agent driving the CLI/MCP just types "yes." The confirmation must occur on a
> surface the requesting agent cannot actuate on the human's behalf (the GUI tap, the phone). This is
> the single most important property in this ADR.

### 3. Gate the operation, not the caller — enforced structurally

Distinguishing "a human typed it" from "an injected agent typed it" at the CLI/MCP boundary is
**unreliable** (the agent presents as the user). So we do **not** authenticate caller-as-human.
Instead, exactly as ADR-0016:

- **Boundary-weakening writers have NO call edge from any agent-reachable transport** (CLI, MCP,
  loopback API). They are reachable *only* from the human approval surface. The control plane is, by
  construction, incapable of applying a weakening — a fully-wrong agent (or a fully-wrong judge) still
  cannot loosen anything through it.
- Generalizes the proven ADR-0016 invariant ("`apply_allowlist_decision` is the sole writer, the
  agent has no call edge") from the *contained* agent to **every** operator and **every** surface.
- Defense in depth with ADR-0011: the live perimeter config self-heals from the signed bundle each
  launch, so direct tampering with the *live* files is partially undone.

## Consequences

**Positive**
- Agent-operability **and** a defensible security posture: the agentic feature adds no new
  boundary-weakening path (the in-scope guarantee), so we can be "agent-native" without being
  "disarmable."
- Extends an already-proven, test-pinned invariant (ADR-0016) rather than inventing a new mechanism.
- The `neutral`/`weakening` split gives a crisp, auditable rule for every future control-plane
  operation and every new surface (CLI/MCP/loopback inherit it for free).

**Negative / cost (honest)**
- **The T4 residual is real and must be documented, not hidden:** a fully-injected, fully-privileged
  host agent can bypass OpenTrApp entirely (raw file/process tampering). The gate is *amplifier
  prevention + cost-raising + ADR-0011 self-heal*, not absolute prevention. We say so. Mitigation we
  *recommend* (not mandate): run the external control agent itself with least privilege where the
  host allows.
- Legitimate human weakening now always costs an out-of-band tap — friction by design (ADR-0016's
  approval-fatigue residual, T5, applies).
- Every control-plane operation must carry a correct `boundary_impact`; a mis-tag that marks a
  weakener `neutral` is a real vulnerability, so the classification is fail-closed + CI-checked.

## Alternatives considered

- **Trust the host agent fully (no gate).** Rejected — a prompt-injected host agent is a realistic,
  in-scope threat (it is the whole premise of the product, one level up); shipping a blessed weakening
  verb for it is self-defeating.
- **Same-channel confirmation ("type yes").** Rejected — the requesting agent controls that channel
  and satisfies the prompt itself. Out-of-band is mandatory.
- **Authenticate human-vs-agent at the CLI.** Rejected — unreliable; the agent presents as the user.
  Gate the *operation*, not the caller.
- **No agentic control plane at all.** Rejected — agent-operability is the ADR-0020 mission; the gate
  is precisely what makes it safe, so removing the feature is unnecessary over-correction.

## Implementation status

Staged, test-first. The full **amplifier-prevention guarantee** (§ Decision) is delivered by Slice 2.

- **Already in place (the ADR-0016 invariant this generalizes).** The loopback HTTP surface deliberately
  mounts *no* boundary-weakening op (`viewer-server/src/routes.rs`; pinned by the orchestrator-check §6
  `DAEMON_ONLY` route-absence contract), and `allowlist::apply_always` — the sole allowlist writer — has
  no agent call edge (pinned by `denial_never_writes_the_allowlist` + orchestrator-check §27).
- **Slice 1 — the classification primitive (shipped 2026-06-27).** `core::boundary::BoundaryImpact`
  (`neutral` | `weakening`), **fail-closed** default `weakening` (an unclassified op is never
  agent-operable), and `agent_operable()` (true only for `neutral`, ADR §2). The control-channel verbs
  are tagged: `ControlRequest::boundary_impact()` → `Pause`/`Shutdown` = `weakening` (they leave the
  perimeter *down*), `Resume`/`Restart` = `neutral` (they return it to full, re-verified protection).
  TDD + mutation-pinned. No behaviour change yet — this is the data the gate consumes.
- **Slice 2 — the gate + the held-approval backend (shipped 2026-06-27).** At the supervisor
  control-dispatch chokepoint, `gate_inbox_request` reads each request's `boundary_impact`: a
  `weakening` one (from the agent-writable `~/.opentrapp/control` inbox) is **held** in the
  `core::approvals` queue — never applied here; only a `neutral` one (Resume/Restart) is applied. The
  held request is applied only by `supervisor::apply_approved` (the generalization of
  `allowlist::apply_always`: the sole pending→applied edge, with **no agent call edge** — `user_pause`,
  the old ungated inbox applier, was removed). The daemon's *own* idle-auto-pause (`arm_pause`) is not
  an inbox request and is untouched; the daemon still stops on SIGTERM (the un-amplified T4 escape).
  Pinned by `inbox_holds_weakening_and_admits_neutral`, the `approvals` queue tests, and an
  orchestrator-check structural pin (parity with the ADR-0016 §27 sole-writer pin). Why "held," not
  "refused": holding is the no-compromise path — the op is queued for the human, not lost.
- **Slice 2b — the human approval surface (shipped 2026-06-27).** The viewer-server (already running on
  the ADR-0022 §3 transport — loopback-only, Host/Origin allowlist, 256-bit bearer, 0600 session,
  nonce→bearer) mounts `POST /api/list_pending_approvals` + `POST /api/approve_weakening` (the latter
  calls `apply_approved` — Pause applies cross-process via the shared markers + podman; a daemon stop
  stays SIGTERM/quit). The `WeakeningApprovalsCard` two-tap (assistant-first copy, no dev vocabulary)
  surfaces it on the Security page. The §6 route contract is reconciled: the daemon-only *direct-apply*
  ops stay unmounted, while the *human-approval* `approve_weakening` route is asserted present (else held
  requests would be unapprovable). The Telegram **reply** path is ruled out: the waker is peek-only by
  hard invariant (ADR-0018, the agent owns that bot's `getUpdates` when the perimeter is up), so the
  reply can't be read without breaking exactly-once delivery — the GUI two-tap is the out-of-band
  surface, exactly as ADR-0016. **Residual (honest, = ADR-0021's accepted T4):** a same-UID injected
  host agent could read the 0600 session + craft an authenticated POST to the *on-demand* (usually-not-
  running) server — strictly higher-cost than dropping a `.req`, so the "no easier path than T4"
  guarantee holds.
- **Slice 3 — the manifest `boundary_impact` field** across the three schema-alignment layers + its CI
  enum check, for the GUI-projected command surface.

## What this ADR does NOT decide

- The **loopback-server / transport** threat model (DNS-rebinding, token handoff, Origin/Host, the
  ephemeral on-demand server) → **ADR-0022**. This ADR is authorization, not transport.
- The concrete **`boundary_impact` tag on each existing operation**, the schema/CI wiring, and the
  out-of-band confirmation UX → implementation, gated on this ADR's acceptance.
- The **MCP adapter** shape → ADR-0022; whatever it is, it inherits rules 1–3 unchanged.
