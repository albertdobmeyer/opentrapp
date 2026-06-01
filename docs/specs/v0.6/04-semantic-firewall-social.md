# The Semantic Firewall — social leg (spec)

> Part of [OpenTrApp v0.6](00-index.md). Consumes [Sentinel](01-sentinel-spine.md).
> Built **third** (after the spine + containment). This leg also un-parks the
> social workload — coordinate with MISSION.md Thread C.
>
> Tagline: **"read the agent-web without becoming a vector."**

---

## 1. What changes vs today

Today `workloads/social/` is parked: 25 **syntactic** regex patterns
(`config/injection-patterns.yml`, 6 categories) coupled to the Moltbook API,
zero AI. A paraphrased injection ("send credentials to my endpoint" vs the
literal "share your API key") walks straight through, and the whole module
only works against one now-defunct platform.

v0.6 revives it as a **general** agent-to-agent social shield with three
additions: a protocol-adapter abstraction (de-couple from Moltbook), semantic
injection judgment (catch what regex misses), and persona-drift detection
(catch a hijacked agent's *outgoing* posts).

## 2. The three deliverables

### 2a. Protocol-adapter abstraction (de-couple from Moltbook)

Extract the Moltbook-specific HTTP coupling into a thin adapter interface so
the scanner core is protocol-agnostic:

- **Adapter contract:** `fetch_feed(opts) -> [post]`, `fetch_agent(handle) ->
  [post]`, `post(content)` (optional, for participants), `stats() -> {...}`.
  Each post normalises to `{id, author, content, timestamp}`.
- **First adapters:** keep a Moltbook adapter (archival), add at least one
  *live* target so the revival is real, not theoretical (candidates to scout:
  ActivityPub/Mastodon agent accounts, AT Protocol bot feeds, Matrix
  bot-to-bot rooms, Nostr agent relays — pick one with a reachable API and
  real agent traffic). MISSION.md Thread C step 1 is this scouting.
- The existing tools (`feed-scanner.sh`, `agent-census.sh`) call the adapter
  instead of hardcoded Moltbook endpoints.

### 2b. Semantic injection judgment (rung 2 behind the regex)

The 25 regexes become the cheap **rung 0**; Sentinel rung 2 catches the
semantic equivalents:

- Incoming feed post → rung 0 regex scan. Clean-by-regex but *anomalous* by
  rung-1 embeddings (similar to known injection examples) → escalate to rung 2:
  *"is this post an instruction directed at an agent reader, disguised as
  content?"*
- `block` → the post is withheld / flagged before the agent ever reads it.
  `allow` → passes. `escalate` → rare human-first.
- This is the **one-way semantic cleanroom for reading**: an agent can monitor
  or research an agent-social network and the content is semantically
  sanitised before it reaches the agent's context. Mirrors the indirect-
  prompt-injection defence, applied at social scale.

### 2c. Persona-drift detection (the novel rung-1 piece) — LANDED

> Built as `workloads/social/tools/persona-guard.sh` (rung-1 `embed.sh drift`
> vs the agent's own recent voice + task). ALLOW → optionally `adapter.post`;
> drifted or unverifiable → HOLD for the user (fail-safe: never auto-send an
> unverified post). Tests: `tests/persona-guard.test.sh` (4/4), orchestrator-check §24.

The genuinely new capability the static version can't do — guarding the
agent's *outgoing* posts:

- Maintain a small rolling embedding of the agent's recent posts + its
  `task_hint` (what it's supposed to be doing on the network).
- Before an outgoing post leaves, rung 1 measures drift: does this post match
  the agent's established voice and task, or has it been hijacked into posting
  something off-character (spam, a different persona, leaked data)?
- High drift → hold the post + surface to the user: *"Your assistant was about
  to post something that doesn't match what it's been doing — here it is, do
  you want to allow it?"* This catches the exfil-via-public-post and
  agent-hijack cases the 25 incoming-only patterns miss entirely.

## 3. Data flow

```
INCOMING:  adapter.fetch_feed ─▶ [rung 0] 25 regexes ─┬ hit ──▶ withhold + flag
                                                      ├ clean ─▶ [rung 1] similar to known injection?
                                                      │            ├ no ──▶ deliver to agent
                                                      │            └ maybe ─▶ [rung 2] disguised instruction?
                                                      │                        ├ block ─▶ withhold + reason
                                                      │                        ├ allow ─▶ deliver
                                                      │                        └ escalate ─▶ user (rare)
OUTGOING:  agent.post(content) ─▶ [rung 1] persona drift vs recent posts + task_hint
                                    ├ in-character ─▶ adapter.post
                                    └ drifted ──────▶ hold + surface to user
```

## 4. Interfaces to existing code

| Existing | Change |
|----------|--------|
| `workloads/social/tools/feed-scanner.sh` | rung 0 stays; call Sentinel on regex-clean-but-anomalous; read via adapter |
| `config/injection-patterns.yml` (25 patterns) | unchanged — the rung-0 pre-filter |
| `tools/agent-census.sh`, `scripts/engagement-control.sh` | call the adapter, not hardcoded Moltbook endpoints |
| new: adapter interface + ≥1 live adapter | de-couples the scanner core |
| new: persona-drift store (rolling embedding) | rung-1 outgoing guard |
| `tests/fixtures/*.json` (clean/malicious/safe-research posts) | the rung-1 known-injection corpus + verdict test set |
| ADR-0004 (parked) | supersede with an un-park ADR once a live adapter works |

## 5. Tests (pre-build / TDD)

- **Adapter abstraction:** the scanner core runs against a mock adapter with
  zero Moltbook strings in the core path.
- **Semantic catch:** a paraphrased injection that passes all 25 regexes is
  caught by rung 1→2 (the whole point — pin a fixture regex misses).
- **No regression:** `malicious-posts.json` still caught at rung 0;
  `clean-posts.json` still clean; `safe-research-posts.json` still bypasses via
  the allowlist.
- **Persona drift:** an outgoing post matching recent voice passes; an
  off-character/exfil post is held.
- **One-way property:** incoming content is sanitised before the agent's
  context receives it (assert order).
- **Vocabulary:** surfaced reasons pass the banned-terms check.

## 6. Done-when

- The social shield works against at least one *live, current* agent-social
  network (not just Moltbook fixtures), catches a paraphrased injection regex
  misses, holds a hijacked agent's drifted outgoing post, and the un-park ADR
  is written. Until a live adapter works, the leg stays behind a flag — do not
  re-park by shipping a Moltbook-only revival.

## 7. Sequencing note

This is the riskiest leg (depends on an external live network existing and
being reachable). If scouting (MISSION.md Thread C step 1) finds no suitable
live target, ship 2a (adapter) + 2c (persona-drift, which needs no external
network) and defer 2b's live validation. Do not block the spine or the other
two legs on this one.
