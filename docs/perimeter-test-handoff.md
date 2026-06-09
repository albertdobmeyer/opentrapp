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
- **WS0-0c** — on a message it *wakes*, delivers **exactly once**, and resumes **security-correct**.
- **WS1-1a** — whether the always-on mitmproxy's RAM is *bounded* over a long session.

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

**Path B — `podman compose` directly (best for WS1-1a; no app needed).**
```
git clone https://github.com/albertdobmeyer/opentrapp && cd opentrapp
# create the runtime .env with your two keys (see infra/proxy for the variable names):
#   ANTHROPIC_API_KEY=...   TELEGRAM_BOT_TOKEN=...
podman compose up -d            # all five services
podman compose ps
```

## 3. Prerequisites on Windows

- **Container runtime:** install **Podman** (Podman Desktop sets up a WSL2-backed `podman
  machine`). Prefer it over Docker Desktop — the footprint doc explains why (podman ~hundreds
  of MB vs Docker Desktop ~1.5–3 GB). Give the podman machine ≥4 GB.
- **`gh` CLI** (for Path A's draft download) authenticated to your GitHub account.
- **Keys:** the `@opentrappbot` token + an Anthropic key (a low-cap key is fine).
- **Ollama** (optional) only if you also exercise the Sentinel AI rungs; not needed for these tests.

## 4. The tests (with consumption-end acceptance criteria)

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
- **Boundary checks after resume** (the part that makes resume *security-correct*, not just
  alive) — run these against the resumed perimeter:
  - From inside the agent container's network namespace, a direct connection to a
    **non-allowlisted** host **fails** (no direct egress).
  - A request to a **non-allowlisted** domain through the proxy is **blocked**.
  - The proxy still **injects the key** on an allowlisted request (the assistant got a real
    LLM reply ⇒ injection worked).
  - `podman exec` into egress and confirm the nftables ruleset is loaded (RFC1918 dropped).
  - Record whether any check fails — a resumed-but-leaky boundary is the worst outcome and
    the reason this test exists.

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
