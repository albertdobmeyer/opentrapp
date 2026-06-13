# Security Policy

## Scope

Single monorepo since [ADR-0013](docs/adr/0013-monorepo-consolidation.md). This policy covers all workloads and infrastructure containers in the OpenTrApp distribution:

- [`opentrapp`](https://github.com/albertdobmeyer/opentrapp) — desktop application and perimeter orchestrator (the entire repository)
- `workloads/agent/`  — runtime-containment workload (`vault-agent`)
- `infra/proxy/`      — L7 egress policy (`vault-proxy`) — domain allowlist + API-key injection
- `infra/egress/`     — L3 egress policy (`vault-egress`) — kernel RFC1918 drop + pinned DNS
- `workloads/skills/`  — supply-chain defense workload (`vault-skills`) — skill scanner + CDR
- `workloads/social/` — agent-social-feed analysis workload (`vault-social`); **parked since 2026-05-03**

The previously-separate `opencli-container`, `openagent-skills`, and `openagent-social` GitHub repositories are archived references — please report vulnerabilities here, not there.

Vulnerabilities in upstream dependencies (Tauri, mitmproxy, Rust crates, npm packages) and in the third-party platforms this software interfaces with (Anthropic API, Telegram, OpenClaw, ClawHub, Moltbook) are out of scope; please report those to their respective maintainers. Advisories we knowingly accept (chiefly the unmaintained Tauri GTK3 webview crates) and how to read the project's OpenSSF Scorecard are documented in [`docs/known-advisories.md`](docs/known-advisories.md).

The full attacker-capability matrix (T1–T6), the perimeter layers that mitigate each, and the residual risks that remain after mitigation are documented in [`docs/threat-model.md`](docs/threat-model.md). The "In scope" and "Out of scope" sections below are aligned with that document.

## Reporting a vulnerability

Do **not** open a public GitHub issue for security vulnerabilities.

Send a private report to **albertdobmeyer@proton.me** with:

- The affected repository and file path(s)
- A description of the vulnerability and a proof-of-concept where applicable
- Steps to reproduce
- Expected impact (confidentiality, integrity, availability; whether host or container scope)

Reports are acknowledged within 48 hours. Severity is assessed using a CVSS-style impact analysis; remediation timing is prioritised accordingly. Reporters are credited in the resulting fix commit unless they request anonymity.

## In scope

The following classes of issue are accepted:

- Container-escape vectors in `workloads/agent/` (capability gain, mount escape, kernel-namespace escape, seccomp bypass)
- Privilege escalation through the OpenTrApp manifest runner (command injection, path traversal, environment-variable leakage)
- Credential exposure (API keys or tokens visible to a container that should not have them, leakage to logs or stderr)
- Supply-chain bypasses in `workloads/skills/` (skill-scanner false negatives, CDR pipeline bypass, manifest tampering between scan and delivery)
- Network-allowlist bypasses through `vault-proxy` (egress to denylisted hosts, request smuggling, header injection enabling unauthorised endpoints)
- Defects in the perimeter lifecycle that leave containers running after the desktop application exits or that allow unauthorised re-entry into a "paused" state

## Out of scope

- Issues that require physical access to the host machine
- Social-engineering attacks that require explicit user approval to succeed (the agent runs in an approval-gated mode by default)
- Issues affecting `vault-social` until the Moltbook API stabilises and the module is un-parked
- Configuration mistakes by an end user that override defaults documented as security-critical
