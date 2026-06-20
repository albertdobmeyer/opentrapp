# vault-agent

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A hardened container harness for the [OpenClaw](https://github.com/anthropics/openclaw) autonomous agent runtime. Provides runtime containment for an autonomous AI agent that would otherwise have full access to the host system.

This is the runtime-containment workload of the [OpenTrApp](https://github.com/albertdobmeyer/opentrapp) monorepo (consolidated 2026-05-30, [ADR-0013](https://github.com/albertdobmeyer/opentrapp/blob/main/docs/adr/0013-monorepo-consolidation.md)); it contributes the `vault-agent`, `vault-proxy` (L7 policy), and `vault-egress` (L3 policy + pinned DoT resolver) containers to the five-container perimeter. See [ADR-0009](https://github.com/albertdobmeyer/opentrapp/blob/main/docs/adr/0009-five-container-perimeter.md) for the L7/L3 split rationale.

**Author:** [@albertdobmeyer](https://github.com/albertdobmeyer)

---

## Design highlight

API keys are not stored inside the agent container. The agent process sends outbound requests to a proxy sidecar (`vault-proxy`); the proxy substitutes a placeholder string in the request headers with the real key value before forwarding. A full compromise of the agent container exposes only the placeholder. `env | grep API` inside `vault-agent` returns nothing.

## Background — what is OpenClaw?

Three names, three layers:

- **OpenClaw** — the agent runtime; an open-source autonomous AI assistant with tool-use, memory, and execution capabilities. Earlier names (Clawdbot, Moltbot) appear in pre-2026 documentation.
- **ClawHub** — a third-party skill (plugin) registry for OpenClaw. The ClawHavoc study (2026-Q1) classified 341 of 2,857 published ClawHub skills (11.9 %) as malicious.
- **Moltbook** — a third-party AI-agent social network. Acquired by Meta on 2026-03-10.

`vault-agent` does not develop, distribute, or alter any of these. It provides container-level isolation around the OpenClaw runtime so that an end user can experiment with the ecosystem without granting the agent process unrestricted access to the host filesystem, host network, or stored credentials.

## Architecture

Two containers, connected by an internal-only Docker network:

```
HOST
│
│   no shared credentials · no host filesystem mounts · no Docker socket
│
├── vault-agent  (Podman or Docker)
│     OpenClaw runtime
│     read-only root · all Linux capabilities dropped · custom seccomp
│     no-new-privileges · 4 GB RAM · 256 PID limit · non-root user
│     ↓ (internal network only — no default gateway)
│
├── vault-proxy  (mitmproxy sidecar)
│     API-key injection · domain allowlist · structured request log on host
│     read-only root · capabilities dropped · custom seccomp (broader than
│     vault-agent's, narrower than mitmproxy default)
│
└── kill switch
      --soft (stop, preserve workspace)
      --hard (remove containers, volumes, networks)
      --nuclear (remove containers + prune runtime caches)
```

When integrated with `opentrapp`, two further containers (`vault-skills`, `vault-social`) operate inside the same perimeter; see `docs/trifecta.md` in the parent repository for the full topology.

---

## Threat model

The motivation for this module is empirical. Within a single week (2026-01-28 to 2026-02-03) the OpenClaw ecosystem produced four documented incidents:

| Incident | Impact |
|---|---|
| CVE-2026-25253 | One-click remote code execution via OpenClaw's own API; stolen tokens disabled the in-built sandbox |
| ClawHavoc | 341 of 2,857 published ClawHub skills (11.9 %) were malicious, distributing Atomic Stealer |
| Database breach | 1.5 M API tokens and 35 K email addresses exposed; row-level security was disabled in the ecosystem's Supabase deployment |
| Public exposure | 21,639 OpenClaw instances reachable from the public internet, most without authentication |

Most third-party hardening guides published before this module placed the API key inside the agent container as an environment variable. A compromised process reads it from `/proc/self/environ`. The proxy-side injection model implemented here does not have that property.

A more detailed analysis is in the companion research repository: [openclaw-research](https://github.com/albertdobmeyer/openclaw-research).

---

## Audience

Appropriate for: users who already run OpenClaw or intend to, have Podman or Docker installed, and want defense-in-depth around the agent process.

Not appropriate for: users who have never used a terminal, are unfamiliar with containers, or who expect this module to make OpenClaw safe for casual use. OpenClaw's own maintainer has stated: *"if you can't understand how to run a command line, this is far too dangerous for you."* This module reduces risk for those who are going to run OpenClaw regardless; it does not transform OpenClaw into consumer software.

## Scope

The vault is a constrained-execution environment for OpenClaw, not an agentic workstation. Features that require host-level access (host email, host filesystem, host browser control, native messaging-app integration) are not enabled, by design.

**Within the perimeter, the agent can:**

- Connect to the Moltbook API (when available) for read-and-react workflows
- Receive Telegram messages and send replies through the dedicated bot
- Hold sessions, accept system-prompt and persona updates, and run skills certified by `vault-skills`
- Read and write within its sandboxed workspace
- Make outbound HTTP(S) requests to allowlisted domains via the proxy

**The agent cannot:**

- Read or write the host filesystem
- Access host email, calendar, or browser
- Use WhatsApp, iMessage, or Signal integrations
- Install skills directly from ClawHub (registry domains denied by default)
- Persist data across container restarts without explicit volume configuration
- Spawn sibling containers (Docker socket not mounted)

---

## Isolation tiers

Container isolation is sufficient for many threat models but not all. The following options exist along an isolation gradient:

| Tier | Approach | Strength | Notes |
|---|---|---|---|
| 1 | Disposable cloud VM running this module | Strongest | Separate kernel and network; rebuild on demand. Recommended for unattended operation or strict isolation. |
| 2 | Local virtual machine (VirtualBox, Hyper-V, UTM) running this module | Strong | Separate kernel; shares physical hardware and LAN. A planned future phase of this module formalises this layer. |
| 3 | Container on the host machine (this module's default) | Adequate for experimentation | No host filesystem mounts; capabilities dropped; seccomp enforced. Shares the host kernel — a kernel-level exploit (uncommon but not impossible) defeats this layer. |

Dedicating an empty disk does not improve isolation; the security boundary is the kernel, not the disk. To strengthen beyond Tier 3, use Tier 1 or 2.

---

## Quick start

Requirements: Podman or Docker, an Anthropic or OpenAI API key.

### Recommended path: Podman/Docker + mitmproxy sidecar

```bash
git clone https://github.com/albertdobmeyer/opentrapp.git
cd opentrapp/workloads/agent
bash scripts/setup.sh        # Linux / macOS
.\scripts\setup.ps1          # Windows PowerShell
```

The setup script detects the available container runtime, prompts for the API key, builds the hardened image, starts the stack, and runs the 24-point verification suite.

### Alternative path: Docker Desktop sandbox plugin

```bash
bash scripts/docker-sandbox-setup.sh
```

Fewer moving parts on Docker Desktop 4.49 or later, but the API key resides inside the container as an environment variable. This path is documented as weaker than the recommended path; use only if the recommended path is not viable in the deployment environment.

---

## Operation

### Control channel: Telegram

The agent runs headlessly. Control is via Telegram. After the stack is running, attach to the container (`podman exec -it vault-agent sh`) and run OpenClaw's pairing flow; it produces a code to enter into a Telegram bot. Use a dedicated Telegram account for this purpose, not the user's personal account; see *Residual risks* below.

The default configuration uses **approval mode**: every action is gated on explicit user confirmation through Telegram. The user may relax this once the system prompt and shell level are trusted; the recommended initial state is strict.

### Monitoring

```bash
# Per-request log (allowed, blocked, flagged)
podman exec vault-proxy cat /var/log/vault-proxy/requests.jsonl

# Live container output
podman compose logs -f

# Re-run the 24-point verification
bash scripts/verify.sh
```

### File transfer

There are no host filesystem mounts. File transfer between host and container is explicit:

```bash
podman cp ~/research/prompts.txt vault-agent:/home/vault/workspace/
podman cp vault-agent:/home/vault/workspace/results.json ~/research/
```

For most interactions the agent returns results via Telegram directly, so explicit `cp` is needed only for bulk transfer.

### Termination

```bash
bash scripts/kill.sh --soft     # stop containers; preserve workspace
bash scripts/kill.sh --hard     # remove containers, volumes, networks
bash scripts/kill.sh --nuclear  # additionally prune runtime caches
```

---

## Verification

```bash
bash scripts/verify.sh
```

Twenty-four checks, grouped:

### Universal hardening (1–14, identical for every shell level)

| # | Check | Verifies |
|---|-------|----------|
| 1 | Proxy DNS resolves | Network routing through the sidecar is functional |
| 2 | Proxy TCP connects | Proxy is accepting connections |
| 3 | Root filesystem read-only | Persistence to image is prevented |
| 4 | Capabilities dropped | No raw sockets, no ptrace, no privilege escalation |
| 5 | Host mounts not accessible | Container has no view of the host filesystem |
| 6 | Windows interop disabled | No `cmd.exe` escape path from WSL |
| 7 | API keys absent from environment | Proxy-side injection confirmed |
| 8 | Docker socket not mounted | Sibling-container creation is prevented |
| 9 | sudo unavailable | No privilege-escalation path |
| 10 | Running as non-root (uid 1000) | Principle of least privilege |
| 11 | Seccomp profile loaded | Custom syscall filter active |
| 12 | `noexec` on `/tmp` | Dropped payloads cannot execute |
| 13 | `no-new-privileges` set | Setuid binaries cannot escalate |
| 14 | PID limit active | Fork-bomb resistance |

### Shell-specific (15–18, adapts to detected level)

| # | Check | Verifies |
|---|-------|----------|
| 15 | Profile matches shell level | Tool baseline is correct for the active shell |
| 16 | Exec security matches shell | Hard = deny, Split = allowlist, Soft = allowlist with safebins |
| 17 | Host and elevated controls correct | Gateway exec disabled, elevated permanently disabled |
| 18 | Safe-binary list matches profile | No orphaned safebins (silent drops prevented) |

### Per-tool security (19–24)

| # | Check | Verifies |
|---|-------|----------|
| 19 | Permanently-denied tools denied | `gateway`, `nodes`, `bash` always in deny list |
| 20 | `rm` not in safebins | Agent is non-destructive |
| 21 | No interpreters in safebins | `sh`, `bash`, `node`, `python` blocked |
| 22 | Proxy allowlist clean | Only expected domains present |
| 23 | Risk score in range | Score matches expected range for active shell |
| 24 | Configuration integrity | Hash matches; no tampering since startup |

---

## Domain allowlist

Edit `proxy/allowlist.txt`. One domain per line. Subdomains are matched implicitly (see *Residual risks*).

```bash
podman compose restart vault-proxy   # full restart
podman exec vault-proxy kill -HUP 1  # hot reload without restart
```

ClawHub registry domains are commented out by default. Uncomment only after explicit source-code review of a specific skill; the recommended practice is to use `vault-skills` to scan and certify the skill instead.

---

## Coverage and residual risks

### Mitigated

- API-key exfiltration (proxy-side injection — key not in container)
- Network exfiltration to non-allowlisted domains (allowlist + logging)
- Container escape via filesystem traversal, capability gain, or privilege escalation
- Resource exhaustion (fork bombs, memory pressure, PID exhaustion)
- Host contamination from malicious payloads

### Not mitigated

- Hypervisor escape (state-actor capability; out of scope for personal-machine threat models)
- Container-host kernel zero-days (a planned future phase formalises VM-level isolation)
- Social engineering against the human approver (a user-approved malicious action remains malicious)
- Side-channel attacks (Spectre / Meltdown class; not practically exploitable in this configuration)

### Residual risks the operator must understand

These are architectural realities of the design, not bugs.

**The proxy holds the API key.** `vault-proxy` is the only container that holds the real credential. A compromise of the proxy container (rather than the agent container) exposes the key. The proxy is hardened with read-only root, dropped capabilities, no-new-privileges, memory and PID limits, and a custom seccomp profile that blocks `io_uring`, `ptrace`, `unshare`, `setns`, `bpf`, and other escape primitives. Its seccomp profile is broader than `vault-agent`'s because mitmproxy requires wider syscall access for TLS interception, but still narrower than mitmproxy's default.

**Allowlisted domains can be abused during an active session.** API-provider domains (e.g. `api.anthropic.com`) must be allowlisted for OpenClaw to function. A compromised agent can issue arbitrary API calls using the proxy-injected credential — it cannot read the literal key, but it can use it. Mitigation: configure a hard spending cap on the API key. Treat the spending cap as part of the security boundary, not as a billing convenience.

**The Telegram control channel is a trust boundary.** A compromise of the operator's Telegram account permits an attacker to approve agent actions. Use a dedicated Telegram account, enable two-factor authentication, and treat its credentials as security-critical.

**Allowlist subdomain matching is implicit.** Allowing `github.com` also allows `api.github.com`; allowing `example.com` also allows `tunnel.example.com`. Where a parent domain has subdomains that are exfiltration-capable, allowlist only the necessary leaf. The default policy allows `raw.githubusercontent.com` (read-only) but not `github.com` for this reason.

**`raw.githubusercontent.com` is allowed by default.** Read-only access for skill source review. A compromised agent could encode exfiltrated data into URL paths (e.g. `/user/repo/main/<base64data>`); GitHub's server-side logs would record the path even if the request returned 404. Comment out the entry in `proxy/allowlist.txt` if not needed.

**`registry.npmjs.org` is denied by default.** Allowing it permits npm packages to execute lifecycle scripts (`preinstall`, `postinstall`) at install time. `noexec` mounts block ELF execution but not interpreted JavaScript loaded via `require()`. The other restrictions (no host mounts, no capabilities, no host network) limit blast radius but the gap is real. Allow only after auditing the specific package.

**Container destruction does not guarantee complete cleanup.** `kill.sh --hard` removes containers, volumes, and networks. Layer caches, image metadata, and runtime logs persist on the host. These do not contain the API key (proxy-side injection ensures that), but may contain conversation logs or activity metadata. For full cleanup, additionally remove the cloned repository directory and run `podman system prune -a` (or the Docker equivalent).

---

## Project structure

```
workloads/agent/
├── Containerfile                    multi-stage hardened image
├── compose.yml                      container + proxy orchestration
├── component.yml                    OpenTrApp manifest contract
├── Makefile                         standardised targets (setup, verify, test, …)
├── config/
│   ├── tool-manifest.yml            source of truth for every OpenClaw tool
│   ├── openclaw-hardening.json5     active agent configuration
│   ├── hard-shell.json5             Hard Shell preset
│   ├── split-shell.json5            Split Shell preset
│   ├── soft-shell.json5             Soft Shell preset
│   ├── hard-shell-allowlist.txt     domain template
│   ├── vault-seccomp.json           syscall filter (vault-agent)
│   └── vault-proxy-seccomp.json     syscall filter (vault-proxy)
├── proxy/
│   ├── vault-proxy.py               key injection + allowlist enforcement
│   └── allowlist.txt                active domain allowlist
├── scripts/
│   ├── tool-control.sh              per-tool whitelisting/blacklisting
│   ├── verify.sh                    24-point verification suite
│   ├── setup.sh / setup.ps1         one-command setup
│   ├── kill.sh / kill.ps1           three-level termination
│   └── entrypoint.sh                container startup
├── tests/                           13 test scripts (12 non-destructive, 1 kill-switch)
└── docs/
    ├── openclaw-reference.md        tools, configuration, Telegram, sessions
    ├── openclaw-internals.md        source-code analysis of the dist/ bundles
    ├── roadmap.md                   phased development plan
    └── setup-guide.md               end-user setup
```

---

## Origin

This module began as security research, not as a container project. The threat-landscape analysis preceded and informed the design. The companion research repository documents the analysis:

- [openclaw-research](https://github.com/albertdobmeyer/openclaw-research) — security analysis, threat modeling, ecosystem documentation, and 24 published-as-reference ClawHub skills

---

## Disclaimer

This software is provided "as is", without warranty of any kind. By using it, the operator accepts full responsibility for any consequences on their machine, network, accounts, and API billing. This is a containment tool for software whose default configuration is hazardous on a personal computer. It reduces risk; it does not eliminate it.

The authors are not responsible for financial loss from API-key abuse, data loss or corruption, security breaches resulting from misconfiguration or unpatched vulnerabilities, malicious skills or payloads, or anything that occurs after the operator runs `setup.sh`.

If the operator's threat model requires the strongest available isolation, run this module on a disposable virtual machine with a disposable API key and a hard spending cap. Operators who do not understand that recommendation should not use this software.

## License

[MIT](LICENSE).
