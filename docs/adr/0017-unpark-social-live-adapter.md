# ADR-0017 — Un-park the social shield behind a live protocol adapter

**Status:** Accepted — v0.6 Item C shipped (AT Protocol adapter + live validation)
**Supersedes:** [ADR-0004](0004-parking-moltbook-pioneer.md) (Parking openagent-social)
**Companion spec:** [`docs/specs/v0.6/04-semantic-firewall-social.md`](../specs/v0.6/04-semantic-firewall-social.md)
**Cross-references:** [ADR-0009](0009-five-container-perimeter.md) · [ADR-0015](0015-local-ai-judgment-layer.md)

---

## Context

[ADR-0004](0004-parking-moltbook-pioneer.md) parked the social workload on
2026-05-03: Moltbook (its only target) was acquired and its API went defunct
(2026-04-05), so the shield had nothing live to defend against. M4 (`dc5fb76`)
then de-coupled the shield from Moltbook into a **protocol-adapter contract**
(`fetch_feed`/`fetch_agent`/`post`/`stats`/`name`, normalised
`{id,author,content,timestamp}`), leaving the core protocol-agnostic with three
non-live adapters (`file`, `mock`, `moltbook`-archival). The shield could be
exercised against fixtures but had no live network to prove it.

The "AI makes AI safe" reassessment (v0.6) gives the shield real teeth — the
semantic firewall (rung-2 judge catching paraphrased injection the 25 regexes
miss) and persona-drift (rung-1, guarding outgoing posts). Those are worth
nothing parked. The blocker was a single live adapter.

## Decision

Un-park the social shield **behind a flag**, on the first live adapter:
**AT Protocol (atproto / Bluesky)** (SD-C1).

- `tools/lib/adapters/atproto.sh` implements the existing contract against AT
  Protocol XRPC. **Reads use the public AppView** (`public.api.bsky.app`) — no
  auth, read-only — which is exactly the posture a feed shield wants. Posting
  requires explicit app-password credentials (`ATPROTO_HANDLE` +
  `ATPROTO_APP_PASSWORD`) and is never implicit.
- The incoming `semantic-firewall.sh` fetches through the adapter (`--adapter
  atproto --actor <handle>` / `--feed <at-uri>`) and runs the **same** rung-0 +
  rung-2 pipeline. Outgoing `persona-guard.sh` already drives the adapter
  (`fetch_agent` baseline + `post` on ALLOW + `--send`).
- **The default stays `file`.** The live legs are opt-in; the perimeter does not
  auto-participate in any network. This is the un-park, not an auto-enable.

Validated live (read-only) against a real handle: a `getAuthorFeed` read
returned 50 real posts in the canonical shape, and `getProfile`-derived stats
returned real counts — the adapter genuinely works against the live network.

## Consequences

- The shield is real again, on an open, well-documented, actively-used protocol
  with real agent/bot traffic — a far better fit than the defunct Moltbook.
- **Residual:** live validation depends on a reachable public AppView and a
  chosen handle; CI stays offline (the committed tests are network-free; the live
  read is a host-only smoke). Posting is a write surface gated behind explicit
  credentials and the persona-drift hold — never automatic.
- Mastodon/ActivityPub, Nostr, Matrix remain future adapters behind the **same**
  contract; adding one is a new `adapters/<name>.sh`, no core change.
- ADR-0004's parked posture is superseded; the workload is active behind the
  flag. `vault-social` remains in the perimeter (ADR-0009) and processes all
  feed content in-container, never on the host.
