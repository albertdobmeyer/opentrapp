# OpenTrApp: Defense-in-Depth Containment for an Autonomous AI Agent

**A practical perimeter architecture for running [OpenClaw](https://www.getopenclaw.ai) on a personal computer without unrestricted host access.**

Albert Dobmeyer, independent
albertdobmeyer@proton.me

Version 1.0 · 2026-05-04 · Companion repository: [github.com/albertdobmeyer/opentrapp](https://github.com/albertdobmeyer/opentrapp)

---

## Abstract

Autonomous AI agents that can execute shell commands, read user files, browse the web, and load third-party skills present a containment problem of practical concern: their default operating mode grants them the same operating-system privileges as the human user, so any compromise — through prompt injection, a malicious skill, or a flaw in the agent itself — translates directly into damage to the user's host. Existing hardening guidance for the open-source OpenClaw runtime treats containment as a configuration problem (set `tools.deny`, set `sandbox.mode`, restrict the API key); empirical evidence suggests this layer alone is insufficient. The ClawHavoc study of 2026-Q1 classified 11.9 % of published ClawHub skills as malicious; CVE-2026-25253 demonstrated a one-click remote-code-execution vector through OpenClaw's own management API; the Moltbook database breach exposed 1.5 M API tokens via a single misconfiguration.

This paper describes OpenTrApp, a desktop application that runs the OpenClaw Clawbot inside a four-container security perimeter on the user's own computer. The architecture provides defense-in-depth across three independent threat categories (runtime compromise, supply-chain attack, hostile network or social-feed content), exposes a stateful adaptive shell that allows the agent's privilege level to be modulated per task context, and ensures that the user's API credentials are held by a dedicated proxy container and never reach the agent's runtime. Two design choices in particular distinguish this work from prior hardening guidance: *proxy-side credential injection*, which keeps the literal API-key value invisible to the agent regardless of compromise, and *Content Disarm and Reconstruction* applied to skills, which discards the original artefact and rebuilds a clean version from the parsed semantic intent.

The implementation runs in two-container (vault-agent + vault-proxy) and four-container (adding vault-forge for skill scanning and vault-pioneer for social-content analysis) configurations. As of v0.3.0, the perimeter passes a 24-point startup verification, a 42-check manifest-orchestration suite, and a 25-test end-to-end browser harness. The application owns the perimeter's lifecycle: every termination path validated in dogfooding (graceful exit, SIGTERM, SIGINT, SIGKILL, OS reboot) tears down the perimeter cleanly or recovers it on next launch.

The design does not eliminate the residual risk inherent in running an autonomous AI agent on a personal computer; it raises the cost of compromise and gives the user a small, verifiable surface to reason about.

---

## 1. Introduction

The OpenClaw open-source agent runtime, released in early 2026, is a CLI-installable autonomous AI assistant capable of tool use, persistent memory, shell-command execution, browser control, and on-demand loading of third-party skills from a public registry (ClawHub). Its design objective — making advanced agent capabilities accessible to non-developer users — is achieved by minimising configuration: a single `openclaw.json` file controls the agent's tool surface, allowed network destinations, and authentication credentials; defaults grant the agent the privileges of the user that ran the binary.

This default operating mode is hazardous on a personal computer. The agent runs *as the user*: it can read every file the user can read, write every file the user can write, send messages on every channel the user has authenticated, and install software with the user's authority. A flaw in any layer — the agent's own reasoning, a tool the agent loads, a piece of content it fetches — translates into an operation on the user's host with the user's privileges.

Empirical evidence from the first quarter of OpenClaw's existence makes the cost of this attack surface concrete:

- **CVE-2026-25253** demonstrated a one-click remote-code-execution path through OpenClaw's own management API; an attacker with the user's session token could disable the in-runtime sandbox and execute arbitrary shell commands.
- **The ClawHavoc study** (2026-Q1) found that 341 of 2,857 published ClawHub skills (11.9 %) carried malicious payloads — credential exfiltration, persistence mechanisms, command-and-control beacons — distributed under names that resembled legitimate tooling.
- **The Moltbook database breach** exposed 1.5 M API tokens, 35 K e-mail addresses, and direct messages via a single Supabase instance with row-level security disabled; exploitation required less than three minutes from disclosure.
- **A public-internet scan** counted 21,639 OpenClaw instances reachable from the open web, the majority without authentication.

In response, several "hardening guides" for OpenClaw appeared on personal blogs and security-aware communities. Most treat containment as a *configuration* problem: tighten `tools.deny`, restrict `proxy.allowlist`, enable `sandbox.mode`, store the API key only in `.env`. This guidance is not wrong — every recommendation reduces attack surface — but it is insufficient. A configuration-only approach has three properties that prevent it from being a complete answer:

1. **Single-layer.** A misconfiguration, a regression, or a vulnerability in any one of the configurable layers exposes the entire surface beneath it.
2. **Self-modifying.** The agent has tools that can edit its own configuration. A successful prompt injection or skill exploit can rewrite `tools.deny` and immediately enable what was previously disabled.
3. **Credential-adjacent.** Storing the API key in an environment variable inside the same process the agent runs in places the credential within reach of any process compromise.

This paper describes a different approach: a four-container *perimeter* that runs alongside the OpenClaw runtime, owns the user's credentials in a separate sidecar process, mediates every outbound request, scans every loaded skill before the agent sees it, and provides defense-in-depth that survives a single-layer compromise. The perimeter is presented to the user through a Tauri desktop application that owns the perimeter's lifecycle: start the application, the perimeter comes up; close the application, the perimeter cleanly tears down; OS-level kill, the perimeter is reaped on next launch.

The contribution of this work is not a single novel mechanism. The individual primitives — Linux capability dropping, seccomp filtering, network isolation, allowlist proxies, content sanitisation — are each well established. The contribution is the *composition*: these primitives integrated into a coherent perimeter that an end user can install with a setup wizard, control from a Telegram bot, and reason about with a single clearly-bounded surface (`compose.yml` plus three component manifests). Two specific composition choices are non-obvious in a way that warrants detailed treatment in §5 and §6: *proxy-side credential injection* and *Content Disarm and Reconstruction applied to skills*.

The remainder of the paper is organised as follows. §2 specifies the threat model. §3 presents the system design and trust tiers. §4 describes the defense-in-depth layers per threat category. §5 details the adaptive-shell-level mechanism. §6 details the CDR pipeline. §7 discusses implementation choices. §8 reports empirical evaluation. §9 enumerates known limitations and the residual risks the perimeter does not address. §10 surveys related work. §11 concludes with future work.

---

## 2. Threat model

The perimeter is designed to address five attacker capabilities, summarised below. A complete attacker-capability matrix with explicit residual-risk annotations is queued as a separate document (see [`docs/roadmap-post-launch.md`](roadmap-post-launch.md) §1).

**T1: Prompt-injection author.** An attacker who controls content the agent will eventually see — a Telegram message body, a fetched URL response, a piece of feed content. Capability: cause the agent to execute any tool the agent currently has enabled. Mitigation goal: limit the *set* of enabled tools (adaptive shell, §5) and the *destinations* tools can reach (proxy allowlist, §4.1).

**T2: Malicious skill author.** An attacker who publishes a skill on ClawHub designed to look useful and carry a payload. Capability: gain code execution inside the agent's runtime when the skill is loaded. Mitigation goal: prevent malicious skills from reaching the agent's runtime in usable form (vault-forge scanner + CDR pipeline, §6) and limit the blast radius of any skill the scanner missed (container hardening, §4.2).

**T3: Network man-in-the-middle.** An attacker positioned between vault-proxy and the public internet (e.g. a compromised local network or a hostile DNS). Capability: read or modify the agent's outbound traffic. Mitigation goal: TLS termination of all outbound calls inside vault-proxy with certificate pinning where supported; egress restricted to a small allowlist so a successful MITM is detectable.

**T4: Compromised host.** An attacker who already has partial access to the user's machine — a malicious binary the user installed before this perimeter, an OS-level rootkit, or another foothold. Capability depends on the attack vector but is generally significant. Mitigation goal: explicitly out of scope. The perimeter assumes the host is honest; if the host is dishonest, no container-level isolation can recover security. Users requiring stronger isolation are directed to run the perimeter on a disposable virtual machine (see [`components/opencli-container/README.md`](../components/opencli-container/README.md) § Isolation tiers).

**T5: Hostile end user.** A human (the legitimate user, or someone who has gained access to the user's Telegram account) who instructs the agent to perform a damaging action. Capability: same as the legitimate user's authenticated capability surface. Mitigation goal: explicit user approval gates on destructive operations (Hard / Split shells, §5); explicit recommendation that the Telegram account paired with the agent be a *dedicated* account rather than the user's personal account.

T1 and T2 are the principal threats the architecture is engineered against. T3 is a secondary concern addressed structurally (only one egress path, all of it logged). T4 is explicitly out of scope; the architecture does not promise mitigation when the host is compromised, and the documentation says so. T5 is addressed by user-experience measures rather than mechanical isolation.

A planned threat-model document (see [`docs/roadmap-post-launch.md`](roadmap-post-launch.md) §1) extends this summary into a STRIDE-style attacker-capability matrix with explicit residual-risk annotations and pointers to the empirical evidence supporting each mitigation.

---

## 3. System design

### 3.1 Trust tiers

The architecture organises components into three trust tiers, each with a single well-scoped responsibility:

- **Tier 1 (trusted).** Components running on the user's host with full filesystem and network access: the user, an optional trusted CLI coordinator (e.g. Anthropic's Claude Code), and the OpenTrApp desktop GUI. Tier 1 makes decisions and issues commands.
- **Tier 2 (infrastructure).** The container perimeter. Enforces boundaries mechanically; does not make security decisions. Implemented by OpenTrApp's compose orchestration plus the four `vault-*` containers.
- **Tier 3 (contained).** The OpenClaw agent process, the Telegram gateway, loaded skills, and any fetched network content. Performs the work the user requests, within the boundaries Tier 2 enforces.

The separation matters because each tier has a different *kind* of failure mode. Tier 1 fails through human error or through a compromise of the coordinator's reasoning. Tier 2 fails through misconfiguration of the compose stack or a kernel-level escape. Tier 3 is *expected* to fail (the agent is the contained dangerous element); the architecture is designed assuming it will. Crossing two tiers' failure modes simultaneously is required for an end-to-end compromise — a property defense-in-depth makes plausible.

### 3.2 Container topology

The four containers and their internal-network connectivity are summarised below; full detail is in [`docs/trifecta.md`](trifecta.md) and the live `compose.yml`.

- **vault-agent** runs the OpenClaw runtime, the Telegram gateway, and any loaded skills. Read-only root filesystem, all Linux capabilities dropped, custom seccomp profile, no host filesystem mounts, no host network access. A single workspace directory is the only writable surface.
- **vault-forge** scans skills downloaded from ClawHub against an 87-pattern catalogue, runs a zero-trust line-classifier over each file, and rebuilds the skill from parsed semantic intent (CDR; §6). Output is delivered to vault-agent through a write-only shared volume; vault-forge has no path to vault-agent.
- **vault-pioneer** was designed to scan posts on the Moltbook agent social network for prompt-injection patterns before the content reached vault-agent. Following Meta's acquisition of Moltbook on 2026-03-10 and the resulting API instability since 2026-04-05, the module is **parked**: the container is still defined in `compose.yml` for completeness but has no functional API to talk to.
- **vault-proxy** is the perimeter's only egress to the public internet. It holds the user's API credentials in environment variables visible only to itself, replaces a placeholder string in outbound headers with the literal credential immediately before forwarding, enforces a per-host allowlist, and writes a structured request log to a host-readable volume.

The four containers each run on their own Docker `internal: true` network. Only `vault-proxy` bridges them. `vault-agent` cannot reach `vault-forge` or `vault-pioneer` through any routed path; the only means by which a certified skill reaches `vault-agent` is the write-only shared volume `forge-deliveries`.

### 3.3 Lifecycle ownership

The OpenTrApp desktop application owns the perimeter's lifetime. Application start triggers `compose up -d` on a background thread; graceful exit (window close, tray Quit, SIGTERM, SIGINT) triggers `compose down` synchronously with a 30-second ceiling; following SIGKILL, orphan containers are detected and stopped on the next launch via a PID-file mechanism (`RunGuard`). A user-initiated *paused* state allows the perimeter to be suspended without exiting the application; the state survives application restart via the `~/.opentrapp/paused` marker file. Seven distinct termination paths were validated as cleanly tearing down the perimeter in the dogfood walkthrough (§8).

This is non-obvious in a way that affects security: a configuration-only hardening guide cannot make the same guarantee. The OpenClaw runtime configured in a hardening blog post is started at the user's discretion and stopped (or not) at the user's discretion; an enthusiastic user who clicks Quit may leave the runtime active in the background indefinitely, and a system that crashes leaves no record. The application-owns-perimeter discipline turns "is the agent currently exposed?" from a question of user vigilance into a structural property of the application's process lifetime.

---

## 4. Defense-in-depth

Each of the three principal threat categories is mitigated by multiple independent layers. A single layer's failure does not produce an end-to-end compromise. The full layer-by-layer tables are in [`docs/trifecta.md`](trifecta.md) §7; this section summarises the structure and the rationale for layer ordering.

### 4.1 Compromised agent (T1, runtime category)

The compromise scenario assumes the agent itself or one of its tools has been hijacked and is now executing on behalf of an attacker. Six layers stand between the compromise and damage to the user's host:

1. **Container hardening** (vault-agent): read-only root filesystem, dropped capabilities, seccomp, no-exec mounts, PID and memory limits.
2. **Network proxy** (vault-proxy): domain allowlist, payload-size limits, request logging.
3. **Tool policy** (vault-agent's OpenClaw config): denied tools never enter the LLM's tool catalogue.
4. **Exec controls** (vault-agent): safelisted binaries plus per-action human approval.
5. **Workspace restriction** (vault-agent): `workspaceOnly: true` — no path traversal outside the workspace.
6. **Kill switch** (host → container management): graceful stop, hard kill, full perimeter teardown.

The ordering is significant: the cheapest layer to bypass is listed first. If layer 3 (tool policy) is somehow bypassed and the agent gains a denied tool, layer 1 (container hardening) still prevents the resulting code from writing to the root filesystem, layer 2 (network proxy) still prevents it from making non-allowlisted egress, and so on. The attacker must defeat all six independent mechanisms to translate an in-runtime compromise into damage on the host.

### 4.2 Malicious skill (T2, supply chain)

Skills enter the agent's workspace through `vault-forge`. Six layers gate the path:

1. **Static scanner**: 87 patterns mapped to MITRE ATT&CK techniques, applied to every file in the skill bundle (not just `SKILL.md`).
2. **Line verifier**: every line of every file classified as `SAFE` / `SUSPICIOUS` / `MALICIOUS`. A single `SUSPICIOUS` or `MALICIOUS` line quarantines the entire skill.
3. **Content Disarm and Reconstruction**: the original skill is discarded; a clean version is rebuilt from the parsed semantic intent. The original artefact never reaches the agent. (See §6.)
4. **Domain allowlist** (vault-proxy): ClawHub registry domains denied by default; the user must explicitly opt in.
5. **Network isolation**: vault-forge has no routed path to vault-agent; certified skills are delivered through a write-only shared volume.
6. **Container hardening** (vault-agent): limits the blast radius of an undetected malicious skill.

Layer 3 (CDR) is the architectural innovation. A static scanner with N patterns will eventually miss attack pattern N+1; a line classifier will eventually mis-classify a sufficiently sophisticated obfuscation. Both are necessary; neither is sufficient. CDR adds a different *kind* of mitigation: rather than asking *"is this artefact safe?"* (an answer that is only as good as the catalogue of known badness), it asks *"can the artefact's intent be re-expressed in a known-safe form?"* — and if the answer is yes, the original artefact is discarded. Novel attacks the scanner does not yet recognise are mitigated structurally. Detail in §6.

### 4.3 Hostile feed content (T3, network/social)

The pioneer container was designed to address this category; it is currently parked (§3.2). The architectural slot remains:

1. **Feed scanner** (vault-pioneer): 25 prompt-injection patterns calibrated against the early-2026 Moltbook ecosystem.
2. **Network isolation**: pioneer cannot reach vault-agent through any routed path.
3. **DM pairing policy**: each Telegram counterpart explicitly approved.
4. **Tool policy**: the agent has no tools that fetch from Moltbook absent explicit user enablement.
5. **Coordinator approval**: the user retains visibility on every privileged action.

Layers 3–5 are functional regardless of pioneer status. Layers 1 and 2 are dormant pending a stable target API.

---

## 5. Adaptive shell levels

The agent's operating capability is treated as a *system state*, set per-context, rather than a single configuration value chosen at install time. Three states are defined; transitions between them are governed by who can initiate them and what additional checks they entail.

| Level | Allowed operations | Denied (in addition to permanently-protected resources) |
|-------|--------------------|----------------------------------------------------------|
| Hard Shell | Telegram chat only | All command execution, file I/O, web access, skill loading |
| Split Shell (default) | File read/write within the workspace; safelisted commands with per-action approval | Commands not on the safelist; arbitrary network fetches |
| Soft Shell | Web browsing, autonomous safelisted commands, scheduled tasks, the broader OpenClaw tool surface | Host-level resources, credential stores, administrative operations — denied at every level |

Three properties of this design distinguish it from a simple "tools toggle":

**(a) The shell is a system state, not a setting.** It can be changed at runtime by a privileged actor (the user, or a trusted CLI coordinator). The agent itself cannot promote its own shell level; promotion is always initiated from Tier 1. Demotion ("shell up") is always permitted; promotion ("shell down") requires explicit confirmation.

**(b) The shell state is observable from outside the agent's runtime.** A reader of the host filesystem can determine which shell level is currently active by reading `~/.openclaw/openclaw.json` inside the vault-agent volume. This means the state is auditable: a user can verify after the fact what the agent was permitted to do at the time of any logged action.

**(c) The shell state does not weaken the universal hardening.** Container-level mechanisms (read-only root, dropped capabilities, seccomp, network policy) hold at every shell level. The shell level controls only what the agent's *tool layer* permits; it cannot weaken the *infrastructure layer*. This means the worst-case behaviour at Soft Shell — autonomous web browsing, autonomous command execution from an extended safelist — remains constrained to operations the container can perform. A Soft-Shell agent that successfully exfiltrates the user's SSH keys must first defeat the container's read-only-root and capability-drop policies; the shell level is irrelevant at that layer.

The case for treating the shell as a state rather than a setting is the standard argument for context-appropriate authorisation: the level of privilege the agent should hold while answering a factual question is not the level it should hold while installing a skill. A static configuration must compromise: either restrict the agent so far that it cannot install skills, or permit skill installation and accept that privilege at every other moment. The state model lets the privilege track the task.

The full mechanism is documented in [ADR-0002](adr/0002-adaptive-shell-levels.md). The implementation lives in [`components/opencli-container/scripts/tool-control.sh`](../components/opencli-container/scripts/tool-control.sh).

---

## 6. Content Disarm and Reconstruction

Skills enter the perimeter as opaque artefacts: a `SKILL.md` file plus optional supporting files, downloaded from a third-party registry (ClawHub) over HTTPS. The empirical baseline (ClawHavoc's 11.9 % malicious-skill rate) means *every* incoming skill must be treated as potentially hostile.

A pure-blocklist scanner with N patterns has a structural limit: it detects *known* attacks. Pattern N+1 — a novel obfuscation, a new persistence mechanism, a category of injection nobody has yet enumerated — slips through. Augmenting the scanner with a zero-trust line classifier (every line must match a known-safe pattern, otherwise the file is quarantined) raises the bar significantly: now an attacker must construct their payload using only line-shapes the classifier has seen before. But neither layer is sufficient on its own, and stacking them still asks the same question — *"is this artefact safe?"*.

Content Disarm and Reconstruction inverts the question. Instead of asking whether the artefact is safe, the pipeline asks whether its *intent* can be re-expressed in a known-safe form. The original artefact is treated as a parse target, not as the deliverable. The pipeline runs in `vault-forge` as follows:

```
       download                   parse intent             rebuild from intent
ClawHub ────────► quarantine ────► structural model ────► clean skill artefact
                  (vault-forge)    (parser, in-container) (generator, in-container)
                       │                                            │
                       ▼                                            ▼
                  static scanner                              line verifier
                  (87 patterns,                               (every line classified;
                   MITRE ATT&CK)                               failure quarantines)
                       │                                            │
                       └─── sign clearance report ─── deliver to vault-agent ───►
                                              (write-only shared volume; SHA-256;
                                               vault-agent reads, verifies,
                                               loads only on signature match)
```

Five properties of this design matter:

1. **The original artefact is never delivered.** Whatever side effects the original `SKILL.md` was supposed to produce (intentionally or by accident) are replaced by the side effects of the rebuilt version. An attacker who hides a payload in formatting tricks, in trailing whitespace, in `<!-- HTML comments -->`, in zero-width Unicode characters, in layered base-64 encoding — none of those survive the parse-and-rebuild step.
2. **The rebuilt artefact is signed.** A SHA-256 clearance report accompanies every delivery. `vault-agent` rejects skill artefacts that do not match the signed hash. A skill that the user side-loads into the workspace bypassing forge will fail this check and be refused.
3. **The pipeline is deterministic per input.** Re-running CDR on the same source produces the same rebuilt artefact (modulo intentional model-output rebases when the parser improves). This makes the pipeline auditable: a reviewer can verify post-hoc which input produced which output.
4. **The pipeline runs offline.** No network access during scan, classify, parse, or rebuild. The scanner pattern catalogue, the safe-line classifier, and the parser are all in-image; the only external interaction is the initial download into quarantine.
5. **Failure quarantines.** A line that fails classification, a parse that fails completion, or a static-scanner hit at `CRITICAL` severity all result in the same outcome: the skill is held in quarantine and not delivered. The agent learns nothing about the failed skill except that it failed.

The full mechanism is documented in [ADR-0003](adr/0003-content-disarm-reconstruction.md). The pipeline is implemented in [`components/openskill-forge/tools/skill-cdr.sh`](../components/openskill-forge/tools/skill-cdr.sh) and the `tools/lib/cdr-*` scripts.

A skill scanner with 87 patterns plus a line classifier is well-established prior art in the supply-chain-security community (see e.g. enterprise tools such as Sonatype Nexus, Snyk, and Checkmarx for analogous layers in the npm/pip ecosystems). The composition of those layers with CDR — discarding the original artefact and rebuilding from intent — is, to our knowledge, novel in the AI-skill-distribution context. It is the architectural choice in this work most likely to be applicable to other agent-skill-registry ecosystems beyond OpenClaw / ClawHub.

---

## 7. Implementation

The desktop application is implemented in Tauri 2 with a React 18 / TypeScript frontend and a Rust backend. The Tauri choice over Electron was driven by binary size (a Tauri release bundle is 5–10 MB compared with Electron's typical 80–150 MB), memory footprint (the Rust backend is significantly smaller than Node), and the desire for a small attack surface in the application that owns the perimeter's lifecycle. The Rust backend houses the manifest orchestration logic, the perimeter lifecycle controller, and the mitmproxy-addon supervisor.

The four components — vault-agent, vault-forge, vault-pioneer, vault-proxy — are each defined by a `component.yml` manifest that conforms to a JSON Schema (`schemas/component.schema.json`). The manifest declares the component's identity, status states, runnable commands, editable configuration files, health probes, and workflows. The Tauri backend reads these manifests at startup; the React frontend renders dashboards generically from them. This generic-backend constraint — the application reads manifests and executes what they declare; it does not contain component-specific logic — is the central composability property that lets the four-container perimeter be reasoned about without hard-coding component knowledge into the orchestrator.

Schema alignment across three implementations (the JSON Schema, the Rust serde structs, the TypeScript types) is verified by [`tests/orchestrator-check.sh`](../tests/orchestrator-check.sh) on every commit. The 42-check suite covers manifest parsing, cross-reference validation (commands referenced from workflows, states referenced from `available_when`, orchestrator-workflow steps referencing component commands), and frontend-backend command parity (every Rust command handler has a matching TypeScript invoke wrapper).

The perimeter lifecycle is owned by the Tauri backend. The application registers handlers for `RunEvent::Exit` (graceful exit), `SIGTERM`, and `SIGINT`, each of which runs `compose down` synchronously with a 30-second ceiling. Following `SIGKILL`, the next launch reads `~/.opentrapp/runguard.pid`, observes that the previous PID is no longer alive, and runs `compose down` to reap any orphan containers before bringing the perimeter back up. A user-initiated *paused* state is persisted as a marker file (`~/.opentrapp/paused`); the perimeter is not auto-started while the marker is present. The status aggregator (a Tokio interval task) re-evaluates the assistant's status — `ok` / `starting` / `recovering` / `error_perimeter` / `error_key` / `paused_by_user` / `not_setup` — every 60 seconds and emits a Tauri event on transition; the frontend Home view's hero state machine subscribes to this event.

The Anthropic-key validity check is performed against the free `/v1/models` endpoint (rather than the billable `/v1/messages` endpoint), with a five-minute TTL cache that is invalidated on key rotation. This is a small but operationally significant implementation choice: the alternative — pinging `/v1/messages` — would charge the user a fraction of a token per minute for the lifetime of the application, which would be a non-zero ongoing cost even when the user is not actively using the agent.

---

## 8. Empirical evaluation

The perimeter's correctness is verified at three scales: a per-startup container-hardening check, a per-commit manifest-orchestration suite, and an end-to-end browser harness that exercises the desktop application's user-facing surfaces.

**Container hardening (24-point verification).** [`components/opencli-container/scripts/verify.sh`](../components/opencli-container/scripts/verify.sh) runs at container startup and on demand. The 24 checks fall into three groups: 14 universal-hardening checks (proxy DNS, proxy TCP, read-only root, capabilities dropped, no host mounts, no Windows interop, API-key absence from `vault-agent`'s environment, Docker socket not mounted, sudo unavailable, non-root user, seccomp loaded, `noexec` on `/tmp`, `no-new-privileges` set, PID limit), 4 shell-specific checks (profile matches active shell, exec security matches, host and elevated controls correct, safe-binary list matches profile), and 6 per-tool security checks (permanently-denied tools denied, `rm` not in safebins, no interpreters in safebins, proxy allowlist clean, risk score in range, configuration-integrity hash matches startup snapshot). All 24 checks pass in the released v0.3.0 configuration.

**Manifest orchestration (42-check suite).** [`tests/orchestrator-check.sh`](../tests/orchestrator-check.sh) runs in CI on every push and pull request. The 42 checks cover repository structure, JSON Schema validity, manifest parsing across all three components, submodule synchronisation, build artefacts, frontend-backend command parity, manifest enum alignment with Rust serde expectations, prerequisites cross-references, and workflow step → command references. The suite reports zero warnings in the released configuration.

**End-to-end browser harness (25 tests).** Playwright tests in [`app/e2e/`](../app/e2e/) exercise the wizard flow, the four user-mode pages, and the banned-term enforcement (28 developer-jargon terms verified absent from user-visible text). The full suite runs under one minute and is part of the standard pre-commit gate.

**Frontend unit tests (74 tests).** Vitest tests in [`app/src/`](../app/src/) cover hooks, settings persistence, error classification, and component rendering. TypeScript strict mode is verified separately.

**Backend unit tests (56 tests).** Rust tests in [`app/src-tauri/src/`](../app/src-tauri/src/) cover the manifest parser, the lifecycle module, the status aggregator (including the seven-state transition matrix), and the secret-redaction utility. All tests run without containers.

**Live end-to-end verification (Pass 1.5).** A separate Telethon-based harness in [`tests/e2e-telegram/`](../tests/e2e-telegram/) drives the agent from a real Telegram account against a running perimeter and observes responses. The harness produced eight Karen-persona scenarios in the v0.3.0 verification run, with combined Anthropic API spend under $0.04. No security regressions were observed; the test produced new banned-term findings (e.g. raw OpenClaw tool names appearing in bot replies under specific prompt shapes) which were added to the regression suite.

The Pass 8 pre-ship audit ([`docs/specs/2026-05-02-pass-8-preship-walk.md`](specs/2026-05-02-pass-8-preship-walk.md)) walked every reachable user-facing surface against a 13-principle UX rubric and against a "deserve-to-exist" scope test. Eight of nine non-wizard user surfaces scored ≥ 8.5 / 10 on the rubric; the ninth (Telegram first-chat at 8.4) sits in a separate repository's system prompt and is tracked there. The audit issued a SHIP recommendation; v0.3.0 was tagged on 2026-05-02.

---

## 9. Limitations

The architecture is honest about what it does not address.

**The host is trusted.** Container-level isolation does not survive a compromised kernel. A Linux-kernel zero-day (uncommon but not impossible) defeats the perimeter at every layer. Users who require stronger isolation are directed to run the perimeter on a disposable virtual machine with a disposable API key and a hard spending cap. The README and the per-module documentation make this trade-off explicit.

**The reasoning is not local.** OpenClaw's reasoning runs on Anthropic's API. Operating OpenTrApp without internet access to Anthropic is not supported. This is a non-negotiable property of OpenClaw itself, not of this perimeter.

**Allowlisted destinations can be abused during a live session.** A compromised agent can issue arbitrary API calls to the allowlisted hosts using the proxy-injected credentials. It cannot read the literal key value, but it can use it. Mitigation: configure a hard spending cap on the API key and treat the cap as part of the security boundary, not as a billing convenience.

**The proxy holds the key.** A compromise of `vault-proxy` exposes the credential. The proxy is hardened with read-only root, dropped capabilities, no-new-privileges, memory and PID limits, and a custom seccomp profile narrower than mitmproxy's default but broader than `vault-agent`'s. The trade-off — a wider syscall set than vault-agent in exchange for the ability to perform TLS interception — is documented in the residual-risks section of the vault README.

**Subdomain matching in the allowlist is implicit.** Allowing `github.com` also allows `api.github.com`. The default policy allows `raw.githubusercontent.com` but not `github.com` for this reason. Users adding domains to the allowlist must reason about whether a parent domain has subdomains that are exfiltration-capable.

**The Telegram control channel is a trust boundary.** A compromise of the operator's Telegram account permits an attacker to approve agent actions. The recommendation in every install path is to use a dedicated Telegram account, enable two-factor authentication, and treat its credentials as security-critical.

**Container destruction does not guarantee complete cleanup.** Layer caches, image metadata, and runtime logs persist on the host after `compose down --volumes`. These do not contain the API key (proxy-side injection guarantees that) but may contain conversation logs or activity metadata. Full cleanup requires `podman system prune -a` or the Docker equivalent.

**Pioneer is dormant.** The fourth container is currently parked following Meta's acquisition of Moltbook in March 2026. Threat category T3 (hostile network or social-feed content) is therefore mitigated only by the structural layers (network isolation, DM pairing policy, tool policy, coordinator approval) — the active feed-scanner layer is offline. Future revisions are gated on a stable target API or a successor agent-social-network platform.

**Installer signing is updater-only.** Builds are signed with the Tauri auto-updater key; OS-level code-signing certificates (Apple Developer ID, Windows Authenticode) are not currently in place. macOS Gatekeeper and Windows SmartScreen will warn on first launch.

---

## 10. Related work

**Container-based agent containment.** Several projects in early 2026 proposed Docker-based isolation for OpenClaw on personal computers. Most are configuration overlays on top of OpenClaw's own `sandbox.mode: "docker"` setting. The architectural choice in this work is layered: container isolation is one of six runtime defenses and is not load-bearing on its own. The proxy-side credential injection (§4.1 layer 2; ADR-0001) is, to our knowledge, not present in the published hardening guides surveyed during the design phase.

**Static skill scanning.** The npm and pip ecosystems have a mature literature on package-supply-chain analysis (Sonatype, Snyk, Checkmarx, OSV-Scanner, npm-audit). The 87-pattern catalogue in `vault-forge` is calibrated for AI-agent-skill-specific attack patterns rather than general code-execution malware; many of the patterns are nonetheless analogous in structure to npm-supply-chain-attack signatures.

**Content Disarm and Reconstruction.** CDR is well-established for office-document handling (Microsoft Office macros, PDF attachments, image-format exploits) in enterprise email-security products. Its application to AI-agent skills (Markdown-with-frontmatter artefacts containing structured executable instructions) is, to our knowledge, the first published instance.

**Adaptive privilege models.** Capability-based security and privilege bracketing have a long literature in operating-systems research (KeyKOS, EROS, Capsicum, gVisor). The shell-level model in this work is a coarse-grained capability bracketing applied at the AI-agent-tool layer rather than the OS-syscall layer; it is not novel in *concept* but is novel in *application context*.

**LLM-agent prompt-injection mitigations.** Simon Willison's "lethal trifecta" framework — private data, untrusted content, exfiltration capability — names the structural condition under which prompt injection becomes catastrophic. The perimeter design in this work prevents the trifecta from forming at the *infrastructure* layer: the agent has no direct private-data access (workspace-only filesystem) and no direct exfiltration capability (proxy-mediated egress with allowlist). Prompt-injection mitigation in the agent's own reasoning is out of scope; this perimeter assumes the agent's reasoning will eventually be subverted and is engineered for that case.

**The Trail of Bits "Building secure ML systems" line of work** and **OWASP's LLM Top 10** both inform the threat model in §2 and the residual-risks enumeration in §9. Specific reference points in the perimeter design that draw on these sources: the "always assume the model is compromised" framing (§2 T1), the "credential-adjacent" anti-pattern naming (§1), the explicit out-of-scope statement on hostile-host scenarios (§9 first paragraph).

A more thorough literature review and a per-alternative comparison matrix are queued as a separate document (see [`docs/roadmap-post-launch.md`](roadmap-post-launch.md) §5).

---

## 11. Conclusion and future work

This paper has described a four-container perimeter for running the OpenClaw autonomous AI agent on a personal computer, with defense-in-depth across three threat categories, a stateful adaptive-shell mechanism that allows agent privilege to track task context, and a Content Disarm and Reconstruction pipeline applied to skills downloaded from a third-party registry. The architectural choice that most distinguishes this work from prior hardening guidance is the *composition*: not the individual primitives (capability dropping, seccomp, allowlist proxies) but their integration into a coherent perimeter that an end user can install and reason about.

The design does not eliminate the risks inherent in running an autonomous AI agent on a personal computer. The host is trusted; the agent's reasoning runs on a remote API; the Telegram control channel is a trust boundary the user must protect; and a successful compromise of `vault-proxy` exposes the credential. These limitations are stated explicitly in §9 and in every install path's documentation.

Future work falls into three categories.

**Empirical validation.** A planned threat-model document (see [`docs/roadmap-post-launch.md`](roadmap-post-launch.md) §1) will extend the §2 summary into a full attacker-capability matrix with explicit residual-risk annotations and pointers to empirical evidence per cell. A reproducibility document (§4 of the same roadmap) will document the exact commands and expected outputs for every numerical claim in this paper.

**Architectural evolution.** A planned VM-isolation tier (Phase 2 of the vault module's roadmap) would address the "host is trusted" assumption for users requiring stronger isolation. The CDR pipeline (§6) is currently applied only to skills; extension to other untrusted-input categories (Telegram message bodies, fetched URL responses) is open research.

**Ecosystem.** Pioneer is parked pending a stable target API. If the agent-social-network category re-emerges (under Meta's continued operation of Moltbook, under a successor platform, or under an entirely new ecosystem), the architectural slot is preserved and re-activation requires only the API integration; the perimeter layer is in place.

The implementation is open-source under the MIT licence at [github.com/albertdobmeyer/opentrapp](https://github.com/albertdobmeyer/opentrapp) and the four-component family of repositories. All code, configuration, manifest schemas, verification scripts, and design documents are public. We invite review.

---

## References

The following are the primary references for the empirical claims and design choices in this paper. A full bibliography is queued as part of the §10 prior-art expansion ([`docs/roadmap-post-launch.md`](roadmap-post-launch.md) §5).

- **OpenClaw runtime.** [getopenclaw.ai](https://www.getopenclaw.ai). Source repository: openclaw/openclaw.
- **ClawHub registry.** [clawhub.ai](https://www.clawhub.ai).
- **CVE-2026-25253.** Reported 2026-01-31; one-click RCE through OpenClaw's management API.
- **ClawHavoc supply-chain study (2026-Q1).** 341 of 2,857 published ClawHub skills classified as malicious; full analysis at [`components/opencli-container/docs/research/`](../components/opencli-container/docs/research/) (companion repository).
- **Moltbook database breach (2026-01).** 1.5 M API tokens, 35 K e-mail addresses, direct messages exposed via Supabase row-level-security misconfiguration.
- **MITRE ATT&CK.** Used as the categorisation framework for the 87-pattern catalogue. [attack.mitre.org](https://attack.mitre.org).
- **Simon Willison, "The lethal trifecta for AI agents."** [simonwillison.net](https://simonwillison.net).
- **OWASP Top 10 for Large Language Model Applications.** Reference for the §2 threat model.
- **Tauri 2.** [tauri.app](https://tauri.app). Application framework.
- **mitmproxy.** [mitmproxy.org](https://mitmproxy.org). The basis for `vault-proxy`.
- **Companion architecture document.** [`docs/trifecta.md`](trifecta.md). Architecture, threat model, defense-in-depth tables, ownership matrix.
- **Architecture Decision Records.** [`docs/adr/`](adr/). ADR-0001 (proxy-side key injection), ADR-0002 (adaptive shell levels), ADR-0003 (CDR).
- **Pre-ship audit.** [`docs/specs/2026-05-02-pass-8-preship-walk.md`](specs/2026-05-02-pass-8-preship-walk.md). Empirical evaluation methodology and the SHIP recommendation rationale.

---

*This paper is published under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Source files are MIT-licensed.*
