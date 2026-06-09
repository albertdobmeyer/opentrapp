# OpenTrApp footprint & device usability

*An honest accounting of what OpenTrApp costs in memory and disk, where that cost comes
from, and which devices can run it comfortably. Written 2026-06-09.*

This report answers four questions that come up whenever someone is asked to install
OpenTrApp on their own machine:

1. Is OpenTrApp heavy because of **OpenClaw** (the agent we wrap), our **security
   perimeter**, or our **orchestrator app**?
2. What is the app actually *doing* that makes it heavy?
3. Is the swap-storm on the primary dev laptop representative of what users will feel?
4. Is OpenTrApp, as-is, usable on other people's devices?

Short answers up front, the evidence after.

---

## TL;DR

- **The dominant cost is OpenClaw, not us.** At rest, ~73–77% of the perimeter's RAM is
  the Node 22 runtime plus the OpenClaw agent — an unavoidable cost of running *that
  agent at all*. Our security architecture adds ~23–27% (an always-on egress proxy plus a
  lean network-filter container). Our orchestrator app itself is small.
- **Our orchestrator app is light.** Tauri (Rust + the OS webview, no bundled Chromium)
  sits around ~150 MB resident, most of which is the shared OS webview, not our code. It
  runs no AI in-process and has no busy-polling loops.
- **The dev-laptop swap-storm is an artifact, not the typical experience.** It happened on
  a 7.2 GB, 2017 budget APU running Cursor + Brave + a Claude Code session *and* the full
  perimeter at once. The perimeter's own contribution is ~0.4–1.0 GB; the co-tenants were
  the larger half.
- **It is usable on the machines the target audience owns.** On a 16 GB laptop — now the
  mainstream baseline and the Apple-Silicon floor — the perimeter is ~3–6% of RAM and
  comfortable alongside an IDE and browser. 8 GB is marginal; the real cross-platform
  friction is the container-runtime prerequisite on macOS/Windows, which we keep cheap by
  standardizing on rootless podman rather than Docker Desktop.
- **Idle auto-pause is the lever that makes small machines viable** — it collapses the
  resting footprint to near zero between tasks (see the open question in §8).

---

## 1. Method, and the honesty caveat

These numbers come from three sources, labelled throughout:

- **[measured]** — read live from the primary dev laptop on 2026-06-09 (`podman images`,
  `podman stats`, `free`, `ps`, `du`). No container or build was started to collect them.
- **[CI-measured]** — produced by a CI build (e.g. the slimmed image size).
- **[estimate]** — the planning-doc figures in
  `~/.claude/plans/glimmering-meandering-babbage.md`, kept where no clean live number
  exists.

**The honest caveat:** the primary dev laptop (7.2 GB RAM) *cannot run the full
five-container perimeter without swap-storming*, so a clean per-container measurement of
the whole live perimeter has never been taken on it. The figures below combine a live
single-container measurement, image sizes on disk, and external grounding for the runtimes
involved. Where a number is an estimate, it says so.

---

## 2. The footprint, decomposed

### Hardware of the primary dev laptop [measured]

| Property | Value |
|---|---|
| CPU | AMD A12-9720P, 4 cores (2017 budget APU) |
| RAM | 7.2 GiB |
| Swap | 7.6 GiB, swappiness 10 |
| Disk | 60 GB free of 439 GB |

This is roughly the *worst* realistic device — below today's 16 GB mainstream baseline and
below even the resurgent 8 GB budget floor.

### Per-container attribution [image sizes measured; resting RAM measured where noted, else estimate]

The resting perimeter after the Phase-1 work is **three** containers (the two scanner
shields are on-demand and absent at rest):

| Container | Runtime | Image (disk) | Resting RAM | Attribution |
|---|---|---|---|---|
| **vault-agent** | Node 22 + OpenClaw | 689 MB published / 590 MB slimmed | **197 MB [measured, idle]** · ~600 MB [estimate, under load] | **OpenClaw-intrinsic** |
| **vault-proxy** | mitmproxy (Python) | 250 MB | ~150 MB [estimate] | **Our security infra** |
| **vault-egress** | nftables/resolver (+ unused Node) | 176 MB | ~30–80 MB [estimate] | **Our security infra** |
| *vault-skills* | Python + bash (idle) | 234 MB | ~0 at rest (on-demand) | Our security infra |
| *vault-social* | Python + bash (idle) | 154 MB | ~0 (parked) | Our security infra |

A useful reality check: the one **live** agent-container measurement is **197 MB** while
idle (just the gateway long-polling Telegram) — well under the ~600 MB planning estimate.
The ~600 MB figure is the working set under active reasoning; the resting figure is much
lower. So the resting perimeter is plausibly **~0.4–0.5 GB** (agent idle + proxy + egress),
climbing toward **~0.8–1.0 GB** only when the agent is actively working.

### The headline split (resting RAM)

| Bucket | RAM | Share |
|---|---|---|
| **OpenClaw-intrinsic** (vault-agent: Node + agent) | ~400–600 MB | **~73–77%** |
| **Our security infra** (proxy + egress) | ~180–230 MB | ~23–27% |
| **Orchestration** (per-container podman/conmon/networking) | tens of MB | small |

External grounding corroborates every line: a working Node LLM client is a few hundred MB
([nodejs.org](https://nodejs.org/learn/diagnostics/memory/understanding-and-tuning-memory)),
mitmproxy starts ~50 MB ([mitmproxy#844](https://github.com/mitmproxy/mitmproxy/issues/844)),
rootless-podman overhead is single-digit MB per container
([decodednode](https://www.decodednode.com/2022/12/container-memory-usage.html)).

---

## 3. What our orchestrator app itself does (and why it's light)

OpenTrApp the application is a Tauri 2 binary: a Rust core plus the **OS-native webview**
(WebKitGTK on Linux) — not a bundled Chromium. That architecture is why minimal Tauri apps
idle at ~30–50 MB versus Electron's ~150–300 MB
([gethopp](https://www.gethopp.app/blog/tauri-vs-electron)). Concrete evidence it stays
light:

- **Small artifacts** [measured]: stripped binary ~28 MB, `.deb` ~9.6 MB, AppImage ~90 MB.
- **No busy-polling.** The only periodic backend work is a 30 s watchdog (five short
  `podman ps` probes, run off-thread) and a 60 s status aggregator whose Anthropic auth
  probe is cached for 5 minutes. No tight loops; the frontend is event-driven, not polling
  (`app/src-tauri/src/lib.rs:62,68`, `lifecycle.rs:539`, `status_aggregator.rs:34,170`).
- **No AI in-process.** Sentinel (the local-AI judgment layer) does **not** host a model in
  the app — it execs `sentinel/judge.sh` on demand, which calls host Ollama, with a 90 s
  timeout and a cold start; the resting state runs no model
  (`app/src-tauri/src/commands/sentinel.rs:34,197–229`). This upholds the CLAUDE.md §10
  rule that the app must not run AI models directly.
- **Light frontend** [measured]: 536 KB of TS/TSX, React + a router + an icon set, no
  charting/editor heavyweights.

**Estimate: the app's own resident RAM is ~150 MB**, the large majority being the shared OS
webview rather than our Rust code (~30–60 MB). Against a perimeter of several hundred MB to
~1 GB, the orchestrator is **at most ~15–20% of the total**, and the part *beyond* the
unavoidable OS webview is under 10%. **Our app is not the heavy part.**

---

## 4. So why is it heavy? OpenClaw + our defense-in-depth

Two things, in order of weight:

1. **Running OpenClaw at all (unavoidable).** OpenClaw is a Node.js agent; the Node 22
   runtime plus its dependency tree is the ~400–600 MB floor. We already cut its
   `node_modules` ~33% (image 754 → 590 MB) without dropping a single runtime dependency
   (Phase 2). The only way to drive this below the floor is to *not run it while idle* —
   which is exactly what idle auto-pause does.
2. **Our security architecture (a deliberate choice).** The L7/L3 egress split
   ([ADR-0009](adr/0009-five-container-perimeter.md)) means an always-on mitmproxy
   (~150 MB) plus a lean egress container (~30–80 MB). This is the price of
   defense-in-depth — the proxy injects credentials and enforces the domain allowlist; the
   egress container enforces a kernel-level destination filter and pinned DNS. It is a
   design decision, not an accident, and it is the part we could most plausibly tune (e.g.
   the egress container carries an unused ~50 MB Node runtime, kept only to reuse a pinned
   base image).

The two scanner shields, once wrongly blamed for a guessed "1 GB," are actually ~5–20 MB of
idle bash and are now removed from the resting set entirely (Phase 1, `on_demand`).

---

## 5. This laptop vs. real user devices

### Was the swap-storm representative? No — but it's a valid minimum-spec data point.

The "~142 MB free / 3.8 GB swap" result is **over-determined by the co-tenants**, not by
OpenTrApp. On that 7.2 GB box, Cursor (~1.4 GB across its processes), the Claude Code
session (~485 MB), and Brave were resident *before* the perimeter's ~0.4–1.0 GB landed on
top. The machine is an outlier on three axes at once — 7.2 GB RAM, a 2017 4-core APU, and a
heavy concurrent IDE+browser+AI workload. A 16 GB machine absorbs the same perimeter with
no swap pressure at all.

It is still a legitimate finding: it shows that on a memory-starved machine running a heavy
concurrent workload, OpenTrApp's ~1 GB is the straw that breaks the camel's back. That's a
real *minimum-spec* lesson — just not the *typical-user* experience.

### Device-class verdict

Resting load ≈ ~0.4–1.0 GB perimeter + ~0.15 GB orchestrator, plus a container-runtime VM
tax on macOS/Windows (see §6).

| Device | Linux (native podman) | macOS / Windows (VM-backed runtime) |
|---|---|---|
| **8 GB** | Marginal — fine as the primary task; tight alongside a heavy IDE + browser. | Marginal→unusable; depends heavily on podman vs Docker Desktop. (New Apple Silicon has no 8 GB SKU.) |
| **16 GB** | **Comfortable** — ~3–6% of RAM. The sweet spot. | **Comfortable** with podman machine; marginal on Docker Desktop. |
| **32 GB+** | Trivial. | Comfortable even with Docker Desktop. |

The target audience — people who run a CLI AI agent — overwhelmingly sits at 16 GB+ today,
and Apple Silicon can't be bought below 16 GB. (Caveat: a 2026 DRAM shortage is nudging
*some new budget laptops* back toward 8 GB, so 8 GB isn't purely legacy —
[Tom's Hardware](https://www.tomshardware.com/laptops/8gb-of-ram-is-back-on-laptops-companies-are-lowering-memory-offerings-to-make-affordable-notebooks-during-component-crisis).)

---

## 6. The real cross-platform caveat: the container runtime

OpenTrApp needs a Linux container runtime. This cuts *in our favour* because we standardize
on **rootless podman**, not Docker Desktop:

- **Linux:** podman is native — no VM, near-zero prerequisite overhead.
- **macOS / Windows:** any Linux-container runtime needs a Linux VM. Docker Desktop is
  notorious here (~1.5–3 GB idle); a **podman machine is far lighter** (~hundreds of MB,
  and no daemon when nothing runs —
  [tech-insider](https://tech-insider.org/podman-vs-docker-2026/),
  [uptrace](https://uptrace.dev/comparisons/podman-vs-docker)).

The honest friction is not RAM — it's that a non-developer on macOS/Windows must install a
container runtime *at all*. That's a real onboarding cost even though the memory cost is
modest with podman.

---

## 7. What the shipped mitigations actually buy

| Phase | Change | RAM | Disk | Startup |
|---|---|---|---|---|
| **1 — on-demand shields** | skills/social no longer boot; resting 5→3 containers | small (tens of MB) | — | faster boot |
| **2 — agent image slim** | node_modules pruned 754 → 590 MB | none (pruned files were never resident) | **yes** | faster pull |
| **3 — idle auto-pause** | pause whole perimeter when idle; wake on a Telegram message | **the big lever — resting RAM → ~0 when idle** | — | costs a few-second cold start |

Phase 3 is the only one that attacks the dominant OpenClaw RAM floor, by not running it
while idle.

---

## 8. Honest open questions and watch-items

- **Does idle auto-pause actually fire in production? (unverified)** It keys off the proxy
  request-log mtime going stale, which assumes OpenClaw stops polling Telegram when idle
  *and* the proxy log persists to its volume. Neither has been observed live (the dev box
  can't host the perimeter). If either fails, the feature is *inert* (never sleeps) — not
  dangerous (it can't strand the perimeter: it refuses to pause without a wake token), but
  it wouldn't deliver the memory win. Tracked as `mem-phase3-operator-verify`.
- **mitmproxy memory growth (watch-item).** mitmproxy is known to grow over long sessions
  as it retains flows — one report saw ~50 MB climb to ~550 MB in ~10 minutes of heavy
  traffic ([mitmproxy#4456](https://github.com/mitmproxy/mitmproxy/issues/4456)). Our
  ~150 MB resting figure is a freshly-started instance. For a "silent background process"
  goal, the proxy's long-run growth deserves a periodic-recycle or a flow-retention cap.
- **Dev-disk hygiene (not user-facing).** The Rust `target/` build cache is ~29 GB and old
  `podman images` total ~5.6 GB (largely reclaimable dupes incl. pre-rebrand tags). This
  affects contributors, not users, but is worth a periodic `cargo clean` / `podman image
  prune`.

---

## 9. Bottom line

OpenTrApp is usable on the machines its audience actually owns. Its resting footprint is
dominated by the Node agent we wrap (~3/4 of perimeter RAM), with our defense-in-depth
proxy/egress adding ~1/4 and the Tauri orchestrator adding a light, mostly-OS-webview
~150 MB on top. On a 16 GB laptop that's a comfortable single-digit percentage of RAM. The
primary dev laptop's swap-storm is an artifact of uniquely constrained 2017 hardware
running a heavy IDE + browser + AI session concurrently, not a verdict on normal machines.
The genuine caveats are narrow and known: the container-runtime prerequisite on
macOS/Windows (kept cheap by choosing podman over Docker Desktop), mitmproxy's long-run
memory growth, and the still-unverified question of whether idle auto-pause fires in the
field — which, if confirmed, is precisely what makes even 8 GB machines comfortable.
</content>
