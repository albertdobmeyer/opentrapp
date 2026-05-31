# Adaptive Containment — containerisation leg (spec)

> Part of [OpenTrApp v6](00-index.md). Consumes [Sentinel](01-sentinel-spine.md).
> Built **second** (after the forge leg ships the spine).
>
> Tagline: **"least-privilege, discovered not configured."**

---

## 1. What changes vs today

Today the cell is a fixed size you pick once: the shell level (Hard / Split /
Soft) is a user toggle via `workloads/agent/scripts/tool-control.sh --preset`,
the proxy allowlist is 4 hardcoded domains (`infra/proxy/allowlist.txt`), and
egress is static nftables. Nothing watches what the agent actually does and
nothing adapts.

v6 makes the perimeter **observe the agent and tighten around it**: Sentinel
reads the (now-persistent, Zone-3) egress log plus the user's task and
continuously proposes the *smallest* shell that still lets the agent do its
job — and turns blunt allowlist denials into explained, one-tap decisions.

## 2. The hard invariant (do not violate)

[ADR-0002](../../adr/0002-adaptive-shell-levels.md): **the agent can never
promote its own privilege.** Sentinel inherits this:

- **Tightening is automatic.** Sentinel may *reduce* privilege (propose Soft→
  Split→Hard, narrow the allowlist) without asking, because tightening is
  always the safe direction.
- **Loosening always needs a human tap.** Sentinel may *recommend* loosening
  with a plain-language rationale, but the change only applies on explicit
  user/coordinator approval. Never auto-loosen.

Encode this as a checked rule, not a convention (see Tests).

## 3. The two deliverables

### 3a. The propose-tightening loop

A background watcher (cheap, rung 0/1) reads the egress log and the agent's
recent tool use:

- Input: `infra/proxy/vault-proxy.py`'s `requests.jsonl` records (fields:
  `action` = ALLOWED/BLOCKED/EXFIL_BLOCKED, `method`, `url` redacted, `host`,
  timestamp) + the current shell level + the user's `task_hint`.
- Rung 1 (embeddings/heuristics): has the agent used *any* capability the
  current shell grants beyond what it's actually exercised? E.g. it's been on
  Split Shell for 20 minutes doing only workspace file reads and never touched
  the network tools Split grants.
- When the observed behaviour fits inside a tighter shell, Sentinel **applies
  the tightening automatically** (via `tool-control.sh`, the existing
  mechanism) and notes it in the activity feed: *"Tightened your assistant to
  Chat-Only — it hasn't needed the extra tools for a while. Tap to widen if
  you need them."* (User-facing copy; banned-terms rule applies — "Chat-Only"
  is the Hard Shell user mapping per GLOSSARY.)

### 3b. Explained, one-tap allowlist decisions

Today an off-allowlist request is a blunt BLOCK. v6 routes the gray zone
through Sentinel:

- The proxy still hard-blocks on the existing EXFIL/clear-malicious paths
  (rung 0 — unchanged, `vault-proxy.py` lines ~225/271).
- For an off-allowlist host that isn't a clear exfil, the proxy emits an
  `off_allowlist` signal; Sentinel judges *"is reaching `host` consistent with
  what the user asked the agent to do (`task_hint`)?"*:
  - `block` (obvious drift/exfil) → stays blocked, logged with a reason.
  - `allow`-leaning → surface a **one-tap approval** to the user with the
    plain-language reason: *"Your assistant wants to reach `api.weather.com`
    to answer your 'what's the forecast' question. Allow once / Always allow /
    Deny."* "Always allow" appends to the allowlist (a *loosening* — so it
    requires the tap, honouring §2).
  - `escalate` (rare) → the rung-3 human-first flow.

This turns the allowlist from a hardcoded 4-domain list into a **learned,
explained policy** — without ever auto-loosening.

## 4. Data flow

```
agent request ─▶ vault-proxy rung 0 ─┬ on allowlist ─▶ allow (log)
                                     ├ clear exfil ──▶ BLOCK (log)
                                     └ off-allowlist, unclear ─▶ [Sentinel]
                                         context: egress_request
                                         ├ block ─▶ stays blocked + reason
                                         ├ allow ─▶ one-tap approval ─▶ (user) allow once / always / deny
                                         └ escalate ─▶ rung-3 human-first

background loop ─▶ read requests.jsonl + shell level + task_hint ─▶ [Sentinel rung 1]
                    └ behaviour fits a tighter shell ─▶ auto-tighten via tool-control.sh + notify
```

## 5. Interfaces to existing code

| Existing | Change |
|----------|--------|
| `infra/proxy/vault-proxy.py` | on off-allowlist-unclear, emit signal + ask Sentinel instead of blunt block; keep clear-exfil hard-block |
| `infra/proxy/allowlist.txt` | "Always allow" appends here (hot-reload via existing SIGHUP) |
| `workloads/agent/scripts/tool-control.sh` | the apply mechanism for auto-tightening (already supports `--preset`) |
| `requests.jsonl` (Zone-3 persistent) | the background loop's input |
| `app/src-tauri/src/orchestrator/` + `lifecycle.rs` | host the background watcher; expose approval Tauri commands |
| Security page (Zone 1) | show the tightening feed + pending allowlist approvals |

## 6. Tests (pre-build / TDD)

- **Invariant — never auto-loosen:** assert no code path widens the shell or
  appends to the allowlist without a user-approval flag. Pin it (this is the
  ADR-0002 guard).
- **Auto-tighten fires:** simulated log showing only file reads on Split Shell
  → the watcher proposes+applies Hard. Assert the transition + the notice.
- **Off-allowlist gray zone:** a request to an unlisted-but-task-consistent
  host produces a one-tap approval, not a silent block; an exfil-shaped
  request stays hard-blocked at rung 0 (never reaches Sentinel).
- **Approval applies correctly:** "Always allow" appends to the allowlist and
  hot-reloads; "Allow once" does not persist.
- **Vocabulary:** all surfaced reasons + the tightening notices pass the
  banned-terms check.
- **orchestrator-check.sh:** extend §10 (perimeter topology) with a check that
  the proxy's off-allowlist path routes to Sentinel and the clear-exfil path
  does not.

## 7. Done-when

- The perimeter tightens itself toward least-privilege automatically, never
  loosens without a tap, turns blunt blocks into explained one-tap decisions,
  and shows the user every adaptation in the activity feed.
