# What OpenTrApp protects against — and what it doesn't

*Plain-language summary. The authoritative, fully-enumerated version is the
[threat model](threat-model.md); the comparison against other approaches is
[why not X](why-not-x.md). This page is the front door — read it before you
trust your machine to an autonomous agent.*

**The one honest sentence:** OpenTrApp **cannot make running an autonomous agent
absolutely safe.** Nothing can. What it does is raise the *cost* of a compromise
by wrapping the agent in layered containment, and it is open about the gaps that
remain. If a page ever tells you it's "safe," distrust the page. This one tells
you where the walls are — and where the doors are.

---

## ✅ What it protects against

These are the threats the perimeter is *engineered* to contain. Each maps to a
category in the [threat model](threat-model.md).

- **A prompt-injected agent trying to phone home or exfiltrate your data** (T1).
  The agent lives on an isolated container network with **no direct route to the
  internet** — every outbound request must pass an application-layer allowlist
  (only known-good hosts) and a kernel-level egress filter. An off-allowlist
  destination is blocked, not warned about.
- **Your API key being stolen by the agent** (T1). The expensive vendor
  credential is **injected by the proxy and never exists inside the agent's
  container** — a compromised agent has nothing to steal.
- **A malicious skill you install** (T2). Roughly 1 in 8 published skills in the
  wild carry a payload. Every skill is scanned against a pattern catalogue and
  **rebuilt from scratch** (Content Disarm & Reconstruction) before the agent can
  load it — the original, possibly-booby-trapped file is discarded.
- **The agent reading files outside its workspace** (T1/T5). It runs read-only,
  with no host filesystem mounts, confined to a workspace volume.
- **A network eavesdropper on your outbound traffic** (T3). All upstream calls are
  TLS-terminated inside the perimeter, and every request is logged so you can see
  what the agent did.
- **Untrusted content touching your real computer** (T6). Skill downloads, scans,
  and feed processing all happen *inside containers* — never on the host filesystem.

---

## ⚠️ What it does NOT protect against

Equal weight, on purpose. These are real and we name them rather than bury them.

- **A computer that's already compromised** (T4 — *explicitly out of scope*). If
  malware or a rootkit is already on your machine, no container boundary can
  recover security. The architecture *assumes the host is honest.*
- **You** (T5). The agent asks for approval on sensitive actions, but if you click
  "Allow" without reading, you approved it. The user is part of the security
  boundary — by design, you are the only one who can loosen it.
- **The agent's own reasoning being fooled.** Prompt-injection defense *inside* the
  language model is the vendor's job, not ours. We assume the reasoning *will*
  eventually be turned, and build the cage around that assumption — we don't
  prevent it.
- **A stolen Telegram account.** Whoever controls the paired Telegram account
  controls the agent. **Use a dedicated account with two-factor**, not your
  personal one.
- **A few known, documented gaps**, tracked openly in the threat model:
  - **Certificate pinning** for the upstream API isn't enforced yet — a compromised
    CA in your host's trust store could intercept upstream of the proxy (which is
    itself a host-compromise, T4).
  - **DNS rebinding** against an allowlisted domain is a *partial* residual risk;
    the kernel egress filter and pinned DNS resolver narrow it but it is not fully
    closed. See the threat model T3 notes.
  - **Installers are signed with the updater key, not OS-level code-signing** yet,
    so macOS Gatekeeper / Windows SmartScreen show a first-launch warning.
  - **Container destruction doesn't guarantee complete cleanup** — activity
    metadata and image caches persist until `podman system prune -a`.

---

## How to raise your own assurance

The perimeter is defense-in-depth, not a guarantee. If your threat model is
higher than the default, stack these on top — they're cheap and they work:

1. **Run it in a disposable VM** with a **disposable API key** and a **hard
   spending cap.** This is the strongest single step against host compromise (T4)
   and budget abuse — it turns "my computer" into "a computer I can throw away."
2. **Pair a dedicated Telegram account** with two-factor, never your personal one.
3. **Keep the agent at the default (least-powerful) shell level** unless you have a
   specific reason to widen it. More power = more blast radius if it's subverted.
4. **Treat the proxy request log as sensitive** if *who you talk to* is private to
   you — it records activity timing and destinations.

---

## The bottom line

This is **experimental software, provided as-is, without warranty.** Autonomous-agent
containment is an open research problem. OpenTrApp is a serious, layered, openly-documented
attempt at it — and it is honest that the residual risk is real. That honesty *is* the
credibility: a defense-in-depth tool that overclaims has already failed the first test.

**Read next:** [full threat model](threat-model.md) · [why not Firejail / gVisor / a VM](why-not-x.md)
· [security & reporting policy](../SECURITY.md) · [the architecture](trifecta.md)
