# OpenTrApp, explained in one page

> Audience: a maintainer, an opencode engineer, or an OpenSSF reviewer who has 60 seconds.
> Full version: [`docs/trifecta.md`](trifecta.md). The decisions: [ADR-0024](adr/0024-product-structure-three-concerns.md)
> (this structure), [ADR-0009](adr/0009-five-container-perimeter.md) (Guard 1), and
> [ADR-0003](adr/0003-content-disarm-reconstruction.md) (Guard 2).

## The whole idea, in two sentences

OpenTrApp runs an autonomous CLI coding agent you don't fully trust, and puts **two guards** around
it. **Guard 1 (egress):** no single box ever holds both your API keys *and* internet access.
**Guard 2 (supply chain):** every skill the agent would load is scanned and rebuilt in a box the
agent *can't touch*, then handed over read-only.

That's it: **one contained agent, two guards.** Everything below is detail.

## The two guards = two ideas

**Guard 1, egress ([privilege separation](adr/0009-five-container-perimeter.md)).** One job
split across two boxes so neither is worth compromising alone:

- `vault-proxy` holds the live API keys and injects them into requests, but has **no route to the
  internet**.
- `vault-egress` is the **only** box that reaches the internet (and the only one with `NET_ADMIN`),
  but holds **no secrets** and runs no application code (just an nftables ruleset and a DoT
  resolver, unbound).
- To steal a key *and* exfiltrate it, an attacker must break **both**, and they are separate boxes.

**Guard 2, the skill firewall ([supply-chain defense](adr/0003-content-disarm-reconstruction.md)).**
Every skill is pattern-scanned and **Content-Disarm-Reconstructed** (parsed to
intent, the original discarded, a clean copy rebuilt) inside `vault-skills`, a box the agent has
**no path to**, then delivered **read-only**. The agent receives a clean reconstruction; the
original bytes never reach it. *The agent is the thing being checked, not the thing doing the
checking.*

> **Why a separate box and not "inside the agent"?** Because an inspector the agent can tamper with
> is not an inspector. A compromised agent sharing the scanner's container could rewrite the pattern
> catalogue, forge its own clearance report, and read the un-disarmed bytes. The isolation *is* the
> defense. ([ADR-0024](adr/0024-product-structure-three-concerns.md), §3.)

## One brand, three concerns, run what you need

OpenTrApp is **one product** organized as **three concerns + an optional GUI**, each
addressable on its own. It ships **today as a desktop app**; the CLI control plane is the
direction, not yet shipped.

| Concern | What it is |
|---|---|
| **Vault** (containerization) | The perimeter itself: the contained agent plus the egress guard. *"The Vault" is the whole containment, **not** the agent's box.* |
| **Skill** (supply chain) | The skill firewall (scan + CDR). Runs inside the Vault, **and standalone** as a pre-install check; no perimeter required. |
| **Social** (agent-social) | An opt-in shield for untrusted agent-social feeds (a second instance of the Guard-2 vetting pattern). A read-only AT Protocol adapter exists and was smoke-tested once ([ADR-0017](adr/0017-unpark-social-live-adapter.md)); no shipped command runs it (defaults use canned data), and full build-out is deferred. |
| **GUI** (optional) | A disposable projection over the same concerns. Today it is how you drive the perimeter; it is never the security boundary. |

You don't have to run the whole bundle: the skill firewall alone vets a skill; the Vault alone
contains an agent.

## The containers (the detail, not the headline)

The Vault is realized as a small set of containers: three run while an agent is active; the skill
firewall is an **on-demand** job; Social is opt-in.

| Container | Role | Secrets? | Internet? | Caps |
|---|---|---|---|---|
| `vault-agent` | the contained **agent** (agent runtime) | no | only via the proxy | all dropped |
| `vault-proxy` | **Guard 1, L7**: allowlist + key injection | **yes** (keys) | **no** (chains upstream) | unprivileged |
| `vault-egress` | **Guard 1, L3**: kernel RFC1918 drop, DoT resolver | no | **yes, the only one** | `NET_ADMIN` only |
| `vault-skills` | **Guard 2**: scan + CDR (on-demand) | no | only via the proxy | all dropped |
| `vault-social` | Social shield (opt-in) | no | only via the proxy | all dropped |

Each workload sits on its own `internal: true` network; only `vault-proxy` bridges them, and only
`vault-egress` touches the internet. The agent-to-skills channel is a **one-way, read-only volume**
(`skills-deliveries`): there is no network path between them. (Source of truth:
[`compose.yml`](../compose.yml); diagrams in [`docs/diagrams.md`](diagrams.md).)

```
  the Vault (= the perimeter)
  ┌────────────────────────────────────────────────────────────────┐
  │  vault-agent ─┐                                                  │
  │  vault-skills ┼─► vault-proxy ──► vault-egress ──► internet      │
  │  (Guard 2)    │   GUARD 1: keys,   GUARD 1: net,                 │
  │               │   no internet      no keys, NET_ADMIN            │
  │               │                                                  │
  │   Guard 2 delivers the cleaned skill to the agent                │
  │   one-way, READ-ONLY: no network path between them               │
  └────────────────────────────────────────────────────────────────┘
   vault-social: opt-in, a second Guard-2 instance (agent-social feeds)
```

## Why this is defensible (and what we deliberately don't do)

- **No box holds both keys and internet** (Guard 1). **No box holds both elevated capabilities and
  application code**: only `vault-egress` has `NET_ADMIN`, and it runs nothing but the kernel
  ruleset and resolver.
- **The agent can't influence the scanner, and the scanner can't reach the agent**, except via the
  one-way, read-only delivery volume (Guard 2).
- **We do not collapse the guards into the agent** to save a container; that would hand a
  compromised agent the keys, the internet, or the scanner it is supposed to be checked by. Memory
  is not a reason to weaken a boundary (the resting perimeter is ~400 MB; ~0 when idle-auto-paused).
  See [ADR-0024](adr/0024-product-structure-three-concerns.md) and the
  [verify-first decision](specs/2026-06-15-windows-session-portability-and-architecture-review.md) §5.

> **Agent-agnostic by design.** The guards don't care which agent sits inside. The OpenClaw recipe
> adds a Telegram gateway; an opencode recipe would wrap a terminal session. The guards are the same,
> which is what makes OpenTrApp recommendable beyond any one agent.
