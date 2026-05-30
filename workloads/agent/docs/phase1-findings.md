# Phase 1 Findings — OpenClaw Compatibility Verification

**Date:** 2026-03-23
**Vault version:** opencli-container @ main (post-Phase-0 + Phase-1 fixes)
**OpenClaw version:** openclaw@2026.2.17 (since upgraded to 2026.2.26 — see `Containerfile` line 19)
**Test key:** Anthropic Haiku-only, $5 cap, no web tools
**Container runtime:** Podman 4.9.3 (rootless) on Ubuntu 24.04

---

## Open Question Answers

### OQ4: Does OpenClaw work on Node 20?

**NO.** OpenClaw 2026.2.17 requires Node >=22.12.0 (`engines.node` field in package.json). The Containerfile was upgraded from `node:20-alpine` to `node:22-alpine` (Node 22.22.1). Build succeeds on Node 22.

### OQ8: Config format — YAML, JSON, or both?

**JSON5 only.** OpenClaw reads `~/.openclaw/openclaw.json` (JSON5 format with comments and trailing commas). It does NOT accept YAML config files. The `--config` CLI flag does not exist; use `--profile <name>` to isolate state under `~/.openclaw-<name>/`.

**Schema validation is strict** (Zod at startup): unknown keys cause the Gateway to refuse to start. No warning mode, no graceful degradation.

### OQ8b: Do our config keys match OpenClaw's schema?

**Our original YAML keys were completely wrong.** Mapping:

| Our YAML key (wrong) | OpenClaw JSON5 key (correct) |
|----------------------|------------------------------|
| `sandbox.mode` | `agents.defaults.sandbox.mode` |
| `exec.approvals.mode` | `tools.exec.security` |
| `tools.elevated: []` | `tools.elevated.enabled: false` |
| `memory.persistent` | Does not exist in schema (removed) |
| `telemetry.enabled` | Not a top-level key (logging.redactSensitive instead) |
| `mdns.enabled` | Does not exist in schema (removed) |
| `pairing.mode` | `channels.<provider>.dmPolicy` |

Three keys from our original config caused startup failures:
- `agents.defaults.model`: expects `{ primary: "provider/model" }` (object), not a string
- `agents.defaults.memory`: unrecognized key
- `tools.exec.strictInlineEval`: unrecognized key in v2026.2.17

### OQ1: Gateway WebSocket API availability

**YES.** The Gateway runs at `ws://127.0.0.1:18789` inside the container. It supports WebSocket connections for session management, agent control, and configuration. However, connecting requires device authentication (Ed25519 keypair + device token). The CLI uses this API internally.

**Hot reload confirmed:** The Gateway watches `~/.openclaw/openclaw.json` and applies changes automatically (debounce 300ms by default). This means shell switching (molt) is possible without container restart for Layers 3-6 changes.

### Layer 4: Sandbox behavior without Docker socket

**Confirmed working as designed.** OpenClaw logs: "Docker not available; skipping sandbox image checks." The `sandbox.mode: "non-main"` setting is accepted but container-spawning features are silently disabled. Application-level restrictions (tool policies, exec security, elevated access) still apply. This validates our spec's Layer 4 design: the vault container IS the sandbox.

### NEW: Domains OpenClaw needs beyond our allowlist

**None.** OpenClaw started successfully with only 3 allowed domains (api.anthropic.com, api.openai.com, raw.githubusercontent.com). The proxy logs show zero BLOCKED requests during normal startup. OpenClaw does not phone home, does not download skills, and does not reach external services at startup.

### NEW: Gateway requires `gateway.mode: "local"`

If `gateway.mode` is not set, the Gateway refuses to start with: "Gateway start blocked: set gateway.mode=local (current: unset) or pass --allow-unconfigured." This must be in the config.

---

## Containerfile / Compose Fixes Required

| Fix | Why |
|-----|-----|
| Package name: `openclaw` (not `@anthropic-ai/openclaw`) | Wrong npm scope |
| Node: 22-alpine (not 20-alpine) | OpenClaw requires >=22.12.0 |
| `--ignore-scripts` during npm install | Skips node-llama-cpp native compilation (not needed — vault uses API, not local LLMs) |
| Binary: symlink instead of COPY | COPY flattens symlinks, breaking relative imports |
| User: delete existing `node:1000` before creating `vault:1000` | Node 22 image has `node` user at UID 1000 |
| Proxy image: `docker.io/` prefix | Podman requires fully qualified image names |
| Proxy: add `SETUID` and `SETGID` capabilities | Mitmproxy entrypoint uses `gosu` for user switching |
| Proxy: remove `read_only: true` and `no-new-privileges:true` | Mitmproxy entrypoint needs writable `/etc` and user switching |
| Vault tmpfs: `mode=1777` instead of `uid=1000` | Podman 4.9.3 doesn't support `uid=` in tmpfs options |
| Add `~/.openclaw` tmpfs mount | OpenClaw config lives at `~/.openclaw/openclaw.json` |
| Entrypoint: place JSON5 config at `~/.openclaw/openclaw.json` | OpenClaw's config path (not `--config` flag) |
| CMD: `openclaw gateway` (not `openclaw --config ...`) | `--config` flag doesn't exist; `gateway` subcommand starts the service |
| Entrypoint: increase CA cert wait to 60s | Proxy startup + cert generation takes longer on first run |

---

## Security Verification Results

### verify.sh: 15/15 PASSED

| # | Check | Result |
|---|-------|--------|
| 1 | vault-proxy hostname resolves | PASS |
| 2 | TCP connect to vault-proxy:8080 | PASS |
| 3 | Root filesystem read-only | PASS |
| 4 | Ping blocked (NET_RAW dropped) | PASS |
| 5 | /mnt/c not accessible | PASS |
| 6 | No Windows binaries in PATH | PASS |
| 7 | API key not in container env | PASS |
| 8 | Docker socket not mounted | PASS |
| 9 | sudo unavailable | PASS |
| 10 | Non-root user (uid 1000) | PASS |
| 11 | Seccomp profile loaded | PASS |
| 12 | Noexec on /tmp | PASS |
| 13 | No-new-privileges set | PASS |
| 14 | PID limit configured (2048) | PASS |
| 15 | Config: exec security = deny | PASS |

**Note:** PID limit shows 2048 instead of 256. Podman rootless may override compose `pids_limit`. To be investigated.

### Network isolation tests: 1/6 + 1 informational

Network tests that require HTTP proxy-protocol communication from Node.js fail because the simple `http.get()` client doesn't properly speak mitmproxy's proxy protocol. However:
- Direct internet bypass is correctly blocked (no default gateway on vault-internal)
- DNS resolution to proxy works
- TCP connection to proxy works
- OpenClaw itself communicates through the proxy successfully
- Proxy correctly blocks and returns 403 for non-allowlisted domains (confirmed in proxy logs)

**Action needed:** Rewrite network tests to use OpenClaw's bundled `undici` ProxyAgent or a similar proper proxy-aware client. Tracked for Phase 2.

### SSH key integrity: VERIFIED

`~/.ssh/hetzner_linuxlaptop` compared byte-for-byte with backup — unchanged. The vault container did not access the host filesystem.

---

## OpenClaw Exploration Summary

| Property | Value |
|----------|-------|
| Version | 2026.2.17 |
| Node runtime | v22.22.1 |
| Binary location | `/usr/local/bin/openclaw` -> `../lib/node_modules/openclaw/openclaw.mjs` |
| Config file | `~/.openclaw/openclaw.json` (JSON5) |
| Gateway port | ws://127.0.0.1:18789 (loopback) |
| Process tree | tini (PID 1) -> openclaw (PID 2) -> openclaw-gateway (PID 15) |
| Model | anthropic/claude-haiku-4-5 |
| Auto-enabled plugins | Telegram, WhatsApp (configured but no credentials) |
| Services started | heartbeat, health-monitor, browser/service, canvas |
| Image size | 715 MB |

### CLI interaction: NOT POSSIBLE without device pairing

The OpenClaw CLI requires device authentication (Ed25519 keypair + device token) to connect to the Gateway. Running `openclaw health` or `openclaw status` fails with "unauthorized: device token mismatch." To interact with OpenClaw, you need either:
1. A configured messaging channel (Telegram/WhatsApp) — Phase 2 work
2. The WebChat UI (http://127.0.0.1:18789/) — not accessible from host (container loopback)
3. Device pairing via `openclaw devices approve` — requires the gateway to have a paired device first

This means **Phase 2 must set up Telegram** before we can test actual agent interaction.

---

## Impact On Later Phases

1. **Hard Shell formalization:** ~~Gear configs must be JSON5 with correct OpenClaw key paths. The old YAML config and `component.yml` references to it need updating.~~ **DONE** — configs are JSON5, component.yml updated (2026-03-27).

2. **Monitoring:** Proxy logs work (JSON at `/var/log/vault-proxy/requests.jsonl`). OpenClaw's own logs are at `/tmp/openclaw/openclaw-*.log` inside the container. Both can be parsed. See `docs/roadmap.md` Phase 2.

3. **Split Shell:** ~~Telegram/WhatsApp setup is required for any interaction. Credential persistence (persistent volume) is confirmed necessary. Hot-reload via config file watching enables shell switching without container restart for Layers 3-6.~~ **DONE** — Split Shell implemented with persistent volume and hot-reload (2026-03-25).

4. **Spec update needed:** ~~The spec references the old YAML config format in multiple places. Update to reflect JSON5 format and correct key paths.~~ **DONE** — component.yml and CLAUDE.md updated (2026-03-27).

5. **Network test rewrite:** The test-network-isolation.sh needs a proxy-aware HTTP client. This is a test tooling issue, not a security issue.

*Note: This document uses the original phase numbering from early development. "Gear 1/2/3" has since been renamed to "Hard/Split/Soft Shell" — see `GLOSSARY.md` in the opentrapp root.*
