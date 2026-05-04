# ADR-0002 — Adaptive shell levels (Hard / Split / Soft) as a system state

**Status:** Accepted
**Decision date:** 2026-03-30 (tool-control system design); reaffirmed 2026-04-15 (architecture v2 redesign)
**Implemented by:** [`components/openclaw-vault/scripts/tool-control.sh`](../../components/openclaw-vault/scripts/tool-control.sh); [`components/openclaw-vault/scripts/tool-control-core.py`](../../components/openclaw-vault/scripts/tool-control-core.py); [`components/openclaw-vault/config/tool-manifest.yml`](../../components/openclaw-vault/config/tool-manifest.yml); [`components/openclaw-vault/config/{hard,split,soft}-shell.json5`](../../components/openclaw-vault/config/)
**Verified by:** Verification checks 15–18 (shell-specific) and 19–24 (per-tool security) in [`components/openclaw-vault/scripts/verify.sh`](../../components/openclaw-vault/scripts/verify.sh)

---

## Context

The OpenClaw runtime exposes a configurable tool surface — at design time, the operator can deny or allow individual tools (`exec`, `read`, `write`, `web_fetch`, `web_search`, `cron`, etc.) and adjust per-action approval mode (`always`, `on-miss`, `never`). A typical hardening guide suggests choosing one configuration and locking it down.

The "one configuration" approach forces a compromise that is structurally unsatisfying:

- **Locked down too far.** The agent can chat but cannot do useful work — answer questions from training data, read files, draft text. Anything that requires file I/O, command execution, or web access is blocked. The agent fails the user's workflow goals.
- **Locked down too little.** The agent has broad capabilities continuously, including during periods (overnight, while the user is in a meeting, during prolonged idle) when the user has no specific oversight. A successful prompt injection during these periods has the agent's full capability surface to exploit.
- **Manual reconfiguration is not realistic.** Asking the user to edit `openclaw.json` and restart the runtime each time their task changes is operationally infeasible for a non-developer user.

Concurrently, the perimeter has *invariant* protections that hold regardless of any configuration choice the user might make: read-only root filesystem in the container, capability drops, seccomp profile, no host filesystem mounts, no host network access, the proxy-side credential injection from ADR-0001. These are mechanical floors below which the system cannot drop.

The architectural opportunity is to treat the *agent's tool capability* as a higher-level construct that can be adjusted per task context, while the *infrastructure protections* remain fixed. The user (or a trusted CLI coordinator) selects a level appropriate to the current task; the perimeter enforces the corresponding tool catalogue, exec policy, and proxy allowlist; the universal hardening continues to apply at every level.

## Decision

The agent's privilege level is treated as a **system state**, not a configuration value. Three states are defined:

| State | Risk score | Tool profile | Exec policy | safebins | Proxy domains |
|-------|-----------|--------------|-------------|----------|---------------|
| Hard Shell | 0.00 | `minimal` | `deny` | 0 | 3 (LLM API + Telegram only) |
| Split Shell *(default)* | 0.18 | `coding` | `allowlist` + `ask: always` | 16 | 3 |
| Soft Shell | 0.34 | `coding` | `allowlist` + `ask: on-miss` | 26 | 4 (adds `raw.githubusercontent.com`) |

Three properties of this design are non-obvious and warrant explicit treatment:

**(a) The shell is a state, not a setting.** Transitioning between levels (a "shell switch") swaps the active configuration files (`config/{hard,split,soft}-shell.json5` and the matching domain allowlist) and restarts the agent container with the new policy. The transition is initiated by `tool-control.sh` from the host. The agent itself cannot promote its own shell level; promotion is always initiated from Tier 1 (the user or a trusted CLI coordinator).

**(b) Demotion is permitted, promotion requires confirmation.** Moving from a more permissive to a more restrictive level ("shell up") is unconditional and instant. Moving the other way ("shell down") requires explicit user or coordinator approval. This is the same direction-of-trust asymmetry that classical capability-based systems use; granting capability is conservative, withdrawing it is not.

**(c) The shell state cannot weaken the universal hardening.** Container-level mechanisms (read-only root, capability drops, seccomp, network policy) hold at every shell level. The shell controls the *tool layer* — what OpenClaw exposes to the LLM as available tools — and the *exec policy* — what OpenClaw executes when the agent asks. It cannot weaken the *infrastructure layer*. A Soft-Shell agent attempting to write to `/etc` faces the same read-only root filesystem as a Hard-Shell agent.

The set of *permanently-protected resources* — root, SSH keys, GPG keys, password stores and keyrings, administrative accounts, the Docker / Podman socket, the perimeter's own configuration files — is denied at every shell level without exception. These cannot be reached even at Soft Shell.

The implementation lives in `tool-control.sh`. The script reads the active shell-level preset (Hard / Split / Soft) from the user's request, resolves it against `config/tool-manifest.yml` (the source-of-truth declaration of every available tool, its risk classification, and its profile membership), generates the corresponding `openclaw.json` from the matching JSON5 preset, swaps the proxy allowlist, and restarts the agent container. Each step is verified before commit; a failed switch leaves the prior state in place.

## Consequences

### Positive

- **The agent has the privilege it needs for the current task and no more.** During a chat-only conversation, Hard Shell denies command execution, file I/O, web access, and skill loading. During focused file work, Split Shell permits workspace I/O and safelisted commands with per-action approval. During autonomous research-style tasks, Soft Shell permits broader autonomy. The privilege tracks the task.
- **Promotion is auditable.** Every transition is logged with timestamp, requesting party (user or coordinator), and prior/new states. A reader of the log can reconstruct the privilege history.
- **Demotion is always available and instant.** A user who suspects something is wrong — anomalous network activity in the proxy log, a Telegram message that doesn't read right — can `make hard-shell` to immediately drop the agent's tool surface to chat-only without restarting the perimeter or losing session state.
- **The state is observable from outside the agent's runtime.** A reader of the host filesystem can determine the active level by inspecting `~/.openclaw/openclaw.json` inside the vault-agent volume. The state is therefore auditable post-hoc.
- **Universal hardening composes cleanly.** The hardening floor (capability drops, seccomp, read-only root, etc.) is independent of the shell level and is verified separately by checks 1–14 of `verify.sh`. Shell-level correctness is verified by checks 15–18; per-tool correctness by checks 19–24. Each check group has a single clear concern.

### Negative

- **A switch is not free.** Restarting the agent container costs ~5–10 seconds and clears any in-memory session state the agent was holding. A coordinator that switches frequently fragments the agent's context. In practice this is rarely a problem (most users settle on Split Shell and stay there), but it is a real cost during exploratory testing.
- **Three states is a coarse-grained gradient.** The design rejected continuous capability adjustment in favour of named discrete levels for predictability — a user can reason about *which* level the agent is in without looking up dozens of individual flags — but at the cost of inflexibility. A workflow that wants Soft-Shell autonomy *except* for one specific tool must add or remove a tool from the safelist via `tool-control.sh`'s per-tool API; the discrete preset itself does not represent that intermediate state cleanly.
- **The state is host-side, not in-container.** A reader of just `vault-agent`'s container state cannot determine the shell level — they would observe the rendered `openclaw.json` but not the metadata that says "this corresponds to Hard Shell." A future revision could surface the level as a runtime label on the container itself.
- **The state needs to be re-applied on every container start.** `entrypoint.sh` re-renders the active configuration on container creation. A change made via `tool-control.sh` while the container is running takes effect on the *next* restart, not immediately. The script handles this by triggering the restart explicitly.

### Neutral

- The risk score (0.00 / 0.18 / 0.34) is a heuristic ranking of the three levels' attack surface; it is not a calibrated metric. The score guides verification (check 23: "risk score in range") but is not load-bearing on user-facing reasoning.

## Alternatives considered

**(A) A single fixed configuration.** The OpenClaw-default approach. Rejected for the reasons in the *Context* section: forces an unsatisfying compromise between usefulness and safety.

**(B) Continuous per-tool enable/disable.** Let the user toggle each tool individually with no preset levels. Rejected because the resulting configuration space (every combination of N tools) is too large for users to reason about; presets give predictable behaviour.

**(C) Time-of-day or activity-based automatic transitions.** Have the perimeter automatically demote to Hard Shell when the user is away (e.g. screen locked, no Telegram messages in the last hour). Rejected because the trigger conditions are operationally fragile and the security gain is marginal: the agent is just as constrained at any shell level by the universal hardening.

**(D) Per-skill capability requests.** Let each skill declare its required capabilities and have the perimeter dynamically grant them per skill. Rejected because it adds a manifest layer (skill capability requests) without addressing the underlying problem (whether the user trusts the skill enough to grant the capabilities). The current design treats skill installation as a Split-Shell-or-above operation that goes through `clawhub-forge`'s scanner-and-CDR pipeline (ADR-0003), which is a structurally different — and stronger — approach than per-skill capability requests.

**(E) Capability tokens passed at the runtime layer.** Each tool invocation carries a capability token that the runtime validates. Rejected because OpenClaw does not expose a capability-token API; bolting one on would require forking the runtime, which is a maintenance burden the perimeter explicitly avoids.

## References

- Companion architecture document: [`docs/trifecta.md`](../trifecta.md) §5 (Adaptive shell)
- Whitepaper: [`docs/whitepaper.md`](../whitepaper.md) §5 (Adaptive shell levels)
- Glossary: [`GLOSSARY.md`](../../GLOSSARY.md) §2 (Shell levels)
- Verification: 24-point check groups 15–18 and 19–24 in [`components/openclaw-vault/scripts/verify.sh`](../../components/openclaw-vault/scripts/verify.sh)
- Implementation: [`components/openclaw-vault/scripts/tool-control.sh`](../../components/openclaw-vault/scripts/tool-control.sh) and [`components/openclaw-vault/scripts/tool-control-core.py`](../../components/openclaw-vault/scripts/tool-control-core.py)
- Source of truth for tool classifications: [`components/openclaw-vault/config/tool-manifest.yml`](../../components/openclaw-vault/config/tool-manifest.yml)
- Per-level configurations: [`components/openclaw-vault/config/{hard,split,soft}-shell.json5`](../../components/openclaw-vault/config/)
- Historical: [`components/openclaw-vault/docs/archive/specs/2026-03-30-tool-control-system-design.md`](../../components/openclaw-vault/docs/archive/specs/2026-03-30-tool-control-system-design.md) (the original tool-control design document, archived)
