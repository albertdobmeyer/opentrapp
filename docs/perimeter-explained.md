# The five-container perimeter, explained in one page

> Audience: someone (a maintainer, an opencode engineer, an OpenSSF reviewer) who has 60
> seconds and wants the architecture to make sense. For the full version, see
> [`docs/trifecta.md`](trifecta.md) and [ADR-0009](adr/0009-five-container-perimeter.md).

## TL;DR

Five containers, **all running in parallel** as one compose stack. Each is a *boundary*,
not a layer wrapping another. They are statically composed (every bring-up starts all of
them), not dynamic or progressive. After [ADR-0013](adr/0013-monorepo-consolidation.md)
the repository is a single monorepo with three workload directories and two infra
directories — no more submodules.

## How we got to five (the honest history)

| Stage | Count | Why we added the boundary |
|------:|------:|---------------------------|
| v0    | **1** | One container around the whole agent. Smallest possible blast radius for "agent escapes its sandbox." |
| v1    | **3** | One container per *area of concern* — the agent runtime, the skill scanner, the social-feed analyser. Compromise of any one cannot reach the others' state. |
| v2    | **4** | Add `vault-proxy`. The three workload containers should not hold API credentials or talk to the internet directly. Pull both responsibilities into a dedicated gateway. ([ADR-0006](adr/0006-four-container-topology.md)) |
| v3    | **5** | Split `vault-proxy` in two. **L7 policy** (domain allowlist, key injection, TLS interception — credential-bearing) and **L3 policy** (kernel-level RFC1918/loopback drop, pinned DNS — privilege-bearing) are structurally different responsibilities. The container that holds live API keys should not also hold `NET_ADMIN`. ([ADR-0009](adr/0009-five-container-perimeter.md)) |

The rule used at every step: *a boundary exists when the responsibilities on either side
of it have structurally different roles (different secrets, different capabilities,
different attack surfaces).* The count grew because we found two responsibilities living
in one container that shouldn't be.

## The five containers, in one table

| # | Container | Owns | Directory | Capabilities | Holds secrets? | Reaches internet? |
|--:|-----------|------|-----------|--------------|----------------|-------------------|
| 1 | `vault-agent`  | Agent runtime + Telegram gateway + loaded skills | `workloads/agent/`  | all dropped | no | only via vault-proxy |
| 2 | `vault-forge`  | Skill scanner + CDR pipeline                     | `workloads/forge/`  | all dropped | no | only via vault-proxy |
| 3 | `vault-social` | Agent-to-agent social-feed analyser *(parked)*   | `workloads/social/` | all dropped | no | only via vault-proxy |
| 4 | `vault-proxy`  | **L7 policy** — allowlist, API-key injection, TLS interception, payload limits, response redaction | `infra/proxy/`  | unprivileged | **yes** (live API keys) | **no** — chains upstream |
| 5 | `vault-egress` | **L3 policy** — kernel RFC1918/loopback drop, pinned DNS, masquerade | `infra/egress/` | `NET_ADMIN` only | no | **yes — only container with `external-net`** |

## The flow, drawn

```
  agent-net ─┐
  forge-net ─┼──→ vault-proxy ──→ egress-net ──→ vault-egress ──→ external-net
social-net ─┘    (L7 policy:                     (L3 policy:
                   credentials,                    NET_ADMIN,
                   allowlist,                      kernel drop,
                   TLS MITM)                       pinned DNS)
```

Each of the three workload containers (agent / forge / social) sits on its own *internal*
network with no default gateway. They can reach `vault-proxy` and nothing else — not each
other, not the host, not the internet. `vault-proxy` bridges the three internal networks
and chains upstream to `vault-egress`. `vault-egress` is the **only** container attached
to `external-net`, and it's the only one with `NET_ADMIN`.

## Frequently asked

**Q: Are containers 4 and 5 *around* the others (nested), or *parallel* to them?**
Parallel. All five run side-by-side as services in one compose stack. The relationship is
"who can talk to whom" (network attachments), not "who wraps whom."

**Q: How does the directory layout map to the containers?**
3 workloads + 2 infra + 1 orchestrator. `workloads/{agent,forge,social}/` each builds
exactly one container. `infra/{proxy,egress}/` each builds exactly one container. `app/`
is the Tauri orchestrator that composes them. The directory name matches the container
name; there is no indirection. (See [ADR-0013](adr/0013-monorepo-consolidation.md) for why
this replaced the earlier three-submodule layout.)

**Q: Are they dynamic / progressive — spun up as needed?**
No. They are statically composed; every perimeter bring-up starts all five. (`vault-social`
is "parked" — its image still builds and the service is defined, but it's not actively
exercised. See [ADR-0004](adr/0004-parking-moltbook-pioneer.md). Thread C of MISSION.md
plans to unpark it as a generalized agent-social shield.)

**Q: Why isn't the agent's own container considered enough? Isn't one container the simplest?**
Because compromise of *any* of the four other responsibilities (skill scan, social-feed
parse, credential handling, network egress) inside the agent's container would let an
attacker pivot through all of them at once. The whole point of the perimeter is that the
credential-bearing container has no internet, the internet-bearing container has no code,
the skill scanner can't influence the agent, and the agent can't influence the scanner.
One container cannot offer those properties; five can.

## Why this design is defensible

- **No single container holds both API credentials and internet access.** vault-proxy
  holds the keys but cannot reach the public internet. vault-egress reaches the internet
  but holds no secrets and runs no application code.
- **No single container holds both elevated capabilities and application code.** Only
  vault-egress has `NET_ADMIN`; it runs nftables and a DNS forwarder — nothing else.
  The four other containers drop all capabilities.
- **Three independent egress defenses.** (1) L7 domain allowlist in vault-proxy,
  (2) post-resolve IP check in vault-proxy, (3) kernel RFC1918 drop in vault-egress. A
  bug in any single layer does not produce the vulnerability. See ADR-0009 §Consequences.
- **The three workload containers cannot reach each other.** Agent, forge, and social
  are on separate internal networks with no path between them. Compromise of one cannot
  pivot to another.

## When to question this design

Honestly: if Tier 3/4 (kernel drop in vault-egress) ever proves operationally fragile,
collapsing back to four containers (with the kernel filter co-located inside vault-proxy)
is a documented alternative — explicitly rejected today, but the rejection has knobs. The
five-container framing is a *public commitment* (README hero, whitepaper, landing page),
so reverting costs another ADR and a doc sweep. We do not revert lightly. But we are not
locked in by ego, only by the cost of re-explaining.
