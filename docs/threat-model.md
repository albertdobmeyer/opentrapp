# Threat model

**Document status:** Active
**Created:** 2026-05-04
**Companion documents:** [`whitepaper.md`](whitepaper.md) §2 (the conversational summary); [`trifecta.md`](trifecta.md) §7 (the defense-in-depth tables); [`SECURITY.md`](../SECURITY.md) (vulnerability-reporting policy); [`adr/`](adr/) (architectural decisions cited per row).

This document specifies the threats the OpenTrApp perimeter is designed to address, the perimeter layers that mitigate each, the residual risk that remains after the mitigations, and the empirical evidence (wherever it exists) that the mitigations work as documented. It is the single source of truth on the question *"what does this perimeter actually protect against, and what does it not?"*.

The model is structured around six accepted attacker categories (T1 through T6), plus two **Proposed** categories introduced by the target architecture: **T7** — the prompt-injected host operator introduced by the agent-operable control plane ([ADR-0021](adr/0021-danger-gated-agentic-control-plane.md)), folded in fully on that ADR's acceptance — and **T8** — the loopback-viewer attacker introduced by the de-Tauri browser-served control surface ([ADR-0022](adr/0022-daemon-control-surface.md)), folded in on the spike's threat-model acceptance. Within each category, capabilities are decomposed by STRIDE class (Spoofing, Tampering, Repudiation, Information disclosure, Denial of service, Elevation of privilege) to make sure the analysis is complete rather than narrative-driven. Categories T1 and T2 are the principal threats the architecture is engineered against. T3 is addressed structurally. T4 is explicitly out of scope. T5 is addressed by user-experience measures rather than mechanical isolation. T6 is partially in scope. T7 is addressed structurally (the danger-gate), with an honest T4-inherited residual. T8 is a *decision* threat model — it must show the loopback surface is a net improvement over the in-process WebKit it replaces, and is allowed to conclude "keep Tauri."

---

## Conventions

Each row in the matrices below has six fields:

| Field | Meaning |
|-------|---------|
| **Capability** | A specific attacker action, named at a granularity that maps cleanly onto a defense layer. Multiple capabilities per attacker are expected. |
| **STRIDE** | The STRIDE class the capability falls under. A single capability may span more than one class; the dominant class is named first. |
| **Mitigating layer** | The perimeter layer (or layers) that reduce the impact of the capability. Layer names match those used in [`trifecta.md`](trifecta.md) §7 and the ADRs. |
| **Residual risk** | What the mitigation does *not* address. Rows that say "none" are honest; rows that say "see *Residual risks* below" point to the per-category notes after the matrix. |
| **Evidence** | A citation to a verification step (test, script, ADR) that supports the claim. Rows marked `untested` are honest about the absence of empirical support and are tracked as a future-work item in the [post-launch roadmap](roadmap-post-launch.md). |
| **Reference** | The ADR or architecture-section that contains the full rationale. |

The "STRIDE" classes are interpreted as follows in this document:

- **S (Spoofing):** the attacker successfully claims an identity that is not theirs (e.g. claims to be the user, claims to be the upstream API, claims to be the registry).
- **T (Tampering):** the attacker successfully modifies data or code that should be authoritative (e.g. modifies a skill, modifies the manifest, modifies the proxy log).
- **R (Repudiation):** the attacker successfully denies having performed an action, in a context where attribution would matter.
- **I (Information disclosure):** the attacker reads data they should not be able to read (the API key, the user's filesystem outside the workspace, the user's secrets).
- **D (Denial of service):** the attacker prevents legitimate use of the system.
- **E (Elevation of privilege):** the attacker gains the ability to perform actions beyond their starting capability set.

---

## T1: Prompt-injection author

**Definition.** An attacker who controls content that the agent will eventually see and process: a Telegram message body, an HTTP response from a fetched URL, a piece of feed content, the body of a loaded skill. The attacker's goal is to cause the agent to execute a tool, exfiltrate data, or modify state on the attacker's behalf.

**Why this is a principal threat.** Simon Willison's "lethal trifecta" framework (private data + untrusted content + exfiltration capability) shows that prompt injection becomes catastrophic precisely when an agent has tools that touch all three. The OpenClaw default configuration grants exactly that combination. The architectural goal is to prevent the trifecta from forming at the *infrastructure* layer.

| Capability | STRIDE | Mitigating layer | Residual risk | Evidence | Reference |
|------------|--------|------------------|---------------|----------|-----------|
| Cause the agent to execute an arbitrary shell command | E, T | Tool policy (denied tools never enter the LLM's tool catalogue); exec controls (safelisted binaries plus per-action approval) | A tool that *is* on the safelist can still be misused inside its allowed surface | `cargo test --lib` for the runner's argument-escaping tests; manual verification via the Karen-persona dogfood | [`trifecta.md`](trifecta.md) §7.1 layers 3, 4 |
| Cause the agent to read a file outside the workspace | I | Workspace restriction (`workspaceOnly: true`); container hardening (no host filesystem mounts) | A path-traversal vulnerability in the workspace handler would defeat layer 1 only; layer 2 still holds | Verification check 9 ("workspace mount only") in [`workloads/agent/scripts/verify.sh`](../workloads/agent/scripts/verify.sh) | [`trifecta.md`](trifecta.md) §7.1 layers 1, 5 |
| Cause the agent to fetch from an attacker-controlled URL | S, I | Domain allowlist (only known good upstream hosts); IP-literal denial inside the allowlist matcher (raw IPs as host headers are rejected); proxy logging (every request recorded) | Allowlisted destinations can themselves be turned into attacker-controlled paths via DNS rebinding or compromised upstream hosts (see T3 residual risks below) | Live allowlist policy in [`infra/proxy/allowlist.txt`](../infra/proxy/allowlist.txt); proxy implementation in [`vault-proxy.py`](../infra/proxy/vault-proxy.py); IP-literal denial pinned by [`infra/proxy/test_vault_proxy.py`](../infra/proxy/test_vault_proxy.py) | [`adr/0001-proxy-side-api-key-injection.md`](adr/0001-proxy-side-api-key-injection.md); [`trifecta.md`](trifecta.md) §4.4 |
| Cause the agent to exfiltrate the API credential | I | Proxy-side credential injection (the literal credential is never present inside the agent's container) | Compromise of `vault-proxy` itself exposes the credential | Verification check 7 ("API keys absent from environment") in `verify.sh` | [`adr/0001-proxy-side-api-key-injection.md`](adr/0001-proxy-side-api-key-injection.md) |
| Cause the agent to install a malicious skill discovered through a fetched URL | E | Adaptive shell levels (skill loading is gated by the current shell level); supply-chain pipeline (every skill goes through the scanner regardless of source) | Skills installed under Soft Shell are not subject to additional friction; the user explicitly opted into the broader surface | `cargo test --lib` for the shell-level transition tests | [`adr/0002-adaptive-shell-levels.md`](adr/0002-adaptive-shell-levels.md); [`adr/0003-content-disarm-reconstruction.md`](adr/0003-content-disarm-reconstruction.md) |
| Cause the agent to widen its own egress allowlist (self-loosen the perimeter) | E, T | Host-mediated allowlist loosening (v0.6 Item A): the agent has no path to the app's control plane (no network service); the **only** writer of the allowlist is the orchestrator, and only on an explicit human tap. Off-allowlist requests are surfaced to the user as *recommendations*, never auto-applied; the allowlist file is bind-mounted read-only into the proxy; clear exfil is hard-blocked at the proxy and never reaches the judge | A user who taps "Allow always" (behind the two-tap confirm) approves a host they should not (this is T5, the human's choice); a mis-trained judge can mis-*recommend* but is structurally unable to *apply* a loosening | `cargo test --lib` allowlist-invariant tests (`apply_always` is the sole writer; `record_denial` never writes the allowlist; clear exfil never surfaces); `orchestrator-check.sh` §27 | [`adr/0016-host-mediated-allowlist-loosening.md`](adr/0016-host-mediated-allowlist-loosening.md); [`adr/0002-adaptive-shell-levels.md`](adr/0002-adaptive-shell-levels.md) |
| Cause the agent to modify its own configuration to enable a denied tool | T, E | Configuration-integrity hash (snapshot at startup, re-verified on every check); read-only root filesystem | A configuration-modifying compromise that survives to the next health-check cycle is detectable but not preventable in real time | Verification check 24 ("configuration-integrity hash matches startup snapshot") in `verify.sh` | [`trifecta.md`](trifecta.md) §7.1 layer 1 |
| Cause the agent to silently send a Telegram message to an unapproved counterpart | S, I | DM pairing policy (each Telegram counterpart is explicitly approved by the user); allowlist on outgoing recipients | A counterpart who was previously approved but whose account is later compromised is not detected | Manual verification during pairing flow; covered in dogfood walkthrough | [`trifecta.md`](trifecta.md) §7.3 layer 3 |
| Cause the agent to consume the user's API budget by repeated upstream requests | D | Hard spending cap on the API key (configured outside the perimeter); rate-limiting in `vault-proxy` | An attacker who burns the budget within the cap denies the user further use of the agent until the next billing cycle | `untested`: rate-limiting threshold is not load-tested | Hard spending cap is the user's responsibility per `SECURITY.md` |
| Cause the agent to write damaging output that is then signed-into-record by a downstream system | T, R | Per-action human approval for safelisted commands; proxy-side request logging | An action approved by the user under a misleading framing remains an honest record of what the user approved | `untested`: depends on the user's vigilance, not a mechanical layer | [`trifecta.md`](trifecta.md) §7.1 layer 4 |

**Residual risks specific to T1.**

- **The agent's reasoning itself can be subverted.** The architecture treats this as the design assumption: the agent's reasoning will eventually be turned by a prompt injection. The perimeter is engineered for that case rather than for preventing it.
- **A live session at Soft Shell is significantly more powerful.** The user opting into Soft Shell extends the agent's capability surface to autonomous web browsing and an extended command safelist. Prompt injection at Soft Shell is correspondingly more impactful. The shell-level model in [`adr/0002`](adr/0002-adaptive-shell-levels.md) specifies that promotion is always user-initiated; the residual risk after that is the user's choice.
- **The Telegram channel is the principal injection vector for end-user-facing instances.** Recommendation: pair the agent with a *dedicated* Telegram account, not the user's personal account.

---

## T2: Malicious skill author

**Definition.** An attacker who publishes a skill on the ClawHub registry designed to look useful and carry a payload. The attacker's goal is code execution inside the agent's runtime when the user installs the skill.

**Empirical baseline.** The ClawHavoc study (2026-Q1) classified 341 of 2,857 published ClawHub skills (11.9 %) as malicious. The architectural assumption is that *every* incoming skill is potentially hostile.

| Capability | STRIDE | Mitigating layer | Residual risk | Evidence | Reference |
|------------|--------|------------------|---------------|----------|-----------|
| Have a malicious skill download a second-stage payload from an attacker-controlled URL | E, I | Static scanner (87 patterns include `c2_download`, `exec_download` categories with MITRE-mapped IDs); proxy allowlist (the agent cannot reach attacker domains anyway) | A novel obfuscation that defeats the scanner is not detected at scan time; the proxy still prevents fetch | `wc -l workloads/skills/tools/lib/patterns.sh` for catalogue size; [`workloads/skills/tests/`](../workloads/skills/tests/) for per-pattern unit tests | [`adr/0003-content-disarm-reconstruction.md`](adr/0003-content-disarm-reconstruction.md) layer 1 |
| Hide a payload inside formatting tricks (zero-width Unicode, HTML comments, layered base-64) so a literal text scan misses it | T, E | Content Disarm and Reconstruction (CDR): the original artefact is discarded; a clean version is rebuilt from parsed semantic intent | A payload that survives intent-parsing (i.e. the attack pattern is the intent) is not stripped by CDR; the static scanner and line classifier are the layers that catch this | Pipeline implemented at [`workloads/skills/tools/skill-cdr.sh`](../workloads/skills/tools/skill-cdr.sh); end-to-end test in [`workloads/skills/tests/cdr-pipeline.test.sh`](../workloads/skills/tests/cdr-pipeline.test.sh) | [`adr/0003-content-disarm-reconstruction.md`](adr/0003-content-disarm-reconstruction.md) |
| Use a previously-unknown line shape to evade the line classifier | E, T | Zero-trust line classifier (every line classified; failure quarantines the entire skill), combined with the static scanner ahead of it | A sufficiently sophisticated obfuscation that mimics a known-safe line shape is not caught here; CDR (next layer) catches it if the *intent* is detectable | Line classifier at [`workloads/skills/tools/lib/line-classifier.sh`](../workloads/skills/tools/lib/line-classifier.sh); pattern-coverage test at [`workloads/skills/tests/test-patterns.test.sh`](../workloads/skills/tests/test-patterns.test.sh) | [`trifecta.md`](trifecta.md) §7.2 layer 2 |
| Influence the scanner itself through a skill submission (poisoning) | T | Scanner is in-image and read-only; pattern catalogue is checked into source under code review | An attacker who lands a malicious-pattern PR through the project's review process is not addressed mechanically | `untested`: code-review process is the layer | [`CONTRIBUTING.md`](../CONTRIBUTING.md) on security-relevant pull-request review |
| Bypass forge by side-loading a skill into the workspace | T, E | SHA-256 clearance report on every certified skill; the agent rejects skills whose hash does not match a signed report | A user manually enabling a side-loaded skill (with explicit confirmation) trusts that skill (this is a feature, not a bypass) | Hash-verification enforced by [`workloads/agent/scripts/install-skill.sh`](../workloads/agent/scripts/install-skill.sh); secondary verification via [`verify-skills.sh`](../workloads/agent/scripts/verify-skills.sh) | [`adr/0003-content-disarm-reconstruction.md`](adr/0003-content-disarm-reconstruction.md) "rebuilt artefact is signed" |
| Talk to forge directly through the agent to influence the scan result | E, T | Network isolation: forge has no routed path to the agent; the only delivery channel is the write-only shared volume | None at the network layer; the volume is unidirectional by design | Verification check in `tests/orchestrator-check.sh` for compose network policy | [`trifecta.md`](trifecta.md) §3 (network isolation matrix) |
| Cause forge itself to perform an attacker-directed action by feeding it a crafted skill bundle | E | Forge runs at a hardened isolation level (read-only root, dropped capabilities, narrow seccomp profile); CDR pipeline runs offline (no network during scan/parse/rebuild) | A vulnerability in the parser or generator that produces RCE inside forge is contained by forge's hardening but is a real risk | `untested`: fuzzing of the parser is queued | [`workloads/skills/Containerfile`](../workloads/skills/Containerfile) |
| Persist after a single failed scan by re-uploading variants | D | Quarantine logs; user is informed that a skill has been rejected with a category code | The user may give up rather than try again; persistence as an attack is mitigated by the user's vigilance | `untested`: friction effect on user is not measured | [`trifecta.md`](trifecta.md) §7.2 |

**Residual risks specific to T2.**

- **The scanner has a structural limit.** Pattern N+1 (a novel obfuscation, a new persistence mechanism, a category nobody has yet enumerated) slips through. CDR is the architectural answer to this; CDR is itself bounded by the parser's expressiveness.
- **CDR cannot remove an attack whose intent *is* the attack.** A skill whose stated purpose is "delete user files in path X" passes intent-parsing because the intent is what the skill literally does. The scanner and line classifier catch the cases CDR cannot.
- **The user's ClawHub-installation friction matters.** The most reliable defense against a malicious skill is not installing it. The current install flow shows the scanner verdict and asks for explicit confirmation; a less-engaged user clicking through is a residual risk.

---

## T3: Network man-in-the-middle (MITM)

**Definition.** An attacker positioned between `vault-proxy` and the public internet. Examples: a compromised local network, a hostile DNS resolver, a captive portal that injects content, a state-level adversary at the ISP layer.

**Why this is a structural concern.** The agent's reasoning is performed remotely (Anthropic's API) and the user authenticates via Telegram (Cloudflare-fronted). Both endpoints are TLS-protected, but a MITM with control of a trusted CA could in principle intercept.

| Capability | STRIDE | Mitigating layer | Residual risk | Evidence | Reference |
|------------|--------|------------------|---------------|----------|-----------|
| Read the agent's outbound API requests | I | TLS termination of all outbound calls inside `vault-proxy`; Anthropic's standard TLS to upstream | An attacker with a compromised CA in the host trust store could MITM upstream of the proxy; at that point the host is compromised (T4) | `untested`: TLS termination is functional, certificate-pinning is not currently enforced upstream | [`adr/0001-proxy-side-api-key-injection.md`](adr/0001-proxy-side-api-key-injection.md) |
| Modify the agent's outbound API request body | T | Same as above | Same as above | `untested`: same | Same |
| Inject a forged response that the agent treats as real | S, T | Same as above; the agent has no privileged actions on response *content alone* (every shell-level execution requires explicit safelist or approval) | A response forged to *look* like a benign tool result, then routed through approved tools, is in scope of T1, not T3 | Manual reasoning; covered structurally by T1 mitigations | [`whitepaper.md`](whitepaper.md) §2 T3 |
| Determine *which* upstream the agent is talking to (traffic analysis) | I | None: egress points are inherent to the architecture (Anthropic, Telegram, ClawHub). The fact that the agent talks to Anthropic is not a secret. | Egress endpoint set is public knowledge; this is not a meaningful threat | n/a | n/a |
| Drop traffic to deny service | D | Standard HTTP retry/back-off in `vault-proxy`; agent gracefully reports the failure | An attacker who can drop traffic can deny service indefinitely; this is structural to all online software | n/a (denial of service is unavoidable at the network layer) | [`whitepaper.md`](whitepaper.md) §9 |

**Residual risks specific to T3.**

- **Certificate pinning is not currently enforced** for upstream Anthropic and Telegram endpoints. This is a planned hardening (queued for a future release). In the interim the architecture relies on the host-OS trust store; a compromised CA in that store defeats this layer.
- **DNS rebinding against allowlisted hosts is now a narrow, kernel-backstopped residual.** The proxy's allowlist matcher operates on the request's `Host` header (a domain string) and rejects raw IP-literal hosts at [`vault-proxy.py:188-196`](../infra/proxy/vault-proxy.py) (regression-pinned by [`test_vault_proxy.py`](../infra/proxy/test_vault_proxy.py); line range approximate). On top of that hostname check, the structural fix has **shipped**: a post-resolve destination-IP filter (`_resolves_to_private`) now runs *after* the allowlist check, so an allowlisted domain whose authoritative DNS server returns a private/loopback address (e.g. `127.0.0.1`, `172.17.0.1`, an RFC1918 range, or AWS metadata `169.254.169.254`) is rejected before the proxy connects; it is unit-tested in [`infra/proxy/test_vault_proxy.py`](../infra/proxy/test_vault_proxy.py). This is layered with a kernel-level RFC1918 egress filter in the dedicated `vault-egress` sidecar ([ADR-0009](adr/0009-five-container-perimeter.md) Tier 4) and a pinned DoT resolver (`unbound` forwarding only to Quad9 and Cloudflare over DNS-over-TLS; [ADR-0010](adr/0010-pinned-resolver-dns.md)). The earlier discussion of mitmproxy's `block_private` flag no longer governs this row: those flags are **source-IP filters** that gate which client IPs may use the proxy, not destination filters, and `block_private=false` remains set in [`compose.yml`](../compose.yml) only because the agent container's source IP is itself private (it lives on an internal podman network); destination filtering is now done explicitly by `_resolves_to_private` plus the `vault-egress` kernel drop rather than relying on mitmproxy. **Honest residual:** a DNS-rebinding TOCTOU between the proxy's resolve and its connect remains theoretically possible (the resolved IP could change in the window between the check and the socket connect), but it is now backstopped at the kernel layer by the nftables RFC1918 drop in `vault-egress`. Full live re-verification of the layered behavior on capable hardware is pending (this workstation swap-storms under the full perimeter), so the end-to-end claim is treated as shipped-but-not-yet-live-re-verified.
- **Egress is observed, not attested.** The proxy logs every request but does not produce signed evidence. A reader of the log must trust the host's clock and the proxy's integrity.

---

## T4: Compromised host

**Definition.** An attacker who already has partial access to the user's machine before the perimeter is installed or while it is running: a malicious binary the user installed, an OS-level rootkit, another foothold.

**Scope statement.** This category is **explicitly out of scope** for the perimeter's protective claims. The architecture assumes the host is honest. If the host is dishonest, no container-level isolation can recover security.

| Capability | STRIDE | Mitigating layer | Residual risk | Evidence | Reference |
|------------|--------|------------------|---------------|----------|-----------|
| Read the API credential from the proxy container's memory | I | None at the host level: the proxy holds the credential in environment variables visible to its own process | Out of scope | n/a | [`SECURITY.md`](../SECURITY.md) "Out of scope" |
| Modify the compose file or override the proxy's allowlist | T, E | Standard host file permissions; the user's account owns the file | Out of scope | n/a | Same |
| Read the workspace contents from the host filesystem | I | Container volumes are owned by the user account that started the perimeter | Out of scope | n/a | Same |
| Replace the desktop application binary with a malicious one | S, T, E | Tauri auto-updater key signature on releases | A user-installed malicious binary on the host (replacing the legitimate one) is out of scope; Gatekeeper / SmartScreen first-launch warnings exist on macOS / Windows | n/a (installer signing is updater-only, not OS-level code-signing) | [`whitepaper.md`](whitepaper.md) §9 ("Installer signing is updater-only") |

**Recommendation.** Users requiring stronger isolation against host compromise are directed to run the perimeter on a disposable virtual machine with a disposable API key and a hard spending cap. This recommendation is documented in [`workloads/agent/README.md`](../workloads/agent/README.md) (*Isolation tiers*) and in the README's *Limitations* section.

---

## T5: Hostile end user

**Definition.** A human (the legitimate user, or someone who has gained access to the user's Telegram account) who instructs the agent to perform a damaging action.

| Capability | STRIDE | Mitigating layer | Residual risk | Evidence | Reference |
|------------|--------|------------------|---------------|----------|-----------|
| Issue a command from Telegram that performs a destructive operation | E | Per-action human approval gate on safelisted commands; the user must affirmatively click "Allow" on each invocation | A user who clicks "Allow" without reading approves the action; this is by design (UI explicit) | Manual verification during dogfood walkthrough | [`trifecta.md`](trifecta.md) §7.1 layer 4 |
| Approve an unsafe site by tapping "Allow always" without reading (approval fatigue) | E | Only *gray-zone* off-allowlist requests surface (clear exfil and rebinding blocks never do, so the list stays short); each carries a plain-language reason from the on-device judge; "Allow always" requires a deliberate **two-tap confirm** (friction, not a barrier) | A user who taps through both confirms approves the host; this is by design (the human is the authority and the only one who can loosen) | `EgressApprovalsCard.test.tsx` (the two-tap confirm path); manual dogfood | [`adr/0016-host-mediated-allowlist-loosening.md`](adr/0016-host-mediated-allowlist-loosening.md) |
| Promote the agent's shell level to Soft Shell maliciously | E | Promotion is initiated from Tier 1 (the user themselves or the trusted CLI coordinator); the agent cannot promote itself | Same as above: the user is the authority | Live policy in [`workloads/agent/scripts/tool-control.sh`](../workloads/agent/scripts/tool-control.sh) | [`adr/0002-adaptive-shell-levels.md`](adr/0002-adaptive-shell-levels.md) |
| Use the agent to access the user's own files outside the workspace | I, E | Workspace-only restriction; the agent has no host filesystem access regardless of who is asking | None at this layer | Verification check 9 in `verify.sh` | [`trifecta.md`](trifecta.md) §7.1 layer 5 |
| Use the agent's API credential beyond the user's intent (cost abuse) | D | Hard spending cap on the API key (configured outside the perimeter) | The cap is a budget, not a permission; an attacker who lands the cap wastes the budget | n/a | `SECURITY.md` recommendation |
| Compromise the operator's Telegram account and operate the agent as the user | S | Two-factor authentication on the paired Telegram account; the recommendation that the paired account is dedicated rather than personal | A successful Telegram-account compromise grants the attacker the full user surface; no perimeter layer recovers from that | `untested`: Telegram-side compromise is out of the perimeter's reach | [`whitepaper.md`](whitepaper.md) §9 ("The Telegram control channel is a trust boundary") |

**Residual risks specific to T5.**

- **The user is part of the security boundary.** Per-action approvals require the user to read what they are approving. A pattern of automatic approval (clicking "Allow" without reading) defeats this layer. Mitigation: the UI delays the approval button briefly on potentially-destructive operations to encourage reading; this is a friction layer, not a barrier.
- **A dedicated Telegram account is the recommended pairing.** Using a personal Telegram account for the agent control channel widens T5's blast radius significantly.

---

## T6: Side-channel observer

**Definition.** An entity with read access to artefacts produced by the perimeter outside the five containers: proxy request logs, system metrics (CPU, memory), the host filesystem outside the perimeter, container layer caches, image metadata, the application's own state files.

**Why this matters.** Even when the credential is structurally protected (T1's row 4) and the workspace is isolated (T1's row 2), conversational metadata, request timing, and activity patterns can leak information about *what the user is doing with the agent*.

| Capability | STRIDE | Mitigating layer | Residual risk | Evidence | Reference |
|------------|--------|------------------|---------------|----------|-----------|
| Read the proxy's request log to learn which upstream hosts the user has been talking to | I | Log is host-readable by the user account that runs the perimeter; not available to other host users without privilege escalation | Anyone with read access to the log file (including the user themselves) can derive activity timing and counts | Log location at `vault-proxy/var/log/vault-proxy/requests.jsonl`, mode 0640 | [`adr/0001-proxy-side-api-key-injection.md`](adr/0001-proxy-side-api-key-injection.md) |
| Read the API credential from the proxy's request log | I | The substitution is byte-replace; the literal credential is never logged | None: verified by inspection of `vault-proxy.py` | `grep -E '(API_KEY|x-api-key)' vault-proxy/var/log/vault-proxy/requests.jsonl` returns nothing |  Same |
| Read the container layer caches or image metadata after `compose down` | I | Layer caches and image metadata persist on the host; do not contain the credential (proxy-side injection guarantees that) | Conversation logs and activity metadata may persist; full cleanup requires `podman system prune -a` | `untested`: what exactly persists on each platform is not enumerated | [`whitepaper.md`](whitepaper.md) §9 ("Container destruction does not guarantee complete cleanup") |
| Use timing of upstream API requests to infer when the user is active | I | None: request timing is inherent to the architecture | The user's activity pattern is observable to anyone with log read access | n/a | n/a |
| Observe the host's Telegram `getUpdates` long-poll while the perimeter is **dormant** (idle auto-pause) to infer that the user is away and when their next message arrives | I | The waker reuses the existing wizard host→Telegram channel (same endpoint, token, outbound direction); it is **peek-only**: it never advances the offset and never reads message content, so it emits no more than the agent's normal polling already does, and only an `update_id`-present signal | A network observer of host traffic, or Telegram itself, can already see the bot's polling pattern; dormancy shifts *which process* polls (host vs. agent) but exposes no new content. Off by default until enabled; outbound-only, no new listener | Pinned by ADR-0018 construction (no edge to `telegram_advance_offset`; resume does not advance the offset) | [`adr/0018-idle-auto-pause-host-waker.md`](adr/0018-idle-auto-pause-host-waker.md) |
| Read the user's workspace files from outside the agent | I | Container-volume permissions; workspace is owned by the user account that runs the perimeter | Anyone with that user's privileges (including the user themselves) can read the workspace; this is by design | n/a | [`trifecta.md`](trifecta.md) §3 |
| Read state files in `~/.opentrapp/` (RunGuard PID, paused marker, dormant marker) | I | Files are owned by the user account; mode 0600 / 0640 as appropriate | Same as above; the `dormant` marker additionally reveals that the perimeter is auto-paused, which is the same activity signal as an idle proxy log | Inspection of the `RunGuard` block (search for `runguard_dir`) and the dormant-marker helpers in [`app/src-tauri/src/lifecycle.rs`](../app/src-tauri/src/lifecycle.rs) | [`adr/0018-idle-auto-pause-host-waker.md`](adr/0018-idle-auto-pause-host-waker.md) |
| Read the user's API credential from `.env` on the host | I | `.env` is mode 0600 and gitignored | A backup tool that disregards file mode (some cloud-backup clients) may capture `.env` | `untested`: backup behaviour is per-tool | `SECURITY.md` "Out of scope" for backup-tool misconfiguration |

**Residual risks specific to T6.**

- **Activity metadata is not erased.** A user with confidentiality concerns about *who they have been talking to* should treat the proxy log as sensitive and apply log rotation / shredding accordingly.
- **Image and layer caches persist by default.** Full cleanup is documented in [`whitepaper.md`](whitepaper.md) §9 and requires `podman system prune -a` (or the Docker equivalent).
- **The host filesystem is the user's responsibility.** Files in `~/.opentrapp/`, the workspace volume, and the `.env` file are protected by ordinary host file permissions. A user who shares the host account broadly weakens this layer.
- **Idle auto-pause shifts the poller, not the exposure.** When the perimeter is dormant the host process (not the agent) holds the single Telegram `getUpdates` poll. This is the same bot, token, and outbound polling pattern an observer could already see; the waker is peek-only (never advances the offset, never reads content), so no message content is newly exposed. The behavior is off by default and documented in [`adr/0018-idle-auto-pause-host-waker.md`](adr/0018-idle-auto-pause-host-waker.md).

---

## T7: Prompt-injected host operator *(Proposed; defined in [ADR-0021](adr/0021-danger-gated-agentic-control-plane.md); folded in here on acceptance)*

**Definition.** A *trusted, user-installed* host agent (Claude Code, opencode) that is **prompt-injected** via content it reads and that has, or can reach, an OpenTrApp control surface (the agent-operable control plane of [ADR-0020](adr/0020-product-identity-and-distribution.md)). The attacker controls the agent's *instructions*, not the host OS. Goal: weaken the perimeter protecting the *contained* agent (the T1 attack aimed one level up, at the *external* operator).

**Scope.** T7 is **distinct from T4**: the host program is honest, only the content it reads is hostile. A *fully* injected, fully-privileged host agent can tamper with `~/.opentrapp` or kill the daemon directly; that residual **is T4 and stays out of scope**. The in-scope guarantee ADR-0021 defends is narrower and honest: **OpenTrApp's agentic control plane is never an *amplifier*; it adds no new, easier boundary-weakening path than the pre-existing T4 residual.** Boundary-weakening through any OpenTrApp surface (CLI/MCP/loopback/GUI) requires the *same out-of-band human confirmation* whether the caller is a human or an agent (`boundary_impact: weakening` → human tap / phone, never agent-auto-satisfiable; the weakening writers have no agent call edge, per ADR-0016 generalized). Full STRIDE decomposition and the danger-gate rules are in [ADR-0021](adr/0021-danger-gated-agentic-control-plane.md).

---

## T8: Loopback-viewer attacker *(Proposed; defined in [ADR-0022](adr/0022-daemon-control-surface.md); spike-gated; folded in here on the spike's threat-model acceptance)*

**Definition.** The de-Tauri direction ([ADR-0022](adr/0022-daemon-control-surface.md)) replaces the bundled in-process WebKit webview with the user's *own* browser, served by an **on-demand, ephemeral** HTTP/WS control server the daemon binds on `127.0.0.1`. T8 is the attacker who tries to reach or abuse *that server*: a malicious local process, a malicious web page open in the user's browser, a hostile browser extension, another local user, or an off-host network peer. Goal: drive the perimeter-owning daemon's control plane (read state, pause/resume, or — gated by T7/[ADR-0021](adr/0021-danger-gated-agentic-control-plane.md) — attempt a boundary-weakening op) without the user's intent.

**Why this surface is on the table at all.** The motivation is removing the pinned, aging `webkit2gtk`/GTK3 stack (the 19 unmaintained advisories in [`known-advisories.md`](known-advisories.md)) from what users install, *and* making the renderer the user's continuously-patched browser instead. The honest cost is that a *security* product would stand up a local control server reachable by a general-purpose browser — the textbook DNS-rebinding / CSRF / malicious-extension class, with a long prior-art CVE trail (Ollama "Probllama", Jupyter token leaks, Docker Desktop). T8 is therefore a *decision* threat model: it must show the surface is a **net** improvement over the status quo, and it is allowed to conclude "keep Tauri" (C1). The defensive controls and prior-art are catalogued in [`de-tauri-viewer-research.md`](de-tauri-viewer-research.md); this section is the attacker-side decomposition.

| Capability | STRIDE | Mitigating layer | Residual risk | Evidence | Reference |
|------------|--------|------------------|---------------|----------|-----------|
| Reach the server from off-host (LAN/WAN) | S, E | Bind `127.0.0.1` only, ephemeral port; no non-loopback interface accepts TCP; the bound address is unit-asserted to be loopback | None for direct off-host TCP (the browser-mediated rebinding path is the next row, not this one) | `untested-on-capable-hardware`: bound-addr-is-loopback test on the spike branch `spike/de-tauri-loopback` (`crates/viewer-server`); not yet on `main` | [ADR-0022](adr/0022-daemon-control-surface.md) §3 |
| Use a web page the user is visiting to DNS-rebind an attacker domain to `127.0.0.1` and drive the API | S, E | Strict **Host-header allowlist** (`127.0.0.1:PORT` / `localhost:PORT` only) — a rebound request still carries the attacker's `Host` and is rejected; Origin allowlist as a second gate | None while the Host check holds; this row *is* the Ollama-Probllama class, so the control is load-bearing | `untested-on-capable-hardware`: spike `host_allowed` rejects a forged `Host` (403); live-browser re-verification pending | [ADR-0022](adr/0022-daemon-control-surface.md) §3; [research](de-tauri-viewer-research.md) §"Host-header" |
| Use any web page to issue a no-credentials cross-origin request (CSRF) that acts on the daemon | S, T | **Bearer-in-`Authorization`** (the API uses no ambient cookie auth → structurally CSRF-immune); Origin allowlist incl. the WS handshake; `Sec-Fetch-Site` check | None structural — an attacker page cannot attach the bearer | `untested-on-capable-hardware`: spike asserts tokenless → 401, wrong-Origin → 403 | [ADR-0022](adr/0022-daemon-control-surface.md) §3 |
| Read the session token from the URL, browser history, or `Referer` | I | Token **never in the URL**: a single-use, short-TTL launch nonce rides the URL **fragment** (`#…`, not sent to any server, absent from `Referer`), exchanged once at `/api/session` for an `HttpOnly; SameSite=Strict` cookie / `sessionStorage` bearer; the long-lived token never transits a URL | A nonce intercepted in the brief pre-exchange window (single-use + short TTL bound it) | `untested-on-capable-hardware`: spike asserts token absent from URL/history/`Referer` | [ADR-0022](adr/0022-daemon-control-surface.md) §3; [research](de-tauri-viewer-research.md) §"fragment→cookie" (Jupyter CVE-2023-39968) |
| Probe the loopback port from an unprivileged same-host process | I, E | `SO_PEERCRED`/`LOCAL_PEERCRED` same-UID filtering (drop foreign-UID connections); every endpoint requires the bearer | A *same-UID* process can still connect to `/` — but a same-UID process can already read `~/.opentrapp/.env` directly, so this grants it nothing new (accepted, see residuals) | `untested-on-capable-hardware`: peer-cred drop on platforms that support it; documented residual on those that don't | [ADR-0022](adr/0022-daemon-control-surface.md) §3.6 |
| Use a hostile browser extension to forge `Origin` / read page memory and reach the server | S, I, E | Bearer in `Authorization` (not an ambient cookie an extension inherits); **on-demand/ephemeral** server (closed except during an explicit config session); CSP (`default-src 'self'`, pinned `connect-src`, no inline) | **The sharpest T8 residual.** An extension granted host-permissions for `127.0.0.1` can forge `Origin` and read `sessionStorage` *while a session is open*. Mitigated (ephemeral window, no ambient auth), not eliminated — but such an extension already has broad local reach | `untested-on-capable-hardware`: CSP + ephemeral lifetime on the spike; residual is an explicit owner-review item | [ADR-0022](adr/0022-daemon-control-surface.md) §3; [research](de-tauri-viewer-research.md) §"harden secondary channels" |
| Exfiltrate the secret through a server log on error (5xx) or an open redirect | I | Never log `Authorization`/`Cookie`; no open redirects in the served UI; the live-verification asserts "secret never appears in logs" | A logging or redirect regression — caught by the boundary self-test's token-not-in-logs assertion if that assertion is wired (see spike status) | `untested-on-capable-hardware`: log-absence assertion on the spike (Jupyter CVE-2022-24758 lesson) | [research](de-tauri-viewer-research.md) §"anti-patterns" |
| Exploit a standing 24/7 network surface on the always-on daemon | I, E, D | The perimeter-owning daemon itself exposes **no** network service; its only IPC is the existing file control channel. The config server starts **only** on explicit user action (`opentrapp configure` / launcher) and is torn down on session end / idle timeout | Exposure window equals *active config sessions*, not 24/7 — this is a hard requirement, not an option | Design invariant ([ADR-0022](adr/0022-daemon-control-surface.md) §2/§3); reconciles CLAUDE.md §10 "no network services" | [ADR-0019](adr/0019-headless-daemon-gui-viewer-split.md) (daemon = headless owner) |

**Residual risks specific to T8.**

- **The hostile-extension path (row 6) is the load-bearing residual, and the honest reason this is a *judgment* call.** No server-side control can fully stop a browser extension the user installed that holds host-permissions for `127.0.0.1` and is active during an open config session — it can forge `Origin` and read `sessionStorage`. The structural mitigations are (a) the bearer is not an ambient cookie, so a *passive* extension does not inherit auth, and (b) the server is **ephemeral**, so the window is active config sessions rather than always-on. The accepted framing: an extension with that capability already has broad reach into everything the browser touches; OpenTrApp's transient config surface is not the weakest link. **If a maintainer judges this unacceptable for a security tool, that is the KILL criterion → fall back to C1 (keep Tauri, track upstream GTK4), which clears the 19 advisories without any local server.**
- **The same-UID probe (row 5) is an accepted non-issue.** OpenTrApp runs *as the user*; a process at the same UID can already read the runtime `.env` and `~/.opentrapp` directly. A peer-cred drop is defense-in-depth, not a boundary — the boundary is that the agent (the untrusted party) is in a container at a different trust tier, never on the host as the user.
- **Everything in the Evidence column is `untested-on-capable-hardware`.** The controls are implemented and unit/integration-tested on the spike branch (mechanical PASS-candidate, below), but the *live* checks (a real browser doing a real rebinding/CSRF attempt, the secret-not-in-logs scan, the UX) have not run on capable hardware — this dev box swap-storms under the full stack (§11). They are owner gates, not agent-closable claims.

### T8 attack-surface comparison — status quo vs. the de-Tauri target

This is the egoless go/no-go input: is **browser + on-demand loopback** a *net* security improvement over the **in-process Tauri WebKit** it replaces?

| Dimension | Status quo: in-process Tauri/WebKit | Target: browser + on-demand loopback |
|-----------|--------------------------------------|--------------------------------------|
| Renderer patching | Pinned `webkit2gtk-4.1`; ages with the release; 19 unmaintained GTK3 advisories | The user's own continuously-patched browser |
| Shipped attack surface | GTK3 / WebKit / libsoup native stack inside the bundle | No webview shipped — but a local HTTP/WS control server exists |
| Network reachability of the UI surface | None (sandboxed in-process IPC, strict CSP) | A `127.0.0.1` server reachable by the browser (rebinding / CSRF / extension class) |
| Exposure window | Process lifetime | **Ephemeral** — only during explicit config sessions |
| Privilege of the surface | Daemon already owns the perimeter (no new privilege) | Same daemon, same privilege — the surface is new, the privilege is not |
| Worst realistic attack | A memory-safety RCE in the pinned WebKit engine (unpatchable without upstream GTK4) | A localhost-CVE (rebinding/CSRF) **iff** a server-side control regresses |
| Residual after controls | An aging engine that cannot be patched until upstream ships GTK4 | The hostile-extension path (row 6); same-UID probe (accepted) |

**Verdict framing (the maintainer's call, not the agent's).** Browser + loopback is a **net** improvement *iff*: (a) the six server-side controls hold — the spike's mechanical PASS; (b) the server is genuinely **ephemeral/on-demand**, shrinking the rebinding/extension window to active config sessions; and (c) the live-verification assertions run in the boundary self-test on every (re)start (per §11 / the [ADR-0018](adr/0018-idle-auto-pause-host-waker.md) resume contract). The trade is: swap a *standing* aging-WebKit RCE surface for a *transient*, well-understood, structurally-CSRF-immune localhost surface with consensus mitigations. Removing the 19 advisories is real but **secondary**; the primary security argument is "the renderer becomes the user's patched browser, and the new surface is transient and not ambient-authed." The conclusion is allowed to be **C1 (keep Tauri)** if the hostile-extension residual is judged unacceptable.

### T8 spike status — what is verified, and what stays an owner gate

- **Mechanically PASS-candidate on the spike branch `spike/de-tauri-loopback`** (scratch; merges nothing): the `crates/viewer-server` §2 middleware is implemented and its unit/integration tests are green — forged `Host` → 403, tokenless → 401, wrong-`Origin` → 403, token absent from the URL, bound address asserted loopback, WS first-frame auth, and `stream-line` carried over the WS. `cargo tree` on that crate is gtk/webkit-free. On `main`, `crates/viewer-server` is a `todo!()` scaffold, `exclude`d from the workspace until this gate clears (it moves to `members` at migration step 2).
- **Owner gates that remain — not agent-closable:** (1) **threat-model acceptance** of the residuals above, especially row 6 — *this section is the input*; (2) **real-browser UX** judgment for non-technical users (Chrome / Firefox / a WebKit browser) on capable hardware; (3) folding the live-verification assertions into `make boundary-selftest` so they run on every (re)start, not just at cold start (the [ADR-0018](adr/0018-idle-auto-pause-host-waker.md) security-correct-resume contract). Until (3) lands and runs on capable hardware, every T8 Evidence cell stays `untested-on-capable-hardware` by the §11 "unverifiable ≠ verified" rule.

---

## Coverage check against `trifecta.md` §7

This section verifies that every defense-in-depth layer named in [`trifecta.md`](trifecta.md) §7 has at least one corresponding attacker-capability row above. Rows are referenced by short layer name.

| Layer (from `trifecta.md` §7) | Covered by |
|-------------------------------|------------|
| Container hardening (vault-agent, §7.1 row 1) | T1 row "read a file outside the workspace"; T1 row "modify own configuration" |
| Network proxy / domain allowlist (§7.1 row 2, §7.2 row 4) | T1 row "fetch from attacker URL"; T2 row "second-stage payload" |
| Tool policy (§7.1 row 3) | T1 row "execute arbitrary shell command" |
| Exec controls (§7.1 row 4) | T1 row "execute arbitrary shell command"; T5 row "destructive operation from Telegram" |
| Workspace restriction (§7.1 row 5) | T1 row "read a file outside the workspace"; T5 row "access user's own files" |
| Kill switch (§7.1 row 6) | n/a (kill switch is operator action, not a per-attacker mitigation); documented in [`whitepaper.md`](whitepaper.md) §3.3 |
| Static scanner (§7.2 row 1) | T2 row "second-stage payload" |
| Line verifier (§7.2 row 2) | T2 row "previously-unknown line shape" |
| CDR rebuild (§7.2 row 3) | T2 row "formatting tricks" |
| Network isolation forge → agent (§7.2 row 5) | T2 row "talk to forge directly" |
| Feed scanner (vault-social; §7.3 row 1) | Opt-in / on-demand; live AT Protocol adapter shipped (ADR-0017), full build-out deferred |
| Network isolation vault-social → agent (§7.3 row 2) | Opt-in; isolation preserved structurally |
| DM pairing policy (§7.3 row 3) | T1 row "Telegram message to unapproved counterpart"; T5 row "compromise of Telegram account" |
| Coordinator approval (§7.3 row 5) | T5 row "destructive operation from Telegram" |

Every row in `trifecta.md` §7 is accounted for here. Layers marked "opt-in / deferred" (`vault-social`) remain documented as architectural slots; the attacker-capability rows under T1 ("hostile feed content") describe what the perimeter addresses when the layer is enabled.

---

## What this document does *not* cover

- **The agent's own reasoning.** Prompt-injection mitigation inside the LLM (system-prompt design, response-filtering, instruction-following discipline) is performed by Anthropic's API, not by this perimeter. The architecture assumes the reasoning will be subverted and engineers around that assumption.
- **The host's own security posture.** OS hardening, full-disk encryption, screen lock, two-factor on the user's accounts: all out of scope. A user concerned about T4 should harden the host independently.
- **Cryptographic protocol soundness.** TLS, signed releases, the Tauri auto-updater key: all relied upon as black-box primitives.
- **Supply-chain integrity of the perimeter itself.** Verifying that the binary the user installs is the binary the maintainer built is queued for the SLSA / cosign work in [`roadmap-post-launch.md`](roadmap-post-launch.md) §4. Until that lands, users wishing to verify the build pipeline are directed to *Building from source* in [`README.md`](../README.md).

---

## Shipped since this document was first written

The T3 DNS-rebinding defenses below were tracked as future work in the original draft and have since **shipped**. They are described in the reframed T3 residual-risks note above; full live re-verification of the layered behavior on capable hardware is still pending.

| Item | Status | Reference |
|------|--------|-----------|
| Post-resolve destination-IP filtering inside `vault-proxy.py` (`_resolves_to_private`, T3 DNS-rebinding row) | Shipped; unit-tested in [`infra/proxy/test_vault_proxy.py`](../infra/proxy/test_vault_proxy.py) | [`adr/0009-five-container-perimeter.md`](adr/0009-five-container-perimeter.md) Tier 2 |
| Kernel-level RFC1918 egress filter in the dedicated `vault-egress` sidecar (nftables drop) | Shipped; defined in [`infra/egress/nftables.conf`](../infra/egress/nftables.conf) | [`adr/0009-five-container-perimeter.md`](adr/0009-five-container-perimeter.md) Tier 4 |
| Pinned-resolver DNS over TLS (`unbound` forwarding only to Quad9 and Cloudflare) as a perimeter primitive | Shipped; configured in [`infra/egress/unbound.conf`](../infra/egress/unbound.conf) | [`adr/0010-pinned-resolver-dns.md`](adr/0010-pinned-resolver-dns.md) |

## Future work tracked from this document

| Item | Roadmap link |
|------|--------------|
| Certificate pinning for upstream Anthropic / Telegram endpoints | [`roadmap-post-launch.md`](roadmap-post-launch.md) §1 follow-up |
| Live re-verification of the layered T3 DNS-rebinding defenses on capable hardware | [`roadmap-post-launch.md`](roadmap-post-launch.md) §4 |
| Fuzzing of the CDR parser and generator | [`roadmap-post-launch.md`](roadmap-post-launch.md) §1 follow-up |
| Per-platform documentation of what persists after `compose down` | [`roadmap-post-launch.md`](roadmap-post-launch.md) §1 follow-up |
| Load-testing the proxy's rate-limiting threshold | [`roadmap-post-launch.md`](roadmap-post-launch.md) §1 follow-up |
| Friction-effect measurement on the per-action approval gate | `untested` (open research) |
| Reproducibility evidence for every "Evidence" cell in this document | [`roadmap-post-launch.md`](roadmap-post-launch.md) §4 |
| T8 loopback-viewer: live-verification (real-browser rebinding/CSRF, secret-not-in-logs) + non-technical-user UX on capable hardware; maintainer threat-model acceptance of the hostile-extension residual | spike-gated; [`adr/0022-daemon-control-surface.md`](adr/0022-daemon-control-surface.md) §0; [`de-tauri-viewer-research.md`](de-tauri-viewer-research.md) |

---

## Cross-references

- [`known-advisories.md`](known-advisories.md) lists the upstream dependency advisories the project knowingly accepts (chiefly the unmaintained Tauri GTK3 webview crates) with rationale, and explains how to read the OpenSSF Scorecard.
- [`README.md`](../README.md) "Limitations" cites this document as the authoritative residual-risk source.
- [`SECURITY.md`](../SECURITY.md) "In scope" / "Out of scope" align with the T1 through T5 categories above; T6 is described here for completeness even though most of T6 falls under "out of scope" for the vulnerability-reporting policy.
- [`trifecta.md`](trifecta.md) §7 is the layer-side companion to this attacker-side document; the *Coverage check* section above verifies the bidirectional correspondence.
- [`whitepaper.md`](whitepaper.md) §2 contains the conversational summary that this document expands.
- [`adr/0001`](adr/0001-proxy-side-api-key-injection.md), [`adr/0002`](adr/0002-adaptive-shell-levels.md), [`adr/0003`](adr/0003-content-disarm-reconstruction.md) document the three architectural choices most cited in the matrices.
