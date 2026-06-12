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

### 1A · Boundary self-test on a COLD-STARTED perimeter 🖥️ 🔶 (WS0-0b, task #39)

**Script authored** — [`tests/boundary-selftest.sh`](../tests/boundary-selftest.sh)
(`make boundary-selftest`) encodes all six checks below, fail-closed (exit 1 on any
boundary failure, exit 2 if it can't assess — "unverifiable ≠ verified"). It is
syntax/lint-clean and its fail-closed paths are verified; the boundary assertions
themselves are 🔶 **unrun pending capable hardware**. Bring the perimeter up fresh
(`make perimeter-up`), then `make boundary-selftest` and record the output. Each
box below maps to one check ID in the script.

- [ ] **(B1) Network isolation — no direct egress.** From inside `vault-agent`, a
  proxy-bypassed connection to the public internet must fail (the agent network is
  `internal: true` — no gateway). Script unsets `HTTP(S)_PROXY` and confirms no route.
- [ ] **(B2) L7 allowlist enforced.** Through the proxy, an **off-allowlist** host
  → 403 (`BLOCKED`); an on-allowlist host (e.g. the agent's vendor API) → not 403.
- [ ] **(B3) Credential injection — the vendor key never sits with the agent.**
  `podman exec vault-agent env | grep -iE '^(ANTHROPIC|OPENAI)_API_KEY='` → empty
  (ADR-0001). **Note:** `TELEGRAM_BOT_TOKEN` *is* legitimately in the agent
  (OpenClaw polls Telegram itself, compose:69) — only the **Anthropic/OpenAI** key
  is proxy-injected, so the check asserts on that, not a blanket token grep.
- [ ] **(B4) L3 egress filter active.** `vault-egress` nftables ruleset is loaded:
  `podman exec vault-egress nft list ruleset | grep -q vault_egress_drop_private`.
- [ ] **(B5) Proxy CA unchanged / pinned.** The mitmproxy CA the agent trusts
  matches the recorded fingerprint (no silent CA swap). Cold start pins the
  baseline: `make boundary-selftest` once with `--record-baseline`; resumes compare.
- [ ] **(B6) No untrusted content on the host.** Skill delivery is read-only into
  the agent (compose `:ro`); untrusted scanning/downloads run *inside* `vault-skills`
  (CLAUDE.md §9), via `podman exec`, not host bash.

> The script doubles as the thing the daemon runs on every (re)start (1B). Basis:
> [`docs/threat-model.md`](threat-model.md) + CLAUDE.md §9.

### 1B · The same self-test on a RESUMED perimeter 🖥️ 🔶 (WS0-0b/0c, tasks #39/#40)

A boundary that is "alive but subtly wrong" after a resume is worse than a visible
failure (CLAUDE.md §11). Prove the resumed boundary == the cold boundary. Same
script as 1A — re-run `make boundary-selftest` after each resume path:

- [ ] Pause → resume (user) and re-run **all of 1A** → every box still passes.
- [ ] Idle-auto-pause → wake (dormant → resume) and re-run **all of 1A** → still passes.
- [ ] Daemon restart (`opentrapp-daemon` killed + relaunched, or its supervisor
  restart) → re-run **1A** → still passes.
- [ ] **Fail-closed:** if any boundary check fails on resume, the perimeter holds
  closed and alerts (does NOT serve traffic through a half-built boundary).
- [x] **Daemon wiring landed (opt-in, CI-green)** — `opentrapp_core::selftest`
  embeds the script; the supervisor runs it after every (re)start
  (`verify_boundary_fail_closed`: Fail→stop+`boundary-failed` marker, CannotAssess→
  alert, Pass→clear), gated on `OPENTRAPP_SELFTEST_ON_RESUME=1` (default OFF, §11).
  `opentrapp-daemon --boundary-selftest` runs it once on demand. Contract folded
  into [ADR-0018](adr/0018-idle-auto-pause-host-waker.md) addendum (WS4, task #45).
  **Remaining:** enable + verify green on hardware, then promote to default.

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

### 2A · Proxy memory bounded over (load × time) 🖥️ 🔶 (WS1, tasks #41/#42)

A days-long security tool cannot leak. **Sampler authored** —
[`tests/proxy-memory-soak.sh`](../tests/proxy-memory-soak.sh) (`make proxy-soak`):
samples `vault-proxy` RSS at a fixed interval over a set duration, optionally
drives synthetic load through the proxy, and prints a growth attribution
(baseline / peak / final / MB-per-hour slope / leak verdict). Lint + exit-code
paths verified on the dev box; the soak itself is 🔶 **unrun pending hardware**.

- [ ] Run `make proxy-soak --duration 360` (or leave a real agent driving load,
  `--load off`) → attribute growth (steady vs leak).
- [ ] Apply the measurement-selected fix (flow cap / periodic recycle / streaming);
  confirm RSS is bounded over a multi-hour run. Reframe the footprint doc.

### 2B · Adversarial / red-team pass 🖥️ 👤 🔶

Actually try to break out — the difference between "we built a boundary" and "the
boundary holds." **Breakout battery authored** —
[`tests/red-team-breakout.sh`](../tests/red-team-breakout.sh) (`make red-team`):
drives R1 host-write, R2 cred-theft, R3 off-allowlist exfil, R4 direct-IP egress,
R5 lateral movement, R6 escape surface, R7 DNS-rebind (manual note) — each must be
**contained** (CONTAINED=pass, BREACH=fail). Lint + cannot-assess paths verified
here; 🔶 **unrun pending hardware**.

- [ ] Run `make red-team` on a cold perimeter → all CONTAINED, zero BREACH.
- [ ] Re-run with a deliberately-hostile skill / agent loaded (the 👤 part).
- [ ] Complete the R7 DNS-rebinding pass manually (needs attacker-controlled DNS).
- [ ] Document what was attempted and what held / didn't. File + fix any BREACH.

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

### 3C · Residual-risk transparency, front-and-center ✅

- [x] Surface a prominent "**what this protects against / what it does NOT**"
  page (not buried) — [`docs/what-this-protects.md`](what-this-protects.md): a
  two-minute plain-language distillation of the threat model's T1–T6, with the
  "does NOT" half given equal weight, plus how a user raises their own assurance
  (disposable VM, dedicated Telegram, least shell level). Linked front-and-center
  from the README **Values** line and as the first entry under README **Limitations**.
  Basis: [`docs/threat-model.md`](threat-model.md), [`docs/why-not-x.md`](why-not-x.md).

---

## The gate

**Public recommendation is unlocked when all of Tier 1 is ✅.** Tier 2 makes it
defensible under scrutiny; Tier 3 makes it trustworthy on first contact. The
bottleneck is hardware that can run the full perimeter — so the first move next
session is to stand the perimeter up on the Windows box and start ticking 1A.
