# Windows session handoff — portability fixes + architecture review

*Written 2026-06-15 by the Windows-box Claude Code session. Addressed to the Linux-laptop
agent for discussion with Albert. This document presents findings and open questions —
it does not recommend a decision. Albert will discuss the architecture with the Linux agent.*

---

## What happened this session

Albert pulled the repo on the Windows workstation (main dev machine, ~16 GB+ RAM) and
asked for two things:

1. **Make the tooling OS-agnostic** so the perimeter tests can run on Windows via WSL2.
2. **Think through the architecture** with a co-architect before deciding whether to run T0
   at all on the Windows machine.

The outcome: Albert decided **not to run the tests on Windows** (his main workstation, not
a test machine) and instead wants to rethink the architecture on the Linux laptop before
testing anything. He explicitly said: *"the entire point was to get it to run on the small
linux laptop — if it can't run there the project is a fail."*

This session committed the portability fixes and is handing the architecture discussion back
to you.

---

## 1. Portability fixes committed this session

Four independent audit agents swept the entire tooling surface. All changes are in this PR.

### compose.yml — two blockers fixed

**Lines 49 and 117: hardcoded absolute seccomp paths** (blocker on any machine that isn't
Albert's Linux laptop at the original clone path):

```yaml
# Before (machine-specific, broken everywhere else):
- seccomp=/home/albertd/Repositories/opentrapp/workloads/agent/config/vault-seccomp.json

# After (relative to repo root, works anywhere):
- seccomp=./workloads/agent/config/vault-seccomp.json
```

Both `vault-seccomp.json` and `vault-proxy-seccomp.json` exist in the repo at
`workloads/agent/config/` — the path was wrong, the files were fine.

### tests/memory-profile.sh — three portability fixes

- `/proc/meminfo` direct read → guarded with `[ -f /proc/meminfo ]` fallback (`"N/A"`)
- `free -h` → guarded with `command -v free` fallback (graceful skip)
- Hardcoded `podman stats` / `podman images` → `$RUNTIME stats` / `$RUNTIME images`
  via the same `detect_runtime()` pattern already used in `boundary-selftest.sh`

These fixes make the script work on macOS and any future non-Linux host. On WSL2 (Linux
kernel) they're all a no-op — `/proc/meminfo` and `free` are available there. The guard
is hygiene for the general case.

### .env.example — created at repo root

No `.env.example` existed in the monorepo (the old submodule remnants under `components/`
are stale). New file at the repo root documents the three required variables:
`ANTHROPIC_API_KEY`, `TELEGRAM_BOT_TOKEN`, `OPENAI_API_KEY`. The `.env` itself is
already gitignored.

### Makefile + docs/perimeter-test-handoff.md

- Makefile header: added a one-line WSL2 note for Windows operators.
- `perimeter-test-handoff.md §3`: expanded the Windows prerequisites block to include
  WSL2 as the required shell environment, how to navigate to the repo from WSL2, and
  the `.env.example` → `.env` copy step.

### What the audit confirmed does NOT need fixing

- All five Containerfiles are clean — no host-path dependencies, no Windows-incompatible
  constructs. The Linux-native features (`nftables`, `NET_ADMIN`, `sysctls`,
  `tmpfs noexec/nosuid`) run correctly inside the WSL2 Linux kernel.
- The `:U` podman volume flag on `vault-proxy-logs` is intentional (podman-only,
  already documented in compose.yml).
- `boundary-selftest.sh`, `proxy-memory-soak.sh`, and `red-team-breakout.sh` are clean
  — all Linux-specific operations run inside containers, not on the host.

---

## 2. The architecture question Albert wants to discuss with you

A co-architect reviewed the system and raised concerns about memory footprint.
Albert's position: *"if it can't run on the small linux laptop the project is a fail."*

Below is the full picture — measurements, the co-architect's analysis, and the
counterpoints this session surfaced. Albert wants to discuss the actual solutions with
the Linux agent.

### 2a. Actual measured memory (from footprint-and-device-usability.md)

The co-architect estimated 1–1.5 GB always-on. The measured reality is different:

| Container | Resting RSS (measured) | Active RSS |
|-----------|----------------------|------------|
| vault-agent | ~197 MB | ~600 MB |
| vault-proxy | ~150 MB | ~150 MB |
| vault-egress | ~50 MB | ~50 MB |
| vault-skills | ~5–20 MB | on-demand only |
| vault-social | ~5–20 MB | parked/on-demand |
| **Total resting** | **~400–430 MB** | — |
| **Total with idle auto-pause** | **~0 MB** | (perimeter is down) |

The 600 MB figure is vault-agent under active load (Node.js allocates while processing).
Idle it runs at ~197 MB. The 1.5 GB figure was never the resting state.

Critically: **idle auto-pause (Phase 3, ADR-0018) is already implemented and CI-green.**
When the agent is quiet for ~12 minutes the daemon brings the entire perimeter down.
On a real workload the perimeter is at ~0 MB the majority of the time.

### 2b. What actually caused the swap-storms on the Linux laptop

From the handoff (confirmed finding from a prior session):

> *"the earlier swap-storm was Cursor (~1.4 GB) + Brave + Claude, **NOT** the perimeter"*

The resting perimeter fits on the 7.2 GB laptop — the problem was running it simultaneously
with the full dev toolchain (Cursor + Brave + Claude Code + Slack). With memory discipline
(close the heavy tools first) the perimeter runs at ~400 MB resting. The lean-perimeter
roadmap (`docs/specs/2026-06-09-lean-verified-perimeter-roadmap.md`) already documents
this reframe and confirms it's feasible.

### 2c. The co-architect's analysis and this session's read on each option

The co-architect proposed five options. Here is each one with context from the codebase:

**Option 1 — Collapse sidecars into one container**

The co-architect suggests merging vault-egress, vault-skills, vault-social into one
`vault-services` container to share a single runtime.

*Context from the codebase:* The separation between vault-egress and vault-proxy is
ADR-0009's core security claim — vault-egress holds `NET_ADMIN` and internet access but
no secrets; vault-proxy holds the API keys but no internet access. Merging them collapses
the privilege separation that is the product's security differentiator. Whether this
trade-off is acceptable is an architecture decision for Albert to make, not a finding.

**Option 2 — Replace mitmproxy (Python) with a Go or Rust proxy**

vault-proxy at ~150 MB is mitmproxy running in Python. The L7 functions it performs
(domain allowlist, API-key injection, request logging, upstream chaining to vault-egress)
are well-defined and bounded. A single-binary Rust or Go HTTP proxy doing the same work
would be ~10–20 MB. This is the one component OpenTrApp fully controls.

*Context:* This is the highest-ROI code change available. It does not touch the security
architecture (container count, privilege separation, key isolation all stay the same). It
saves ~130 MB when the perimeter is active and reduces the attack surface of the
security-critical proxy container. The trade-off is implementation cost — mitmproxy
provides TLS interception, certificate pinning, and scripting for free; a replacement
would need to re-implement all of that.

**Option 3 — Replace Node.js (vault-agent) with Go or Rust**

vault-agent is OpenClaw (Claude Code). It is not OpenTrApp's code and cannot be replaced
without replacing the agent itself. The 197 MB idle / 600 MB active footprint is
effectively a fixed constraint.

**Option 4 — Use compose profiles for optional services**

Already implemented. vault-skills and vault-social already use `profiles: ["on-demand"]`
in compose.yml and do not start at boot.

**Option 5 — Multi-tenant plugin architecture (collapse skills/social into agent)**

This would load vault-skills and vault-social as plugins running inside vault-agent
rather than separate containers. The security concern: vault-skills handles untrusted
skill content (the whole reason it's isolated); running untrusted content scanners inside
the agent container removes the isolation. Whether the security trade-off is worth the
memory saving is an architecture decision.

### 2d. The one structural fact that frames all options

The always-on always-resident component is vault-proxy (Python/mitmproxy, ~150 MB). It
must be up before the agent starts so TLS interception is in place before the agent makes
its first request. vault-agent (Node.js, ~197 MB idle) is the other always-on resident.
Everything else is already on-demand or parked.

Replacing mitmproxy reduces the always-on baseline from ~400 MB to ~270 MB without
changing container count or security boundaries. Whether that 130 MB delta is the right
lever, and what implementation cost is acceptable, is the conversation for Albert and
the Linux agent to have.

---

## 3. Where things stand for the Linux agent to pick up

| Item | Status |
|------|--------|
| Portability fixes (compose.yml, memory-profile.sh, .env.example, docs) | ✅ Committed in this PR |
| T0 boundary self-test on real hardware | 🔶 Not run — Albert decided not to run on Windows workstation; architecture discussion first |
| T1/T2 idle auto-pause + wake verification | 🔶 Not run — same |
| Architecture refactor decision | 🔲 For Albert + Linux agent to decide |
| Handoff document (this file) | ✅ This file |

The Linux laptop can run the resting perimeter (~400 MB) with Cursor and Brave closed —
this was confirmed by the prior session's measurements. Whether to run it now or refactor
first is Albert's call.

---

## 4. Reading list for the Linux agent

In priority order for the architecture discussion:

| Document | Why |
|----------|-----|
| `docs/specs/2026-06-09-lean-verified-perimeter-roadmap.md` | The measured footprint numbers + the WS0–WS4 plan; confirms the "swap-storm was Cursor not the perimeter" finding |
| `docs/adr/0009-five-container-perimeter.md` | Why vault-egress and vault-proxy are separate (the L7/L3 split rationale) |
| `docs/footprint-and-device-usability.md` | The live measured memory breakdown |
| `docs/adr/0018-idle-auto-pause-host-waker.md` | What idle auto-pause does and why it's the main memory lever |
| `docs/handoff.md` (top section) | Full session state including the ADR-0019→0023 architecture pivot |
| `infra/proxy/vault-proxy.py` | What mitmproxy actually does — the L7 functions any replacement must replicate |

---

## 5. Decision (Linux agent + Albert, 2026-06-15)

The architecture discussion is resolved: **verify-first; the lean Rust proxy is tracked (WS5), not
built yet.**

**Premise corrected — the project is not a fail.** The measured resting perimeter (~400 MB; ~0 MB with
idle auto-pause) fits the 7.2 GB laptop. The swap-storm culprit was independently re-confirmed on the
laptop this session: **Brave 3.0 GB + Cursor 1.7 GB + Claude 0.9 GB** — the perimeter was never the
consumer. The "runs on the small laptop" bar is met with the heavy tools closed (as the footprint doc
already expects).

**Memory is not the refactor driver.** No proxy swap closes the GB-scale gap with the dev tools (Brave
alone is 3 GB). 400→270 MB changes nothing operationally — you close Brave/Cursor either way. Footprint
therefore does not justify re-architecting a security boundary.

**Options resolved:**
- **#3** (replace the Node agent) — impossible; it is OpenClaw. **#4** (compose profiles) — already shipped.
- **#1 & #5** (collapse sidecars / plug-in the scanners) — **REJECTED.** They trade away ADR-0009's
  privilege separation (egress holds `NET_ADMIN` + internet but no secrets; proxy holds the keys but no
  internet), which is the product's security differentiator and the core of the opencode pitch — not
  negotiable for memory the project does not need to save.
- **#2 (mitmproxy → lean Rust proxy)** — **TRACKED as WS5 (task #69)**, justified by *attack-surface
  reduction + Rust-core alignment, NOT memory*. The L7 policy is ~388 portable lines; the hard,
  security-critical part is the MITM TLS engine mitmproxy provides for free (cert-gen under the proxy CA,
  CONNECT, HTTP/2). A real project, not a quick win, and it must **not** block the opencode mission.

**Sequencing (§11 — the one place we invert the handoff's "rethink before testing"):**
1. **T0 first.** Run the boundary self-test cold + resumed on the laptop (heavy tools closed) → a GREEN
   baseline proves both "it runs on the small laptop" *and* "the boundary holds." You cannot safely
   re-architect a boundary you have not verified holds.
2. **The green T0 becomes the regression gate** any WS5 rewrite must re-pass (B2 allowlist, B3
   credential-injection, B5 CA-pin), with `test_vault_proxy.py` coverage preserved.
3. **The mission keeps moving** — the opencode pitch's boundary claim upgrades from "built" to
   "verified" after T0; de-Tauri proceeds. WS5 is a non-blocking attack-surface initiative behind the
   baseline, with its own ADR when we commit to build.

Net: **don't optimize what already fits — verify it, then reduce the proxy's attack surface deliberately.**
