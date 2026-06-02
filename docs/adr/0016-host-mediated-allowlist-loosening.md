# ADR-0016 — Host-mediated allowlist loosening

**Status:** Accepted — v0.6 Item A shipped (egress allowlist approvals)
**Companion spec:** [`docs/specs/v0.6/08-completion-plan.md`](../specs/v0.6/08-completion-plan.md) §4
**Cross-references:** [ADR-0001](0001-proxy-side-api-key-injection.md) · [ADR-0002](0002-adaptive-shell-levels.md) · [ADR-0009](0009-five-container-perimeter.md) · [ADR-0011](0011-zero-trust-self-sufficient-bootstrap.md) · [ADR-0015](0015-local-ai-judgment-layer.md)

---

## Context

Through v0.5.0 an off-allowlist request from the agent was a blunt 403 in
`vault-proxy` (`requests.jsonl` → `{"action":"BLOCKED","reason":"domain not in
allowlist"}`). The only way to allow a new destination was to hand-edit
`allowlist.txt` in the source tree — invisible to the end user, and impossible
from the packaged app. Adaptive Containment (the v0.6 reassessment) calls for the
opposite end of the loop: when the agent is blocked reaching a site it plausibly
needs for the user's task, the user should be able to make an **informed one-tap
decision**, with the on-device judge (ADR-0015) explaining the trade-off.

This is the **only new write/loosening surface in v0.6**, so it must be built so
the [ADR-0002](0002-adaptive-shell-levels.md) invariant holds *by construction*:
**only the human can loosen the perimeter; the agent never can.**

## Decision

A read-only recommendation path plus a single, human-gated write path, on the
orchestrator's container-management layer (`vault-proxy` is infra — it has no
`component.yml` — so this is not the manifest channel).

1. **Read + recommend (no write capability).** `list_egress_approvals` reads the
   proxy log from the `vault-proxy-logs` volume, extracts the *gray-zone*
   off-allowlist hosts only (`action=="BLOCKED"` with reason `"domain not in
   allowlist"`), asks the Sentinel judge about each (`context: egress_request`),
   and returns the ones the judge did not clearly block, each with a plain-language
   reason. **Clear exfil (`EXFIL_BLOCKED`) and DNS-rebinding blocks are filtered
   out before the judge** — they are hard rung-0 blocks and never approvable.

2. **The single writer.** `apply_allowlist_decision` is the **only** code path
   that writes the allowlist, and only on `decision == "always"` (the human tap).
   `"deny"` records the host so it does not re-prompt (SD-A2) and **never** writes
   the allowlist. The agent has no call edge to either.

3. **Two-file persistence (the packaged runtime overwrites the seed each launch).**
   The proxy reads one file, `~/.opentrapp/perimeter/allowlist.txt` (the *live*
   file), which is re-staged from the signed bundle every launch (self-heals
   tampering — ADR-0011). User "Always allow" hosts therefore persist in a
   separate `~/.opentrapp/allowlist.additions.txt` **outside** the staged path,
   and are re-merged into the live file after staging (at bootstrap). Denied hosts
   live in `~/.opentrapp/egress-denials.txt`.

4. **In-place append + SIGHUP (never temp+rename).** The live allowlist is a
   single-file bind-mount; an atomic temp+rename would swap the inode the
   container is pinned to, so the proxy would keep reading the old file. Appends
   are in-place (`O_APPEND`, fsync); the file stays read-only *to the container*.
   `podman kill --signal=HUP vault-proxy` then triggers the proxy's existing
   `_reload_allowlist`, which does the in-memory atomic swap that prevents an
   empty-set window.

## Consequences

- **The invariant is structural, not conventional.** Write-authority is
  human-via-GUI by construction: the agent has no control-plane path, the judge
  only recommends, the allowlist is `:ro` to the container. A fully-wrong judge
  still cannot loosen anything. Pinned by `cargo test --lib` (the sole-writer and
  never-surface-clear-exfil tests) and `orchestrator-check.sh` §27.
- **Approval fatigue is the residual risk (T5).** Only gray-zone hosts surface,
  each with a reason, and "Allow always" carries a deliberate two-tap confirm.
  A user who taps through both approves the host — by design.
- **"Allow once" is deferred (SD-A1).** Ships "Always allow" + "Deny" only; a TTL
  / one-shot allow would need extra L7 support in the proxy.
- **No `vault-proxy.py` change.** The proxy still reads one allowlist file and
  already had `_reload_allowlist`/SIGHUP; the two-file model is entirely host-side.

## Alternatives considered

- **Let the agent request a loosening it auto-applies on judge approval.** Rejected
  outright — it would hand the loosening decision to a component the threat model
  assumes is compromised (T1). The judge advises; the human decides.
- **Atomic temp+rename of the allowlist.** Rejected — breaks the single-file
  bind-mount (inode swap); the proxy's in-memory swap already covers atomicity.
- **A persistent additions file read directly by the proxy (two files in-container).**
  Rejected — needs a `vault-proxy.py` change; the host-side merge keeps the proxy
  reading exactly one file.
