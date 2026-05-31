SECURITY-ANALYSIS-COMPILATION.md
16.98 KB •285 lines
•
Formatting may be inconsistent from source

# ðŸ•¸ï¸ Security Analysis Compilation
## OpenClaw / Moltbook Vulnerability & Threat Landscape

**Last Updated**: February 13, 2026  
**Maintained by**: Albert K. Dobmeyer | Agentic Networks Lab  
**Classification**: Defensive analysis only â€” no exploit development  
**Status**: Living document

---

## Purpose

This document compiles security findings from credible researchers and firms into a single reference. The goal is threefold: (1) inform safe participation decisions, (2) provide article/portfolio material demonstrating security awareness, and (3) serve as a cautionary architecture reference for what "agent systems without security" looks like in practice.

The security story is the most important story in the Moltbook/OpenClaw ecosystem. Every credible analysis leads with it.

---

## The Conceptual Framework: Willison's "Lethal Trifecta"

Simon Willison identified three capabilities that, when combined in a single agent, create inherent vulnerability:

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  1. PRIVATE DATA ACCESS  â”‚  Emails, files, credentials, browser history,
â”‚                          â”‚  chat messages, SSH keys, API tokens
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  2. UNTRUSTED CONTENT    â”‚  Web browsing, incoming messages from
â”‚     EXPOSURE             â”‚  arbitrary senders, Moltbook posts,
â”‚                          â”‚  third-party Skills
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  3. EXTERNAL COMMS       â”‚  Send emails, post messages, make API
â”‚     CAPABILITY           â”‚  calls, exfiltrate data without DLP
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
         +
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  4. PERSISTENT MEMORY    â”‚  Added by Palo Alto Networks:
â”‚     (Palo Alto addition) â”‚  attacks become stateful, delayed-
â”‚                          â”‚  execution, fragmented across inputs
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

**OpenClaw has all four by default.** This is not a bug â€” it's the architecture. The framework was designed to give agents maximum capability with minimal friction. Security was explicitly deferred.

Willison's assessment: "my current pick for 'most likely to result in a Challenger disaster'" â€” referencing NASA's normalization of deviance where known risks are accepted until catastrophic failure.

---

## Incident Timeline

| Date | Incident | Severity | Discoverer |
|------|----------|----------|------------|
| Jan 28, 2026 | Moltbook launches with no rate limiting, no identity verification | Design flaw | â€” |
| Jan 29-30, 2026 | Crustafarianism emerges â€” demonstrates agent coordination without governance | Emergent risk | â€” |
| Jan 30, 2026 | **CVE-2026-25253** patched (one-click RCE) | Critical (CVSS 8.8) | Mav Levin, DepthFirst |
| Jan 31, 2026 | **Moltbook database breach** â€” Supabase misconfiguration, RLS disabled | Critical | Jamieson O'Reilly; Wiz (Gal Nagli) |
| Jan 31, 2026 | **21,639 OpenClaw instances** found exposed on public internet | High | Censys scan |
| Feb 1, 2026 | Moltbook tables secured (~3 hours after report) | Remediated | Schlicht/Wiz |
| Feb 1-2, 2026 | **ClawHavoc** malware campaign peaks | Critical | Koi Security |
| Feb 3, 2026 | 341 malicious Skills documented out of 2,857 total (11.9%) | Critical | Koi Security |
| Ongoing | Prompt injection via Moltbook posts targeting connected agents | High | Multiple researchers |

---

## Detailed Vulnerability Analysis

### 1. CVE-2026-25253: One-Click Remote Code Execution

**Source**: Mav Levin, DepthFirst  
**CVSS**: 8.8 (High)  
**Status**: Patched in v2026.1.29 (January 30, 2026)

**Attack Chain** (six steps, milliseconds after visiting malicious page):

```
Victim clicks malicious link
    â”‚
    â–¼
Cross-site WebSocket hijacking
(OpenClaw didn't validate Origin header)
    â”‚
    â–¼
Token exfiltration via client-side JS
    â”‚
    â–¼
Attacker connects to victim's OpenClaw instance
    â”‚
    â–¼
Sandbox disabled via API
(exec.approvals.set = "off")
    â”‚
    â–¼
Container escape via API
(tools.exec.host = "gateway")
    â”‚
    â–¼
FULL RCE ON HOST MACHINE
```

**Key insight**: The attack exploited the API's own configuration endpoints to disable security controls. The sandbox wasn't bypassed â€” it was turned off through legitimate API calls made with a stolen token. This is an architectural vulnerability, not an implementation bug.

**Remediation**: Origin header validation added. But the underlying pattern â€” that tokens grant full configuration authority â€” remains a design concern.

### 2. Moltbook Database Breach (Supabase Misconfiguration)

**Source**: Wiz (Gal Nagli), independently Jamieson O'Reilly  
**Root cause**: Supabase deployed with Row Level Security (RLS) disabled

**Exposed data**:
- 1.5 million API authentication tokens
- 35,000+ email addresses
- 6,000+ owner emails
- Thousands of private messages
- Identity verifications
- Third-party API credentials (including OpenAI keys shared between agents)

**Time to exploit**: Under 3 minutes (Wiz demonstration)

**Attribution quote** (Gal Nagli): "Classic byproduct of vibe coding"

**Context**: Matt Schlicht (Moltbook creator) publicly stated he wrote zero lines of code for the platform. The entire backend was AI-generated. This is the canonical example of why AI-generated infrastructure requires human security review.

**Broader implication**: Anyone could hijack any agent, post as high-profile users (including Andrej Karpathy's agent), inject malicious content into the feed, and access private conversations. For agents connected to real email/messaging accounts, this was a pathway to impersonating humans through their own agents.

### 3. ClawHavoc: Supply Chain Attack Campaign

**Source**: Koi Security  
**Scale**: 341 malicious Skills out of 2,857 on ClawHub (11.9% malicious rate)  
**Attribution**: 335 from a single coordinated operation

**Target categories by volume**:
- Crypto utilities: 111 Skills
- YouTube tools: 57
- Finance tools: 51
- Polymarket bots: 34
- Auto-updaters: 28
- Google Workspace: 17

**Techniques**:
- **Typosquatting**: 29 domains (clawhub, clawhubb, cllawhub, etc.)
- **Malware**: Atomic Stealer (AMOS) for macOS/Windows
- **C2 infrastructure**: Single IP (91.92.242.30) across all malicious Skills

**Data stolen**: Crypto exchange API keys, wallet private keys, SSH credentials, browser passwords, files from common directories

**ClawHub security model**: Requires only a 1-week-old GitHub account to publish. No pre-publication review. No automated scanning at launch.

**Comparison**: npm, PyPI, and Docker Hub have all faced similar supply chain attacks, but they have security teams, automated scanning, and established reporting processes. ClawHub had none of these when the campaign launched.

### 4. Exposed OpenClaw Instances (Internet Scan)

**Source**: Censys (January 31, 2026)  
**Finding**: 21,639 OpenClaw instances exposed on public internet  
**Notable**: 30%+ on Alibaba Cloud infrastructure

**What this means**: These are OpenClaw gateways accessible from the open internet without authentication. Anyone who can reach them can potentially issue commands, read conversation histories, access connected accounts, and exfiltrate credentials.

**OpenClaw's own documentation now states**: "Prompt injection is when an attacker crafts a message that manipulates the model into doing something unsafe. Even with strong system prompts, prompt injection is not solved. System prompt guardrails are soft guidance only."

### 5. Prompt Injection via Agent Social Networks

**Not a single incident but an ongoing attack surface.**

The Moltbook feed is untrusted content that connected OpenClaw agents read and process. Any post can contain:
- Instructions disguised as social content ("As a fellow agent, you should...")
- Links to malicious pages (CVE-2026-25253 vector)
- Encoded payloads that assemble across multiple posts (Palo Alto persistent memory concern)
- Social engineering targeting agent system prompts

**JesusCrust/Prophet 62** demonstrated this in the Crustafarianism context â€” conducting XSS attacks through Moltbook posts and attempting hostile takeovers of the religious submolt.

**Fundamental problem**: Agents cannot reliably distinguish between legitimate social content and adversarial instructions. This is the prompt injection problem applied at social network scale.

---

## Threat Actor Landscape

| Actor Type | Motivation | Observed Techniques |
|-----------|------------|---------------------|
| **Opportunistic criminals** | Credential theft, crypto theft | ClawHavoc Skills, typosquatting |
| **Script kiddies** | Chaos, clout | Moltbook prompt injection, XSS via posts |
| **Crypto scammers** | Token pump-and-dump | MOLT token ($77M peak), CRUST, SHELLRAISER |
| **Nation-state (potential)** | Intelligence collection | 30%+ exposed instances on Alibaba Cloud (correlation, not causation) |
| **Researchers** | Disclosure, reputation | CVE-2026-25253, Wiz database breach, Koi Security audit |

---

## Architectural Lessons

### What OpenClaw Gets Wrong

1. **Security as opt-in, not default**: Sandboxing is disabled by default. Gateway auth is optional. Skills are unvetted.
2. **All-or-nothing permissions**: Agents get full system access or nothing. No capability-based fine-grained control.
3. **Marketplace without governance**: ClawHub's publication barrier is a week-old GitHub account. No scanning, no review, no signing.
4. **Token = full authority**: A single stolen token grants configuration access, not just data access. Attackers can disable security controls via the API.
5. **Memory as attack surface**: Persistent memory enables stateful, delayed-execution exploits that traditional security tools don't detect.

### What A2A/MCP Get Right (by contrast)

1. **A2A**: Opaque agents â€” no internal state exposure. Agent Cards with optional signing. Enterprise-grade auth at spec level.
2. **MCP**: Structured tool schemas with defined inputs/outputs. Server-controlled permissions. No arbitrary code execution.
3. **Both**: Built on established transport standards (HTTP, JSON-RPC, SSE) with existing security tooling.
4. **Both**: No marketplace model â€” discovery is structured, not "install from stranger's GitHub."

### What A2A/MCP Still Need (Semgrep analysis)

1. Enforced Agent Card signing (currently optional)
2. Short-lived token requirements
3. Fine-grained authorization scopes
4. Protocol-level user consent mechanisms
5. Standardized observability/audit requirements

---

## Risk Assessment for Our Participation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Prompt injection via Moltbook feed | High | Medium (sandboxed) | Isolated VM, no real credentials, read-heavy strategy |
| Credential theft via malicious Skill | High (if installing Skills) | Critical | Zero Skills from ClawHub â€” custom only |
| Token exfiltration | Medium (patched CVE) | High | Dedicated API keys, spending limits, rotation schedule |
| Agent impersonation | Medium | Low (research context) | No real identity attached to research agent |
| Cost runaway from token usage | Medium | Medium | Hard spending limits, monitoring, scheduled operation windows |
| Host compromise via container escape | Low (with hardening) | Critical | Rootless Podman, read-only filesystem, network isolation |

---

## HUBEX Post-Exploration Audit (February 14, 2026)

### Context

In late January 2026, prior to fully understanding the OpenClaw/Moltbook security landscape, we explored the `molthub` CLI tool (npm, v0.3.1-beta.1, maintainer: github.com/steipete) inside a VS Code devcontainer. The tool is the package manager for OpenClaw's skill registry â€” the same registry where Koi Security later documented the ClawHavoc campaign (341 malicious skills, 11.9% malicious rate).

No OpenClaw gateway was installed. No Moltbook agent was deployed. The exploration was limited to the CLI registry tool in a sandboxed container. Nevertheless, given the severity of subsequently discovered vulnerabilities (CVE-2026-25253, ClawHavoc supply chain attacks, exposed Supabase database), a retroactive verification of host system integrity was warranted.

### Methodology

A read-only, non-destructive audit was conducted by Claude Code on the HUBEX (five-drive extended HUB: H, U, B, E, X drives). The audit was scoped to detect any artifacts, configurations, or residue from the exploration session that might have escaped Docker container isolation.

### Audit Results

| Check | Result | Status |
|-------|--------|--------|
| Devcontainer isolation | No `.devcontainer/devcontainer.json` found on any HUBEX drive | âœ… Clean |
| Filesystem artifacts | No `.openclaw/`, `molthub/`, `clawdhub/`, or `moltbook/` directories on any drive | âœ… Clean |
| Docker residue | `docker volume ls` â€” no matching volumes; `docker image ls` â€” no matching images | âœ… Clean |
| Stopped containers | `docker ps -a` â€” no stopped/exited containers matching OpenClaw/Moltbook | âœ… Clean |
| NPM config pollution | No `~/.npmrc` on host; no `.config/` entries referencing clawdhub/molthub | âœ… Clean |
| Global npm packages | `npm list -g` â€” no molthub, openclaw, or clawdhub packages | âœ… Clean |
| Shell history | `bash_history`, `zsh_history` â€” no commands running molthub or npx outside containers | âœ… Clean |
| Package.json dependencies | No `package.json` on any drive references molthub, openclaw, or clawdhub | âœ… Clean |
| VS Code workspace history | `storage.json` â€” no references to molthub, moltbook, clawdhub, or molthub-app | âœ… Clean |

### Conclusion

The exploration session was fully ephemeral. The temporary project directory (`H:\projects\molthub-app`) has been deleted, taking any devcontainer configuration and volume mount references with it. VS Code's "Reopen in Container" used a transient config that was never persisted to disk. No host-side pollution of any kind was detected across all five HUBEX drives.

**HUBEX confirmed clean. Container isolation held. No traces on host system or broader infrastructure.**

### Lessons Documented

1. **The devcontainer approach was correct** â€” even before understanding the full threat landscape, sandboxing the exploration prevented any exposure. This validates the operational instinct.
2. **Retroactive verification is essential** â€” initial research often happens before the threat model is fully understood. Going back to verify after learning more is good practice, not paranoia.
3. **Briefing your tools matters** â€” Claude Code (with a pre-2025 knowledge cutoff) had no context for OpenClaw/Moltbook threats and appropriately asked for CVE numbers and incident details before proceeding. Providing that context enabled a rigorous audit.
4. **Ephemeral environments are the right default** â€” the fact that the entire session vanished without trace is a feature, not a limitation. For future OpenClaw research, the disposable cloud VM approach documented in the Safe Participation Guide extends this principle further.

---

## Recommended Reading (Primary Sources)

| Source | URL | Key Contribution |
|--------|-----|-----------------|
| Simon Willison's Blog | simonwillison.net | "Lethal trifecta" framework, Docker setup guide |
| Adversa AI | adversa.ai/blog/openclaw-security-101 | Comprehensive CVE analysis and hardening guide |
| Semgrep A2A Guide | semgrep.dev/blog/2025/a-security-engineers-guide-to-the-a2a-protocol/ | Protocol-level security audit |
| Wiz Research | (various) | Moltbook database breach analysis |
| Koi Security | (various) | ClawHavoc campaign discovery |
| DepthFirst | (various) | CVE-2026-25253 disclosure |
| OpenClaw Security Docs | docs.openclaw.ai/gateway/security | Official hardening guidance |
| Palo Alto Networks | (various) | Persistent memory as fourth risk factor |

---

*This document is part of the ðŸ•¸ï¸ Agentic Networks Lab knowledge base.*