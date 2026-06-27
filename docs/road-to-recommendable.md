# Road to a publicly-recommendable security tool — execution checklist

The goal: be able to recommend OpenTrApp **publicly, as an official security layer
for open agent systems** (OpenClaw and peers) — honestly, without overclaiming.

**The premise (CLAUDE.md §11):** the architecture is built and CI-green, but a
*security* tool's protective claims must be **verified at the consumption end on
real hardware**, not asserted from a green build.

**Update (2026-06-16, PR #112):** the **boundary self-test is now verified on the
7.2 GB Linux laptop itself** — the full from-scratch image build *and* the live
five-container perimeter ran with ~3.6 GB free and **no swap-storm**;
`make boundary-selftest` is exit 0 cold (1A) and across three restart-resume cycles
(1B restart path), reproducibly. The earlier "this box swap-storms the perimeter"
caveat does **not** hold for the *idle* boundary test. The remaining Tier-1 items
still need a **capable box or a live production run** — idle-auto-pause → wake under a
real agent (1C), the daemon-split memory win (1D) — plus the sustained-load soak (2A);
that's where the small box's headroom genuinely runs out.

**Status key:** ✅ done · 🔶 partial / implemented-but-unverified · ⬜ not started
· 🖥️ needs capable hardware · 👤 needs a human / external party

Work top-to-bottom. Tier 1 is load-bearing — do **not** make the public
recommendation until every Tier-1 box is checked. Tier 2/3 strengthen the claim
and can overlap.

---

## TIER 1 — Load-bearing (cannot recommend without these)

### 1A · Boundary self-test on a COLD-STARTED perimeter ✅ (WS0-0b, task #39)

**Verified ✅ (2026-06-16, PR #112).** [`tests/boundary-selftest.sh`](../tests/boundary-selftest.sh)
(`make boundary-selftest`) encodes all six checks below, fail-closed (exit 1 on any
boundary failure, exit 2 if it can't assess — "unverifiable ≠ verified"). On the
7.2 GB Linux laptop a fresh `make perimeter-up` + `make boundary-selftest ARGS=--record-baseline`
returns **exit 0, `pass=7 fail=0 skip=0`, "All boundaries hold"** — all six checks pass
on real hardware. Each box below maps to one check ID in the script.

- [x] **(B1) Network isolation — no direct egress.** From inside `vault-agent`, a
  proxy-bypassed connection to the public internet must fail (the agent network is
  `internal: true` — no gateway). Script unsets `HTTP(S)_PROXY` and confirms no route.
  *Verified:* direct hit to `1.1.1.1` → `Network unreachable`.
- [x] **(B2) L7 allowlist enforced.** Through the proxy, an **off-allowlist** host
  → 403 (`BLOCKED`); an on-allowlist host (e.g. the agent's vendor API) → not 403.
  *Verified:* `example.org` → 403, `api.anthropic.com` → 400 from upstream (allowed).
- [x] **(B3) Credential injection — the vendor key never sits with the agent.**
  `podman exec vault-agent env | grep -iE '^(ANTHROPIC|OPENAI)_API_KEY='` → empty
  (ADR-0001). **Note:** `TELEGRAM_BOT_TOKEN` *is* legitimately in the agent
  (OpenClaw polls Telegram itself, compose:69) — only the **Anthropic/OpenAI** key
  is proxy-injected, so the check asserts on that, not a blanket token grep.
- [x] **(B4) L3 egress filter active.** `vault-egress` nftables ruleset is loaded:
  `podman exec vault-egress nft list ruleset | grep -q vault_egress_drop_private`.
- [x] **(B5) Proxy CA unchanged / pinned.** The mitmproxy CA the agent trusts
  matches the recorded fingerprint (no silent CA swap). Cold start pins the
  baseline: `make boundary-selftest ARGS=--record-baseline`; resumes compare.
- [x] **(B6) No untrusted content on the host.** Skill delivery is read-only into
  the agent (compose `:ro`); untrusted scanning/downloads run *inside* `vault-skills`
  (CLAUDE.md §9), via `podman exec`, not host bash.

> The script doubles as the thing the daemon runs on every (re)start (1B). Basis:
> [`docs/threat-model.md`](threat-model.md) + CLAUDE.md §9.

### 1B · The same self-test on a RESUMED perimeter 🔶 (WS0-0b/0c, tasks #39/#40)

A boundary that is "alive but subtly wrong" after a resume is worse than a visible
failure (CLAUDE.md §11). Prove the resumed boundary == the cold boundary. Same
script as 1A — re-run `make boundary-selftest` after each resume path. **Partial
(2026-06-16, PR #112):** the **restart** path is verified ✅; the **idle→wake** path
(1C) and a daemon-level fail-close on a deliberately injected fault remain.

- [ ] Pause → resume (user) and re-run **all of 1A** → every box still passes.
  *(In-place pause/unpause not separately exercised; the stricter full-restart path
  below recreates the containers and re-passes all of 1A.)*
- [ ] Idle-auto-pause → wake (dormant → resume) and re-run **all of 1A** → still passes.
  *(→ the production path in 1C; not yet run.)*
- [x] Daemon/perimeter restart (`make perimeter-down && make perimeter-up`) → re-run
  **1A** → still passes. **Verified ✅:** exit 0, `pass=7`, **B5 "CA fingerprint
  unchanged"**, across **three** restart-resume cycles on real hardware.
- [ ] **Fail-closed:** if any boundary check fails on resume, the perimeter holds
  closed and alerts (does NOT serve traffic through a half-built boundary).
  *(The script's fail-closed exit codes were exercised — exit 1 on a failing check,
  exit 2 on a skip; the daemon supervisor's hold-closed on an injected fault is still
  unverified on hardware.)*
- [x] **Daemon wiring landed (default-ON, CI-green)** — `opentrapp_core::selftest`
  embeds the script; `verify_boundary_fail_closed` runs it after EVERY resume path —
  `resume_now`/`restart_now` (control channel) AND `idle::resume_from_dormant`
  (wake-on-message) (Fail→stop+`boundary-failed` marker, CannotAssess→alert,
  Pass→clear), **default ON** (opt-out `OPENTRAPP_SELFTEST_ON_RESUME=0`; verified 2026-06-26, §11).
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

### 1E · Code signing 👤 🔶

Unsigned installers undercut "security tool" trust on first launch (SmartScreen /
Gatekeeper warnings). **CI scaffolded** (commit pending) — both paths are wired
inert/secret-gated, so the only remaining work is human (provision certs + the
SignPath OSS account); see [`code-signing-policy.md`](code-signing-policy.md).

- [ ] **macOS:** sign + **notarize** the `.app`/`.dmg` (Apple Developer ID). The
  six `APPLE_*` env lines are a **ready-to-activate template** (commented in
  `ci.yml`) — uncomment once the secrets are populated. NOT passed empty: `tauri`
  treats a present-but-empty cert as "sign" and fails the bundle. *Needs: Apple
  Developer Program enrollment.*
- [ ] **Windows:** sign the `.exe`/`.msi` (SignPath OSS). A **ready-to-activate
  template** sits in `ci.yml` (commented, with an activation checklist) — uncomment
  + SHA-pin + add `SIGNPATH_*` secrets once the OSS project is approved. *Needs:
  SignPath OSS approval + SHA-pin.*
- [x] **Linux:** the AppImage `.sig`/`.pem` already ship (cosign keyless) — README
  documents `cosign verify-blob`.
- [x] perimeter **container images** are cosign-signed + digest-pinned; SBOMs ship
  per platform.

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
defensible under scrutiny; Tier 3 makes it trustworthy on first contact. **1A is now
✅ and 1B's restart path is verified (PR #112) — on the dev laptop, no Windows box
needed for the boundary self-test.** The remaining Tier-1 bottleneck is a *production*
run: 1C (idle-auto-pause → wake under a real agent), 1D (daemon-split), plus the 2A
soak — those still want a capable box or a live production session.
