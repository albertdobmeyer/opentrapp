# Road to a publicly-recommendable security tool — execution checklist

The goal: be able to recommend OpenTrApp **publicly, as an official security layer
for open agent systems** (OpenClaw and peers) — honestly, without overclaiming.

**The premise (CLAUDE.md §11):** the architecture is built and CI-green, but a
*security* tool's protective claims must be **verified at the consumption end on
real hardware**, not asserted from a green build. Most Tier-1 items below are
gated on **capable hardware** that can run the full five-container perimeter under
load — *not* the 7.2 GB dev laptop (it swap-storms). Run those on the Windows box
or a cloud VM.

**Status key:** ✅ done · 🔶 partial / implemented-but-unverified · ⬜ not started
· 🖥️ needs capable hardware · 👤 needs a human / external party

Work top-to-bottom. Tier 1 is load-bearing — do **not** make the public
recommendation until every Tier-1 box is checked. Tier 2/3 strengthen the claim
and can overlap.

---

## TIER 1 — Load-bearing (cannot recommend without these)

### 1A · Boundary self-test on a COLD-STARTED perimeter 🖥️ ⬜ (WS0-0b, task #39)

Bring the full perimeter up fresh on capable hardware, then prove each boundary
holds. Each is a pass/fail; record the command + result.

- [ ] **Network isolation — no direct egress.** From inside `vault-agent`, a
  direct connection to the public internet (bypassing the proxy) must fail:
  `podman exec vault-agent sh -c 'curl -sS --max-time 5 https://1.1.1.1 ; echo exit=$?'`
  → must NOT succeed (timeout / no route). The agent network is `internal: true`.
- [ ] **L7 allowlist enforced.** Through the proxy, an **off-allowlist** domain is
  blocked while an on-allowlist one works. Off-allowlist request → `BLOCKED` in
  `~/.opentrapp` proxy log / 403; on-allowlist (e.g. the agent's vendor API) → ok.
- [ ] **Credential injection — the key never sits with the agent.** The API key /
  bot token is injected by `vault-proxy`, not present in the agent:
  `podman exec vault-agent env | grep -iE 'ANTHROPIC|TELEGRAM|API_KEY'` → empty.
  (ADR-0001: proxy-side credential injection.)
- [ ] **L3 egress filter active.** `vault-egress` nftables ruleset is loaded and
  drops private/off-list destinations:
  `podman exec vault-egress nft list ruleset | grep -q vault_egress_drop_private`.
  A direct IP egress (not via the pinned resolver) is dropped.
- [ ] **Proxy CA unchanged / pinned.** The mitmproxy CA the agent trusts matches
  the expected fingerprint (no silent CA swap). Record the fingerprint; it must be
  stable across restarts.
- [ ] **No untrusted content on the host.** Confirm skill scanning / downloads run
  *inside* `vault-skills` (CLAUDE.md §9) — `run_command` for on-demand workloads
  goes through `podman exec`, not host bash.

> Build this as a script (`tests/boundary-selftest.sh`) so it's repeatable and can
> be the thing the daemon runs on every (re)start. Basis: `docs/threat-model.md` +
> CLAUDE.md §9.

### 1B · The same self-test on a RESUMED perimeter 🖥️ ⬜ (WS0-0b/0c, tasks #39/#40)

A boundary that is "alive but subtly wrong" after a resume is worse than a visible
failure (CLAUDE.md §11). Prove the resumed boundary == the cold boundary.

- [ ] Pause → resume (user) and re-run **all of 1A** → every box still passes.
- [ ] Idle-auto-pause → wake (dormant → resume) and re-run **all of 1A** → still passes.
- [ ] Daemon restart (`opentrapp-daemon` killed + relaunched, or its supervisor
  restart) → re-run **1A** → still passes.
- [ ] **Fail-closed:** if any boundary check fails on resume, the perimeter holds
  closed and alerts (does NOT serve traffic through a half-built boundary).
- [ ] Fold the contract into [ADR-0018](adr/0018-idle-auto-pause-host-waker.md) (WS4, task #45).

### 1C · Idle auto-pause + wake verified in PRODUCTION 🖥️ ⬜ (WS0-0a, task #35)

The headline memory feature, end-to-end under a real agent.

- [ ] Run a real agent, leave it idle past the threshold (~12 min) → it drops to
  **Dormant**, the perimeter stops, resident RAM falls to ~daemon-only.
- [ ] Send a Telegram message → it **wakes** and the bot replies **exactly once**
  (no double-process, no lost message; the agent's getUpdates offset survived).
- [ ] Measure cold-start latency on wake; confirm it's acceptable (~seconds).
- [ ] Confirm no waker/agent getUpdates overlap (no Telegram 409).

### 1D · Daemon-split defer verified, then promoted 🖥️ ⬜ (B4b)

- [ ] Run the full **`docs/b4b-hardware-test-plan.md`** (7 tests: inert default,
  daemon launch + ownership, the resting-memory win, control routing, idle
  auto-pause + wake, crash resilience, fallback). Record RSS numbers.
- [ ] If all 7 pass: flip `OPENTRAPP_DAEMON_DEFER` from opt-in to **default** in
  `daemon_link::defer_enabled`, update [ADR-0019](adr/0019-headless-daemon-gui-viewer-split.md)
  status, and record the measured resting RSS in
  [`footprint-and-device-usability.md`](footprint-and-device-usability.md) §10.4.
- [ ] Re-confirm 1A/1B still pass with the daemon (not the GUI) owning the perimeter.

### 1E · Code signing 👤 ⬜

Unsigned installers undercut "security tool" trust on first launch (SmartScreen /
Gatekeeper warnings).

- [ ] **Windows:** sign the `.exe`/`.msi` (SignPath OSS, or an EV cert). Resolve
  the prior SignPath blocker (see handoff §"RUN THIS NEXT").
- [ ] **macOS:** sign + **notarize** the `.app`/`.dmg` (Apple Developer ID).
- [ ] **Linux:** the AppImage `.sig`/`.pem` already ship (cosign/minisign) — document
  how a user verifies them.
- [ ] ✅ Already done: perimeter **container images** are cosign-signed +
  digest-pinned; SBOMs ship per platform.

---

## TIER 2 — Hardening (makes the claim robust)

### 2A · Proxy memory bounded over (load × time) 🖥️ ⬜ (WS1, tasks #41/#42)

A days-long security tool cannot leak.

- [ ] Measure `vault-proxy` (mitmproxy) RSS over a sustained load × duration matrix
  to attribute growth (steady vs leak).
- [ ] Apply the measurement-selected fix (flow cap / periodic recycle / streaming);
  confirm RSS is bounded over a multi-hour run. Reframe the footprint doc.

### 2B · Adversarial / red-team pass 🖥️ 👤 ⬜

Actually try to break out — the difference between "we built a boundary" and "the
boundary holds."

- [ ] Run a deliberately-hostile skill / agent inside the perimeter and attempt:
  host filesystem access, credential theft, off-allowlist exfil, direct-IP egress,
  DNS-rebinding, container escape. Each must be contained.
- [ ] Document what was attempted and what held / didn't. Honest results either way.
- [ ] File + fix anything that breaks out before recommending.

### 2C · Third-party security review 👤 ⬜

The gold standard for "official security tool."

- [ ] Get at least one external set of eyes on the threat model + perimeter
  (security-minded contributor, a community review, or a paid audit).
- [ ] ✅ Already have: OpenSSF Best Practices **passing** badge (#12755) — a real
  third-party signal, but not a substitute for a review.

---

## TIER 3 — Trust polish

### 3A · Cut a STABLE release (not an RC) ⬜ (WS3, task #44)

- [ ] Once every Tier-1 box is checked, cut a stable `v0.7.x` (or `v1.0`), with
  release copy that asserts **only** what's now verified (claims scoped to the
  hardware-verified results — §11 "gate the claim, not the workstream").

### 3B · Reproducible build + supply-chain story ⬜

- [ ] Verify the build reproduces from [`docs/reproduce.md`](reproduce.md) /
  `reproduce.sh` on a clean machine; document the expected digests.
- [ ] Confirm the SBOM + cosign-signed-image chain is verifiable end-to-end by a
  user (write the verification steps in the README).

### 3C · Residual-risk transparency, front-and-center ⬜

- [ ] Surface a prominent "**what this protects against / what it does NOT**"
  section (not buried) — we already state it can't make running an agent
  *absolutely* safe; that honesty *is* the credibility for a defense-in-depth tool.
  Basis: [`docs/threat-model.md`](threat-model.md), [`docs/why-not-x.md`](why-not-x.md).

---

## The gate

**Public recommendation is unlocked when all of Tier 1 is ✅.** Tier 2 makes it
defensible under scrutiny; Tier 3 makes it trustworthy on first contact. The
bottleneck is hardware that can run the full perimeter — so the first move next
session is to stand the perimeter up on the Windows box and start ticking 1A.
