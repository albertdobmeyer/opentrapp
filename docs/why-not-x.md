# Why not X? Prior-art comparison

**Document status:** Active
**Created:** 2026-05-04
**Companion documents:** [`whitepaper.md`](whitepaper.md) §10 (related work, narrative); [`threat-model.md`](threat-model.md) (the attacker model the comparisons below are evaluated against); [`trifecta.md`](trifecta.md) §7 (the defense-in-depth tables).

This document answers the question a security-aware reader is most likely to ask first: *"why this design rather than $X$?"* for each well-established alternative containment strategy that could plausibly be applied to OpenClaw on a personal computer. Each section names the alternative, summarises what it offers, names what it does *not* offer, and states the differential against this work.

The alternatives are not mutually exclusive with this design. Several (notably the OpenClaw native sandbox-mode and the proxy-side allowlist) are *layered* into this perimeter rather than positioned as competitors. The discussion is "why not $X$ *alone*" wherever the alternative is being treated as a standalone answer.

The threat model in [`threat-model.md`](threat-model.md) names the attacker categories T1 through T6, plus T7 (proposed). Each comparison cites which categories the alternative addresses and which it does not.

---

## 1. OpenClaw's own `sandbox.mode: "docker"`

**What it offers.** A configuration value that asks the agent runtime to launch each agent invocation inside a fresh Docker container. The container has the agent's tools, a writable workspace, and outbound network access. Configuration is a single line in `openclaw.json`; no separate orchestration layer.

**What it does not offer.**

- **No credential isolation (T1).** The API key is read from the same environment that the runtime starts the container with. The container inherits it.
- **No supply-chain layer (T2).** Skills are downloaded and loaded inside the same container the agent runs in. A malicious skill executes inside the agent's runtime by design.
- **Single layer (no defense-in-depth).** A misconfiguration of `tools.deny`, a regression in `sandbox.mode`'s container-spec, or a vulnerability in the runtime itself exposes the entire surface.
- **Lifecycle is the user's responsibility.** The container may be left running after the OpenClaw CLI exits; cleanup is the user's vigilance.

**Differential against this work.** OpenTrApp uses `sandbox.mode` (the runtime's native containerisation) as **layer 1 of 6** for T1. Layers 2 to 6 (proxy allowlist, tool policy, exec controls, workspace restriction, kill switch) are this perimeter's contribution. T2's six layers (scanner, line verifier, CDR, allowlist, network isolation, container hardening) and the proxy-side credential injection are entirely absent from the standalone-`sandbox.mode` approach.

**Reference.** [OpenClaw documentation, `sandbox.mode`](https://www.getopenclaw.ai/docs/configuration#sandbox).

---

## 2. Firejail / bubblewrap

**What it offers.** Process-level sandboxing via Linux namespaces and seccomp, without containers. Firejail provides per-application profiles (filesystem restrictions, capability drops, syscall filters) configurable through a small declarative syntax. Bubblewrap is the lower-level primitive; Firejail wraps it with usability.

**What it does not offer.**

- **Network policy is per-process, not per-egress.** Firejail can block all network access or allow all network access, but does not naturally express "this process can reach `api.anthropic.com` on port 443 with this credential and nothing else."
- **No supply-chain pipeline.** Firejail isolates a process; it does not gate skill downloads.
- **No credential isolation.** The credential is in the user's environment when Firejail starts the agent; the sandbox does not separate the agent from its credentials.
- **macOS / Windows portability.** Firejail is Linux-only. Bubblewrap is Linux-only. The architecture would not work on the user base's full target platform set.
- **Per-application profile maintenance.** Each new agent tool is a configuration burden; a tool that needs file access must be granted file access generally.

**Differential against this work.** The container approach gains cross-platform portability (Docker Desktop / Podman Desktop on macOS and Windows, plus native Linux), gains structured network policy (per-container egress filtered through a single proxy), and gains the architectural slot for a separate supply-chain container (the Skill Firewall, `vault-skills`, informally "forge"). Firejail addresses T1 partially (process-level isolation analogous to vault-agent's hardening) but does not address T2, addresses T3 only via "block all egress", and does not address the credential-isolation thread of T1 at all.

**Reference.** [firejail.wordpress.com](https://firejail.wordpress.com/); [containers/bubblewrap](https://github.com/containers/bubblewrap).

---

## 3. gVisor

**What it offers.** A Google-developed user-space kernel that intercepts syscalls from the contained workload and re-implements a (deliberately narrow) subset in user-space Go. The result is a stronger isolation boundary than standard containers: a kernel exploit in the contained workload finds itself talking to gVisor's kernel rather than the host kernel.

**What it does not offer.**

- **Performance overhead.** I/O-heavy workloads see significant performance reduction. Acceptable for many server workloads; less acceptable for an interactive desktop application that the user would notice.
- **Supply-chain pipeline.** Same as Firejail: gVisor isolates execution; it does not gate downloads.
- **Credential isolation.** Same as Firejail.
- **Cross-platform portability.** Linux-only; the macOS / Windows path requires running gVisor inside a Linux VM, at which point the user is paying VM overhead to run a user-space kernel.
- **Operational simplicity.** gVisor as a runtime requires either Docker's `runsc` runtime or a Kubernetes integration; for a desktop application installed by a non-developer user, the operational surface is larger than ordinary containers.

**Differential against this work.** gVisor is the **next isolation tier** the project would consider for users with stronger requirements. The current architecture's container hardening (read-only root, dropped capabilities, seccomp, narrow network policy) approximates gVisor's protections at a fraction of the operational cost. Users who require VM-equivalent isolation are currently directed to a disposable virtual machine ([`whitepaper.md`](whitepaper.md) §9); a future "VM-isolation tier" is queued for the agent workload (`vault-agent`) in [`roadmap.md`](roadmap.md).

**Reference.** [gvisor.dev](https://gvisor.dev).

---

## 4. Native macOS / Windows app sandboxes

**What it offers.** Platform-level sandboxing: macOS App Sandbox (entitlements via the `Sandbox.entitlements` plist), Windows AppContainer / MIC, iOS-style permission gating. Designed for consumer applications; opt-in entitlement system; well-integrated with the OS UI.

**What it does not offer.**

- **Doesn't compose with the agent runtime.** The OpenClaw binary expects to spawn child processes (tools), read and write files, and make network calls. macOS App Sandbox restricts these in ways that conflict with the runtime's expected behaviour without per-tool entitlement work that is not feasible for an open-source project to maintain.
- **Container abstraction is rejected by the platform.** macOS prefers signed code with declared entitlements; an architecture that runs the agent inside a Linux container that runs inside Docker Desktop's hypervisor is necessarily *outside* the App Sandbox model.
- **Windows AppContainer is even more constrained.** Same architectural mismatch.

**Differential against this work.** A Tauri 2 desktop application running outside the App Sandbox / AppContainer layer is the architectural choice. The trade-off is documented: the application asks the user's host OS for the privileges it needs (Docker / Podman runnable, network access, filesystem access in the user's home directory), and the perimeter's defense-in-depth runs at the container layer rather than at the platform-sandbox layer.

**Reference.** [Apple: App Sandbox](https://developer.apple.com/documentation/security/app_sandbox); [Microsoft: AppContainer](https://learn.microsoft.com/en-us/windows/win32/secauthz/appcontainer-isolation).

---

## 5. VM-only isolation (the "disposable cloud VM" recommendation)

**What it offers.** Run the agent on a fresh, disposable virtual machine: a cloud-vendor-provided VM, a local VM via VirtualBox or qemu, or a lightweight microVM (Firecracker, Cloud Hypervisor). On compromise: terminate the VM. Hardware-backed isolation; one of the strongest practical containment boundaries.

**What it does not offer.**

- **Friction.** A user who has to provision a fresh VM, install OpenClaw inside it, configure SSH or similar to interact with it, and remember to destroy it after use will not consistently do so. The architecture is self-defeating: the user who is willing to do this work probably did not need help being careful about agent containment in the first place.
- **No supply-chain pipeline.** A VM contains the agent's runtime; it does not scan skills before they enter the runtime. Skills are downloaded and loaded inside the VM, exactly as in the standalone `sandbox.mode` case.
- **No credential isolation between agent and credential.** The credential is in the VM's environment; whatever runs inside the VM can read it.
- **Cost.** A continuously-running cloud VM has an ongoing dollar cost, even when idle. A disposable-per-use pattern works only for users with strong configuration discipline.

**Differential against this work.** This perimeter aims for a **consumer-installable** product. A cloud-VM recommendation is correct for users with the operational discipline and budget to do it; for the larger population of users who would benefit from running OpenClaw with stronger containment than `sandbox.mode` alone but will not provision per-session VMs, the five-container perimeter is the practical answer. The two approaches are complementary: a user with high security requirements would naturally combine them (run this perimeter inside a disposable VM).

**Reference.** [Firecracker](https://firecracker-microvm.github.io/); [Cloud Hypervisor](https://www.cloudhypervisor.org/).

---

## 6. Static skill scanners only (no CDR)

**What it offers.** Scanners like Sonatype Nexus, Snyk, Checkmarx, OSV-Scanner, and `npm-audit` are well-established for the npm and pip ecosystems. Apply the analogous approach to skills: scan every incoming skill against a catalogue of known-bad patterns; reject on hit.

**What it does not offer.**

- **Pattern N+1 problem.** A scanner with $N$ patterns detects $N$ attacks. Pattern $N{+}1$ (a novel obfuscation, a new persistence mechanism, a category nobody has yet enumerated) slips through. The 87-pattern catalogue in `vault-skills` is the architectural floor, not the ceiling.
- **Obfuscation tolerance.** Layered base-64, zero-width Unicode, HTML-comment-encoded payloads, and similar tricks defeat pure-text pattern matching. A line-level zero-trust classifier (also in `vault-skills`) addresses some of this; both layers stack against obfuscation but neither is sufficient.
- **Same-shape-as-safe attacks.** A skill whose surface looks like a known-safe template but whose semantic effect is malicious passes a scanner that asks *"is the *artefact* safe?"*.

**Differential against this work.** The Skill Firewall pipeline (`vault-skills`) applies all three layers: static scanner ($N$ = 87), zero-trust line classifier, *and* Content Disarm and Reconstruction. CDR ([`adr/0003-content-disarm-reconstruction.md`](adr/0003-content-disarm-reconstruction.md)) is the architectural innovation: rather than asking "is this artefact safe?" (an answer only as good as the catalogue of known badness), it asks "can the artefact's intent be re-expressed in a known-safe form?", and if yes, the original artefact is discarded. CDR catches a strict superset of what the scanner catches and addresses categories the scanner cannot enumerate.

**Reference.** [Sonatype Nexus](https://www.sonatype.com/products/nexus-repository); [Snyk](https://snyk.io/); [Checkmarx](https://checkmarx.com/); [OSV-Scanner](https://google.github.io/osv-scanner/).

---

## 7. Allowlist proxy only (no container hardening)

**What it offers.** Run the agent in its default configuration (no container, no `sandbox.mode`, no separate forge), but route all outbound HTTP through a local allowlist proxy that filters destinations and injects credentials.

**What it does not offer.**

- **No isolation of the runtime itself.** A successful prompt injection or malicious skill executes with the user's own privileges. File system access is the user's full filesystem; process execution is at the user's authority; the proxy controls only outbound HTTP.
- **No supply-chain pipeline.** Skills are loaded into the unhardened runtime; an early-stage skill payload that performs damage on the local filesystem is not gated by the proxy at all.
- **Proxy-side credential injection still works in this approach**, but its value is reduced because the runtime itself is fully exposed.

**Differential against this work.** The proxy is one of six layers for T1, not the whole defense. A proxy-only approach addresses egress (T1's "fetch from attacker URL", T2's "second-stage payload") but does not address local-filesystem attacks, local-command-execution attacks, or credential exposure on the local machine.

**Reference.** Several blog-posted hardening recipes recommend this approach as an entry point; the layered perimeter in this work goes further.

---

## 8. Disable tools at the OpenClaw config level (no perimeter)

**What it offers.** The simplest possible answer: configure `tools.deny` to a restrictive set, set `proxy.allowlist`, store the API key only in `.env`. No containers, no separate forge, no Tauri application.

**What it does not offer.**

- **Self-modifying.** The agent has tools that can edit its own configuration. A successful prompt injection or skill exploit can rewrite `tools.deny` and immediately enable what was previously disabled. Configuration as a security boundary is brittle when the contained workload can edit the configuration.
- **Single layer.** Misconfiguration anywhere (a typo in `tools.deny`, a forgotten entry in `proxy.allowlist`, a wildcard expansion that does not match expectations) exposes the surface beneath it.
- **Credential-adjacent.** Storing the credential in `.env` next to the runtime puts it within reach of any process compromise inside the runtime.
- **No lifecycle ownership.** The runtime starts and stops at the user's discretion; "is the agent currently exposed?" is a question of user vigilance rather than a structural property.

**Differential against this work.** This is the baseline the architecture is engineered against. The whitepaper's introductory argument ([`whitepaper.md`](whitepaper.md) §1) names *single-layer*, *self-modifying*, and *credential-adjacent* as the three structural reasons configuration-only hardening is insufficient. The five-container perimeter addresses all three.

---

## 9. Capability-based OS sandboxes (Capsicum, KeyKOS, EROS)

**What it offers.** Operating-systems research (KeyKOS in the 1980s, EROS / Coyotos in the 2000s, FreeBSD's Capsicum from 2010) explored capability-based security at the syscall layer: a process can perform an operation only if it holds a capability for it; capabilities cannot be forged; the kernel enforces the model.

**What it does not offer.**

- **Mainstream-OS availability.** Capsicum is FreeBSD-only; KeyKOS and EROS are research systems with no production presence. Linux's seccomp is a partial analogue but operates at the syscall-filter level rather than the capability level.
- **Per-application maintenance.** A capability-based agent runtime would require re-engineering OpenClaw against a capability-passing API. This is a research project, not a containment strategy.

**Differential against this work.** The shell-level model in this project ([`adr/0002-adaptive-shell-levels.md`](adr/0002-adaptive-shell-levels.md)) is a coarse-grained capability bracketing applied at the AI-agent-tool layer rather than the OS-syscall layer. It is not novel in *concept* but is novel in *application context*: rather than asking the OS to enforce capabilities, the architecture asks the agent's tool-control layer to do so, with container-level hardening as the enforcement floor.

**Reference.** [Capsicum (FreeBSD)](https://www.freebsd.org/cgi/man.cgi?query=capsicum); [KeyKOS](https://en.wikipedia.org/wiki/KeyKOS); [EROS](https://www.eros-os.org/).

---

## Summary table

| Alternative | T1 (runtime) | T2 (supply chain) | T3 (network MITM) | Credential isolation | Cross-platform |
|-------------|:-:|:-:|:-:|:-:|:-:|
| OpenClaw `sandbox.mode` (Docker) alone | Partial (1 layer) | None | None | No | Yes |
| Firejail / bubblewrap | Partial | None | Block-all only | No | Linux only |
| gVisor | Strong (kernel-level) | None | Block-all only | No | Linux only |
| macOS / Windows app sandbox | Strong (where it composes) | None | Platform-level | No | Per platform only |
| Disposable VM | Very strong (hardware) | None | None | No | Yes |
| Allowlist proxy only | None | None | Strong | Optional | Yes |
| Tools-deny config only | None | None | Allowlist only | No | Yes |
| Capsicum / capability OS | Strong (research) | None | None | No | FreeBSD only |
| **This perimeter (OpenTrApp)** | **Strong (6 layers)** | **Strong (6 layers + CDR)** | **Allowlist + logging** | **Yes (proxy-side)** | **Yes** |

The "Strong" / "Partial" / "None" classification follows the [`threat-model.md`](threat-model.md) attacker-capability matrix: "Strong" means the attacker category has multiple independent mitigating layers each backed by an evidence cell; "Partial" means a subset of capabilities are addressed; "None" means the alternative does not address the category. The "Cross-platform" column is for the user-installable target platform set (Linux, macOS, Windows on x86-64 / Apple Silicon); installers for all three are published (the README lists `.deb` / `.rpm` / `.AppImage` for Linux, `.dmg` for macOS, and `.msi` / `.exe` for Windows). Linux is the primary tested target; the macOS and Windows installers are not OS-level code-signed (signing is updater-only, per [`threat-model.md`](threat-model.md) T4).

The architectural choice that most distinguishes this work (and that the alternatives above do not provide) is the **composition**. Container hardening, allowlist proxy, supply-chain pipeline, and credential isolation each exist independently in the literature; their integration into a coherent perimeter that an end user can install with a setup wizard, control from a Telegram bot, and reason about with a single clearly-bounded surface (`compose.yml` plus three component manifests) is the contribution.

---

## Cross-references

- [`whitepaper.md`](whitepaper.md) §10 (related work, narrative form): the conversational counterpart to this comparison matrix.
- [`threat-model.md`](threat-model.md): the attacker model that the "Strong / Partial / None" classifications above are evaluated against.
- [`README.md`](../README.md) "Limitations": cites this document for the differential against alternative containment strategies.
- [`trifecta.md`](trifecta.md) §7: the layer-by-layer enumeration of the perimeter design that the alternatives above are compared against.
- [`adr/0001`](adr/0001-proxy-side-api-key-injection.md), [`adr/0002`](adr/0002-adaptive-shell-levels.md), [`adr/0003`](adr/0003-content-disarm-reconstruction.md): the three architectural decisions most cited in the comparisons.
