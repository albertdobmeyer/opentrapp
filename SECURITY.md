# Security Policy

## Scope

This policy covers the four repositories that compose the OpenTrApp distribution:

- [`opentrapp`](https://github.com/albertdobmeyer/opentrapp) — desktop application and perimeter orchestrator
- [`opencli-container`](https://github.com/albertdobmeyer/opencli-container) — runtime-containment module (`vault-agent`, `vault-proxy`)
- [`openskill-forge`](https://github.com/albertdobmeyer/openskill-forge) — supply-chain defense module (`vault-forge`)
- [`openagent-social`](https://github.com/albertdobmeyer/openagent-social) — social-content analysis module (`vault-pioneer`); **parked since 2026-05-03**, see the repository's README

Vulnerabilities in upstream dependencies (Tauri, mitmproxy, Rust crates, npm packages) and in the third-party platforms this software interfaces with (Anthropic API, Telegram, OpenClaw, ClawHub, Moltbook) are out of scope; please report those to their respective maintainers.

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

- Container-escape vectors in `opencli-container` (capability gain, mount escape, kernel-namespace escape, seccomp bypass)
- Privilege escalation through the OpenTrApp manifest runner (command injection, path traversal, environment-variable leakage)
- Credential exposure (API keys or tokens visible to a container that should not have them, leakage to logs or stderr)
- Supply-chain bypasses in `openskill-forge` (skill-scanner false negatives, CDR pipeline bypass, manifest tampering between scan and delivery)
- Network-allowlist bypasses through `vault-proxy` (egress to denylisted hosts, request smuggling, header injection enabling unauthorised endpoints)
- Defects in the perimeter lifecycle that leave containers running after the desktop application exits or that allow unauthorised re-entry into a "paused" state

## Out of scope

- Issues that require physical access to the host machine
- Social-engineering attacks that require explicit user approval to succeed (the agent runs in an approval-gated mode by default)
- Issues affecting `vault-pioneer` until the Moltbook API stabilises and the module is un-parked
- Configuration mistakes by an end user that override defaults documented as security-critical
