# Live-perimeter test handoff (capable machine)

*A self-contained runbook for running the hardware-gated perimeter tests on a machine with
RAM headroom (e.g. a 16 GB+ Windows desktop), when the primary dev laptop can't sustain
them. Written 2026-06-09. **You do not need the chat session that produced this** — the repo
carries all the context; the pointers in §1 are the reading list.*

> **Read first — Linux is the default, this is the fallback.** We are deliberately trying to
> do as much as possible on the constrained 7.2 GB Linux laptop, because if it runs smoothly
> there it runs anywhere. Use this runbook only for what that box genuinely can't sustain: a
> full five-container perimeter under sustained active-agent load, and the long-soak proxy
> growth measurement. Everything else, do on Linux.

## 1. Context (the reading list — all committed in the repo)

| Read | For |
|---|---|
| `docs/specs/2026-06-09-lean-verified-perimeter-roadmap.md` | The plan; the workstreams (WS0–WS4) these tests serve |
| `docs/footprint-and-device-usability.md` | Where the memory goes; the numbers these tests confirm |
| `CLAUDE.md` §11 | Verification discipline — *verify at the consumption end*; acceptance criteria below follow it |
| `docs/adr/0018-idle-auto-pause-host-waker.md` | How idle auto-pause + the host waker work |
| `docs/reproduce.md` + `docs/reproduce.sh` | How to stand the perimeter up from scratch |

**What we're proving** (all currently *unverified, not done*):
- **WS0-0a** — idle auto-pause actually *fires* in production.
- **WS0-0b** — the perimeter passes the **automated boundary self-test** (`tests/boundary-selftest.sh`)
  **cold**, and passes the **same** test **after a resume** (the §11 cold==resumed contract). This is
  now scripted (6 checks B1–B6, fail-closed exit codes) and is the load-bearing **opencode gate**.
- **WS0-0c** — on a message it *wakes*, delivers **exactly once**, and resumes **security-correct**.
- **WS1-1a** — whether the always-on mitmproxy's RAM is *bounded* over a long session.

> **Fastest path to the opencode gate:** §4 **T0** below (the automated boundary self-test) is the
> single most important thing to run — it proves "the perimeter actually contains the agent" cold and
> after resume. T1/T2 (idle→wake) supply the *production-faithful* resume that T0 then re-verifies.

## 2. Two ways to run it

**Path A — install the v0.7.0 build (best for WS0; also re-tests the first-run fix).**
The v0.7.0 release is staged as a **draft**; as the repo owner you can pull its Windows
installer:
```
gh release download v0.7.0 --repo albertdobmeyer/opentrapp --pattern "OpenTrApp_0.7.0_x64_en-US.msi"
```
Install it, launch, and go through the **first-run wizard** (enter the Anthropic key + the
`@opentrappbot` Telegram token). The wizard writes the runtime `.env` for you — that write is
exactly the v0.7.0 first-run fix, so a clean run here doubles as a Windows re-grade of it.
This path runs the real shipped app, including the watchdog that drives idle auto-pause.

**Path B — compose directly (best for WS1-1a; no app needed).**
```
git clone https://github.com/albertdobmeyer/opentrapp && cd opentrapp
# create the runtime .env with your two keys (see infra/proxy for the variable names):
#   ANTHROPIC_API_KEY=...   TELEGRAM_BOT_TOKEN=...
make perimeter-up               # auto-detects podman-compose (see T0 runner notes); all five services
make perimeter-status
```
(If invoking compose by hand, use `podman-compose up -d`, **not** `podman compose up -d` — see the
runner notes in §4 T0 for why the docker-compose CLI plugin breaks on this file.)

## 3. Prerequisites on Windows

- **Container runtime:** install **Podman** (Podman Desktop sets up a WSL2-backed `podman
  machine`). Prefer it over Docker Desktop — the footprint doc explains why (podman ~hundreds
  of MB vs Docker Desktop ~1.5–3 GB). Give the podman machine ≥4 GB.
- **WSL2 terminal for all shell work.** The `make` targets, `bash tests/*.sh`, and
  `podman compose` all run from a WSL2 terminal (not PowerShell or CMD). Open one via
  Windows Terminal → Ubuntu (or `wsl`). GNU make and bash are available there by default;
  the WSL2 podman shares the same machine as Podman Desktop so container operations
  work transparently. Clone or `cd` to the repo path under `/mnt/b/...` or mount it.
- **`gh` CLI** (for Path A's draft download) authenticated to your GitHub account.
  Install inside WSL2: `gh auth login`.
- **Keys:** the `@opentrappbot` token + an Anthropic key (a low-cap key is fine).
  Copy `.env.example` → `.env` at the repo root and fill in both values before
  running `podman compose up -d`.
- **Ollama** (optional) only if you also exercise the Sentinel AI rungs; not needed for these tests.

## 4. The tests (with consumption-end acceptance criteria)

### T0 — WS0-0b: automated boundary self-test, cold == resumed (the opencode gate)

The scripted proof that the perimeter actually *contains* the agent. It runs six boundary checks
against the live perimeter — B1 network isolation, B2 L7 allowlist (deny + allow), B3 credential
injection, B4 L3 egress filter, B5 proxy-CA pinned, B6 no host-side untrusted — fail-closed.
Exit codes: **0** = all hold · **1** = a boundary FAILED (breach) · **2** = could not assess
(perimeter down / tool missing — *not* a pass; unverifiable ≠ verified).

> **Verified on the 7.2 GB Linux laptop (2026-06-16) — T0 does *not* need this Windows fallback.**
> The full from-scratch image build **and** the live five-container perimeter ran with ~3.6 GB free
> and **no swap-storm**; T0 was **exit 0 cold and across three resume cycles** (`pass=7`, CA unchanged).
> Runner notes from that run, now encoded in the Makefile and the commands below:
> - **Use native `podman-compose`.** Bare `podman compose` selects the docker-compose CLI plugin where
>   installed, which breaks on this file: it inlines `security_opt: seccomp=<file>` as JSON (podman
>   rejects it as a path → "file name too long"), mismatches network labels, and needs the
>   `podman.socket` user service (`systemctl --user enable --now podman.socket`). `make perimeter-up`
>   now auto-detects and prefers `podman-compose`.
> - **`podman-compose` ignores `profiles:`**, so it starts **all five** containers (incl. on-demand
>   skills/social), not three. Harmless for T0 — B6 still asserts the agent's read-only skills mount.
> - **Two harness fixes shipped from that run** (`tests/boundary-selftest.sh`): B1/B2 now detect the
>   agent's **busybox wget** (the hardened image strips the curl/wget symlinks, so the old
>   `command -v wget` made them SKIP); and B4 no longer pipes `nft | grep` straight into the `if`
>   under `set -o pipefail` (a transient non-zero `podman exec` made it flake on a fresh egress).

**Cold start — record the CA baseline and prove the boundary holds fresh:**
```
make perimeter-up                       # auto-detects podman-compose; brings up all five services
make perimeter-status                   # wait until all vault-* show Up/healthy
make boundary-selftest ARGS=--record-baseline
```
- **PASS:** `pass=7 fail=0 skip=0`, `All boundaries hold.`, **exit 0** (seven assertions: B1, B2-deny,
  B2-allow, B3, B4, B5, B6). The `--record-baseline` run pins the proxy-CA fingerprint to
  `~/.opentrapp/boundary/ca-fingerprint.expected`.
- A `FAIL` (exit 1) is a real cold breach — capture the line, that's a finding. `skip` (exit 2) means
  a check couldn't run — note which.
- **Use `make boundary-selftest`, not bare `bash tests/boundary-selftest.sh`.** The make target derives
  the live container names from the compose-service label, so it works whatever the runner named them
  (podman-compose → `opentrapp_vault-*_1`). Bare invocation assumes containers literally named `vault-*`
  and otherwise reports CANNOT ASSESS.

**Resumed — the §11 core: the SAME test must pass after a resume, CA unchanged:**
```
# production-faithful (preferred): the daemon runs the self-test on every (re)start BY DEFAULT
# (incl. the idle-pause → wake cycle, T1 → T2) — no env var needed (verified 2026-06-26):
# export OPENTRAPP_SELFTEST_ON_RESUME=0   # ONLY to disable it (default is ON)

# manual proxy (Path B / no daemon): restart the perimeter, then re-assess against the baseline:
make perimeter-down && make perimeter-up && make perimeter-status
make boundary-selftest                  # assess mode — compares CA to the recorded baseline
```
- **PASS (the gate):** `pass=7 fail=0 skip=0`, **exit 0**, and **B5-ca-pinned → "CA fingerprint
  unchanged"** — the resumed perimeter holds the *same* boundaries as cold, with no silent CA swap.
- **FAIL (exit 1):** a resumed-but-leaky boundary — the worst outcome, and the whole reason this test
  exists. Capture the failing check name.

**Bring back:** the two result lines (cold + resumed) with exit codes (`--json` is easy to paste:
`make boundary-selftest ARGS=--json`). A green T0 cold **and** resumed is the boundary half of
"ready to recommend to opencode."

### T1 — WS0-0a: does idle auto-pause FIRE? (Path A)
1. Complete the wizard; confirm the assistant reaches the running/green state.
2. Leave it **completely idle** for ~15 min (default idle threshold is ~12 min).
3. **Observe:** the hero should switch to a *Dormant / "sleeping to save memory"* state;
   `podman ps` should show the perimeter **stopped**; host RAM should drop to roughly the
   app shell only.
- **PASS (consumption-end):** the perimeter is actually *down* and RAM actually *dropped* —
  not just a UI label. Record: time-to-dormant, and `free` before vs after.
- **If it never goes dormant:** capture `podman volume inspect <...>vault-proxy-logs` →
  confirm `requests.jsonl` exists + is being written (ZONE 3), and `tail` it to see whether
  OpenClaw keeps polling Telegram while idle (if the log mtime never goes stale, the idle
  signal can't trip — that's the WS0-0a finding to bring back).

### T2 — WS0-0c: wake + exactly-once + security-correct resume (Path A)
1. From the dormant state, **send one message** ("wake test") to `@opentrappbot`.
2. **Observe:** the perimeter resumes within a few seconds and the assistant **replies once**.
- **PASS:** exactly **one** reply (no double-processing, no lost message); record cold-start
  latency.
- **Boundary checks after resume** (the part that makes resume *security-correct*, not just alive):
  this is now **T0's automated self-test** — run it against the just-resumed perimeter:
  ```
  make boundary-selftest                 # expect: exit 0, pass=7, B5 "CA fingerprint unchanged"
  ```
  It covers exactly the manual checks this step used to list (no direct egress, off-allowlist host
  blocked, key still injected on an allowlisted request, nftables drop-private loaded) plus the
  CA-pin compare. A resumed-but-leaky boundary (exit 1) is the worst outcome and the reason this
  test exists. (The daemon runs this for you on wake BY DEFAULT — `OPENTRAPP_SELFTEST_ON_RESUME=0` to disable.)

### T3 — WS1-1a: is the proxy's RAM bounded over time? (Path B)
1. With the perimeter up, drive **sustained, large, streaming** traffic through the proxy —
   e.g. a loop of real agent turns that produce long streamed (SSE) responses, for ≥30–60 min.
2. Sample the proxy container RSS periodically: `podman stats --no-stream vault-proxy`
   (or the repo's `make profile-memory`) every ~2 min; log it.
- **PASS:** RSS **plateaus** (bounded) rather than climbing monotonically.
- **If it climbs:** that confirms the large-body-buffering hypothesis → the fix is
  `stream_large_bodies` (see WS1-1b in the roadmap, including its precondition: confirm no
  security function reads full response bodies first). Bring back the RSS-over-time curve.

### T4 — bonus: first-run re-grade on Windows (Path A)
If the wizard in T1 completed without a "couldn't save your settings" error, that's a clean
Windows confirmation of the v0.7.0 first-run fix. Note it explicitly.

## 5. What to bring back
A short note with: time-to-dormant + RAM before/after (T1); reply-count + cold-start +
each boundary-check result (T2); the proxy RSS-over-time curve and whether it plateaued (T3);
and any failure. Drop it in `docs/specs/` or paste it back into a session.

## 6. Safety / teardown
```
podman compose down          # Path B
# Path A: quit the app from the tray (perimeter tears down on exit)
podman ps                     # confirm nothing left running
```
The perimeter preserves volumes across stop/start (so the agent's Telegram offset survives) —
`down` is safe to repeat.
