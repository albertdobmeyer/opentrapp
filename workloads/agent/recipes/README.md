# Agent recipes

`vault-agent` = a hardened, **agent-agnostic base** + a pluggable **recipe** that installs
and configures one specific CLI agent. This is what lets the Vault wrap agents other than
OpenClaw — the agent-recipe ([ADR-0024](../../../docs/adr/0024-product-structure-three-concerns.md));
it pairs with the data-driven credential injection in `infra/proxy/vault-proxy.py`, which is
already agent-agnostic.

## Base vs recipe

**Base — agent-agnostic; in `Containerfile` + `scripts/entrypoint.sh`.** The hardening that
defines the perimeter and never depends on which agent runs: `node:22-alpine`, `tini` PID 1,
removed package managers / `curl` / `wget` / `rm` / `chown`, the non-root `vault` user, the
`HTTP(S)_PROXY` + `NODE_EXTRA_CA_CERTS` env (all egress via vault-proxy), read-only root +
dropped caps + seccomp (compose.yml), and the entrypoint's CA-cert wait + read-only
skill-integrity check.

**Recipe — agent-specific; in `recipes/<name>/`.** Installs (and eventually launches) the agent.
- **`install.sh`** (build-time, runs in the builder stage): installs the agent's runtime into
  `/usr/local/lib/node_modules` (the production stage COPYs it) and installs its own build deps.
  Selected by the **`AGENT_RECIPE`** build arg (default `openclaw`); a compose consumer sets it
  via `build.args.AGENT_RECIPE`.

## Status

| Recipe | Build install | Runtime (config + launch) |
|---|---|---|
| **openclaw** | ✅ `recipes/openclaw/install.sh` | Still in `Containerfile` (the `openclaw.mjs` symlink, the Telegram-proxy patch, `openclaw-hardening.json5`, the `node … openclaw.mjs gateway` CMD) + `entrypoint.sh` (auth profile, config lock, `CONSTRAINTS.md`). **Fully recipe-izing the runtime is a tracked follow-up** — it restructures security-critical, partially-unverifiable-here code, so it was deliberately not bundled with the build-install extraction. |
| **opencode** | ⛔ scaffold — `recipes/opencode/install.sh` fails the build | Not built. |

## opencode: what's still needed (part 2b)

A correct opencode recipe is **blocked on verified opencode facts** (the recon could not be
trusted on specifics). Confirm from primary sources, then implement:
- the **install** command (npm? a `curl … | sh` installer? a prebuilt binary?),
- the **launch** invocation, and that opencode is a **TTY session** — *not* a Telegram bot — so
  the opencode recipe **drops** the Telegram gateway/waker entirely,
- its **config path** (the analogue of `~/.openclaw/`),
- an **`injection.json`** for the user's provider, consumed by the data-driven proxy table.

Until those are verified, `AGENT_RECIPE=opencode` fails the build by design rather than
shipping a guess (CLAUDE.md §11 — don't ship unverified claims).
