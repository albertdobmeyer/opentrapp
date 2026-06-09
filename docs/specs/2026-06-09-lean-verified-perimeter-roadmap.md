# Roadmap — a lean *and* verifiably-correct perimeter

*Created 2026-06-09, out of the footprint/device-usability investigation
(`docs/footprint-and-device-usability.md`) and the co-architect review that followed.
This roadmap is gated by `CLAUDE.md` §11 (verification discipline): every dependent step
starts only after the verification it rests on is green, and acceptance criteria are
stated at the end that **consumes** the output.*

## The organizing principle

Two sentences carry the whole plan:

1. **The part we can optimize is the part we can't compromise.** ~75% of resting RAM is
   OpenClaw's Node runtime (not ours to cut); the ~25% that is ours — the always-on
   mitmproxy + egress container (the L7/L3 split, [ADR-0009](../adr/0009-five-container-perimeter.md))
   — *is* the product. So we make it leaner without ever weakening it; the real lever is
   temporal (pause it when idle), not architectural (gut it).
2. **The temporal lever is only an asset if it's verified.** Idle auto-pause is the answer
   to footprint *only if* it actually fires and resumes **security-correct** — both
   currently unverified. So verification gates the footprint story, not the other way
   round.

## The concerns this roadmap tackles (nothing dropped)

| # | Concern | Workstream |
|---|---|---|
| C1 | Idle auto-pause may never *fire* — proxy-log mtime never goes stale if OpenClaw long-polls forever, or the log doesn't persist (ZONE 3) | **WS0** |
| C2 | Resume may be "alive but security-wrong" — no boundary self-test before reporting healthy | **WS0** |
| C3 | Proxy memory growth — unbounded background RSS undermines the "silent" claim | **WS1** |
| C4 | The 75/25 split is non-stationary — false by end of a long session | **WS1** |
| C5 | `vault-egress` carries an unused Node runtime — attack surface in the privileged container | **WS2** |
| C6 | v0.7.0 publish decision rests on idle auto-pause being default-ON, which is unverified | **WS3 (blocked)** |
| C7 | The exactly-once + boundary-correct wake test (`@opentrappbot`) is unfinished | **WS0** |
| C8 | ZONE 3 proxy-log persistence — gates whether C1's idle signal exists at all | **WS0** |
| C9 | Verification-gating discipline must be codified, not session-local | **WS4** |

## WS0 — Prove idle auto-pause is real *(the gate; everything else waits on it)*

**Goal:** demonstrate, on capable hardware, that the perimeter (a) drops to dormant when
idle, and (b) resumes **security-correct**, not merely alive.

- **0a — Does it fire? (C1, C8)** Stand the full perimeter up on a machine with RAM
  headroom. Confirm two things the dev box can't show: (i) the proxy request log persists
  to its `vault-proxy-logs` volume (ZONE 3 — if it falls back to in-container `/tmp`,
  `read_egress_log_last_activity_ms()` returns `None` and auto-pause silently never fires);
  (ii) OpenClaw's Telegram long-poll cadence actually lets the log mtime go stale past the
  idle threshold (if it polls forever, the signal never trips). If (ii) fails, the idle
  signal must change from "any egress" to "non-poll egress."
- **0b — Define + enforce "security-correct resume" (C2).** Add a post-resume boundary
  self-test, run **before** the perimeter is reported healthy; any failure holds
  fail-closed and alerts. Draft contract (refine in [ADR-0018](../adr/0018-idle-auto-pause-host-waker.md)):
  1. From the agent's network namespace, a direct connection to a non-allowlisted IP **fails**.
  2. A canary request to an allowlisted host reaches the upstream **with the key injected**.
  3. A request to a non-allowlisted domain is **blocked by the proxy**.
  4. The proxy CA presented to the agent is **identical** to the pre-dormancy CA.
  5. `vault-egress` **drops** RFC1918 + non-allowlisted destinations.
  Plus the ordering invariant: the agent container has **no network path until proxy +
  egress report healthy** (a cold-start race here = a transient bypass window). This is the
  real blast-radius control — more than the five checks above. **Test it as an actual
  network-deny during the wake window**, not as a code-level "proxy starts before agent"
  sequence assertion: prove the agent container *literally cannot egress* until the
  health-check passes. Startup order in code is not evidence the agent had no network path
  during the transition.
- **0c — Upgrade the wake test (C7).** The `@opentrappbot` live test asserts the boundary
  self-test **and** exactly-once delivery (peek is non-destructive → agent consumes from
  its own offset once), not just "a reply arrived."

**Hardware:** 0a/0c need a live perimeter → **capable machine** (this 7.2 GB box
swap-storms). 0b is mostly code + the self-test harness, authorable here.

**Done when:** on capable hardware, idle → dormant fires within the threshold; a message
wakes it; the resume self-test passes all five invariants + ordering; delivery is
exactly-once; cold-start latency is measured and recorded. *(Consumption-end criteria: the
boundary actually blocks/injects post-resume — not "containers are up".)*

## WS1 — Tame the discretionary footprint (the proxy) *(measure before fixing)*

**Goal:** make the one always-on component we own (mitmproxy) bounded over a long session,
and replace the static 75/25 with a time-aware model.

- **1a — Attribute the growth (C3).** Static pass already done: our addon is clean (the
  allowlist is a `set`, wholesale-replaced; the hooks retain nothing per-request), and we
  run headless `mitmdump` (not the flow-retaining `mitmweb`), with **no
  `stream_large_bodies`**. So the live measurement targets the remaining candidate:
  proxy-only RSS over (load × time), driving streaming (SSE) Anthropic-shaped responses,
  to see whether growth is large-body buffering. **One measurement resolves both C3 (the
  fix) and C4 (the time-axis).**
- **1b — Fix per evidence (C3).** Apply the fix the measurement *selects* — likely
  streaming / `stream_large_bodies` for large bodies, **not** flow-eviction (the symptom I
  must not assume). Re-measure to confirm RSS is bounded over an 8 h session.
  - **Precondition (WS1's "don't silently weaken the boundary"):** `stream_large_bodies`
    stops mitmproxy buffering the full response body, so the addon can no longer inspect
    *complete response bodies*. This fix is permissible **only** if our security functions
    act exclusively on request headers + host (key injection, host-based allowlist) and
    never on response bodies — believed true, but **confirm it explicitly before applying,
    not as an assumption**. If any security function needs the full response body,
    `stream_large_bodies` is off the table and 1b re-selects from the measurement.
- **1c — Reframe the split (C4).** Replace the single 75/25 figure in
  `docs/footprint-and-device-usability.md` with the 2-D model over (agent activity ×
  session age); call out the worst corner (idle agent, aged proxy ≈ 75% *ours*), which is
  the "silent background all day" scenario.

**Hardware:** 1a/1b's live RSS run needs a proxy-only container under load — feasible in
isolation on a machine with headroom (it does **not** need the full perimeter).

**Done when:** proxy resting RSS is bounded (no unbounded climb) across a long session,
confirmed by measurement; the footprint doc states the time-dependent split.

## WS2 — Harden + slim the boundary *(low priority, opportunistic)*

- **2a — De-Node `vault-egress` (C5).** Move egress to a Node-less pinned base
  (alpine/distroless). Framed as **attack-surface reduction**, not disk tidiness: egress is
  the one container with `NET_ADMIN` + internet, so an unused interpreter there is the
  highest-leverage post-compromise uplift to remove. Cost: one more pinned base digest in
  the cosign/SLSA chain — document it. Do when next touching egress.

**Done when:** `infra/egress/` ships no interpreter it doesn't run; the new base digest is
pinned and verified.

## WS3 — Publish v0.7.0 *(BLOCKED on WS0)*

The release is staged as a **draft** (built, signed, cosign-verified). It must not publish
until the footprint narrative it implies is true.

- **Gate:** WS0 green — idle auto-pause both *fires* and *resumes security-correct*.
- **Narrative scope (gate the claim, not the workstream).** WS1 does **not** block this
  publish — over-gating would contradict the fail-OFF logic (don't hold a finished,
  shippable thing hostage to an unfinished optimization). But WS1 bounds what the release
  may *claim*. The v0.7.x copy may rest the footprint story on the **temporal lever** (idle
  auto-pause recycles the whole perimeter, proxy included → covered by WS0) and publish
  freely. It may **not** assert the always-on proxy as a standalone *bounded / lean /
  well-behaved* property until WS1 has landed and measured it — that claim is WS1's to
  verify. Scope the assertion to what's verified.
- **Version rule:** if WS0/WS1 land code changes (resume self-test, proxy streaming fix),
  they ride a **v0.7.1** (new tagged build); if WS0 verifies the **existing** v0.7.0 binary
  with no code change, publish the **v0.7.0 draft as-is**. Do not bump for its own sake.
- **Fallback:** if WS0 shows the wake path is unreliable and can't be fixed quickly, ship
  with idle auto-pause **default-OFF** (one-line change) rather than block the first-run
  fix + footprint wins that are already done and verified.

**Done when:** the published release's headline claims are all verified, not asserted.

## WS4 — Codify the discipline *(cross-cutting, mostly done)*

- ✅ `CLAUDE.md` §11 — verification discipline (consumption-end, gating, fail-closed
  boundaries).
- ✅ Memory: `verify-the-consumption-end`.
- ☐ Fold the WS0-0b resume contract into [ADR-0018](../adr/0018-idle-auto-pause-host-waker.md)
  as the authoritative "security-correct resume" definition.

## Dependency graph

```
WS0 (fires? + security-correct resume) ──gates──▶ WS3 (publish)
   │                                                  ▲
   └── 0b resume contract ──▶ WS4 (ADR-0018)          │
WS1 (1a measure ──gates──▶ 1b fix ──▶ 1c reframe doc) ─┘  (a code fix → v0.7.1 path)
WS2 (egress de-Node) ── independent, low priority, opportunistic
```

## Hardware reality — this laptop is the benchmark, not the blocker

**Reframe (2026-06-09, operator direction):** the 7.2 GB / 2017-APU dev laptop is not "too
small to test on" — it is the **pass/fail oracle**. If the perimeter runs smoothly *here*,
it runs anywhere. So the optimization target is concrete and the CONSTITUTION guardrail
becomes the success metric: **the resting perimeter runs on this box with swap < 500 MB.**

Most of this roadmap is doable here with memory discipline. The resting perimeter is
~0.4–0.5 GB (**measured**: idle agent 197 MB + proxy ~150 + egress ~50), which fits on this
box once the co-tenants are closed — the earlier swap-storm was Cursor (~1.4 GB) + Brave +
Claude, **not** the perimeter. Linux test protocol:

1. Close Cursor + Brave; `ollama stop` any loaded model. Verify `free -h` shows **> 3 GB
   free** and swap reclaiming.
2. Bring the resting perimeter up **incrementally** (egress → proxy → agent), running
   `make profile-memory` after each, so a surprise is caught at one container, not five.
3. **Continuous swap-watch**; tear down immediately if swap crosses 500 MB.
4. Measure the real resting footprint (confirm the ~0.4–0.5 GB estimate), then run WS0-0a
   (idle ~12 min → dormant) and WS1-1a (proxy-only RSS — the lightest, do it first).

The **fallback** for genuinely heavier runs (full five-container under sustained
active-agent load; the long-soak proxy-growth curve) is a capable machine —
`docs/perimeter-test-handoff.md` is the self-contained runbook. But the default is: **do it
here, and let this box's limits be the benchmark.** Until a given item is verified (here or
on capable hardware), it is **unverified, not done**, and WS3 stays blocked.

## Done-when (whole roadmap)

Idle auto-pause is proven to fire and to resume security-correct (WS0); the always-on proxy
is bounded over a long session and the footprint doc tells the time-true story (WS1); the
privileged boundary container carries no dead interpreter (WS2); v0.7.x publishes with every
headline claim verified at its consumption end (WS3); and the discipline that got us here is
written down, not remembered (WS4).
