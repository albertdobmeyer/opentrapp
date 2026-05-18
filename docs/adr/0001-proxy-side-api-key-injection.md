# ADR-0001 — Proxy-side API-key injection

**Status:** Accepted
**Decision date:** 2026-03-23 (vault-proxy initial implementation); reaffirmed 2026-04-15 (architecture v2 redesign)
**Implemented by:** [`components/opencli-container/proxy/vault-proxy.py`](../../components/opencli-container/proxy/vault-proxy.py); [`components/opencli-container/scripts/entrypoint.sh`](../../components/opencli-container/scripts/entrypoint.sh); [`components/opencli-container/compose.yml`](../../components/opencli-container/compose.yml)
**Verified by:** Verification check 7 in [`components/opencli-container/scripts/verify.sh`](../../components/opencli-container/scripts/verify.sh) (`API keys absent from vault-agent's environment`)

---

## Context

The OpenClaw runtime makes outbound HTTPS calls to one or more LLM provider APIs (Anthropic, OpenAI). Each call requires an authentication credential — `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, or equivalent — which the runtime conventionally reads from a process environment variable.

The default OpenClaw configuration places this credential in the same process the agent runs in. The credential is consequently visible to:

1. The agent itself, when it executes a tool that reads `/proc/self/environ` or invokes a shell that inherits the environment
2. Any process that compromises the agent's runtime (a malicious skill, a successful prompt injection, an exploitable bug in the runtime)
3. Any process with read access to the agent's `/proc/<pid>/environ` (the user, root, anyone with `ptrace`)
4. Any backup or core-dump that captures the runtime's memory

The empirical record makes the cost of this exposure concrete. The Moltbook database breach (2026-01) exposed 1.5 M API tokens via a single misconfigured Supabase row-level-security policy; a non-trivial fraction of those tokens were embedded in OpenClaw runtime configurations. CVE-2026-25253 demonstrated a one-click RCE through OpenClaw's management API; an attacker who triggered it gained the runtime's environment, including the credential.

A complete containment story for the OpenClaw runtime cannot leave the credential in the agent's process environment.

## Decision

The user's API credentials are held by a dedicated proxy container (`vault-proxy`) and never enter the agent's container (`vault-agent`).

Mechanically:

1. The user places their real credentials in a host-side `.env` file with mode `0600`. The file is gitignored.
2. `compose.yml` sets the `vault-proxy` container's environment from the host `.env`. The `vault-agent` container's environment receives a placeholder string in the same variable name (e.g. `ANTHROPIC_API_KEY=PLACEHOLDER_KEY_REPLACED_BY_PROXY`).
3. `vault-proxy` is an mitmproxy-based addon (Python, ~150 lines) that runs in front of the agent's outbound HTTPS. The agent constructs API requests using the placeholder string in the `x-api-key` (Anthropic) or `Authorization` (OpenAI) header.
4. Before forwarding the request to the public internet, `vault-proxy` substitutes the placeholder string in the request header with the literal credential read from its own environment. The substitution is a single-pass byte-replace; the credential is never logged.
5. `vault-proxy` enforces a domain allowlist. Requests to non-allowlisted hosts are rejected with a logged 403 before any header substitution occurs.

The credential is consequently visible only to the `vault-proxy` process. `vault-agent` and any tool the agent runs see only the placeholder. `env | grep API` inside `vault-agent` returns nothing of value.

## Consequences

### Positive

- **The credential cannot leak through agent compromise.** A successful prompt injection, a malicious skill, or a runtime exploit operating from inside `vault-agent` exposes only the placeholder. The literal credential remains on the host inside the proxy container.
- **The credential cannot leak through agent logs or core dumps.** Whatever the agent process writes — including stack traces, crash dumps, debug logs, OpenClaw's own request audit log — captures only the placeholder.
- **Domain-level enforcement composes naturally with credential injection.** Because the proxy is already on the egress path to substitute the credential, adding an allowlist filter is structurally free.
- **The substitution is auditable.** Every substitution is logged (host, status, byte counts) to `vault-proxy/var/log/vault-proxy/requests.jsonl`. A user reviewing the log can verify which destinations the credential was used against.
- **Credential rotation is a single host-side file edit.** No image rebuild, no agent restart cycle that loses session state, no manual key-distribution dance. Edit `.env`, restart `vault-proxy` (~3 seconds), agent continues with the new credential transparently.

### Negative

- **`vault-proxy` becomes the single point of credential exposure.** A compromise of `vault-proxy` (rather than `vault-agent`) exposes the credential. The proxy is hardened with a custom seccomp profile and the same capability-drop discipline as the agent, but the proxy's seccomp policy is necessarily wider than the agent's because mitmproxy requires syscalls (notably `socket`-family operations and TLS interception primitives) that the agent does not need. The trade-off — wider syscall surface in `vault-proxy` in exchange for credential isolation in `vault-agent` — is documented in the [vault README's residual-risks section](../../components/opencli-container/README.md#residual-risks-the-operator-must-understand).
- **Allowlisted destinations can still be abused during a live session.** A compromised agent cannot read the credential but can issue arbitrary calls to allowlisted hosts using it. Mitigation: configure a hard spending cap on the API key. The cap is part of the security boundary, not a billing convenience.
- **TLS-interception MITM is required.** `vault-proxy` must terminate the agent's outbound TLS and re-establish TLS to the upstream provider (otherwise it cannot read the headers to substitute). This requires the agent to trust mitmproxy's CA inside the container, which is set up by `scripts/entrypoint.sh` and is invisible to the user. The mitmproxy CA is per-container and re-generated on container creation; it does not persist on the host.
- **Operationally, this rules out direct-to-API agent paths.** The agent cannot bypass the proxy even if a contributor adds a new tool that wants to make a direct outbound call. This is a feature (every egress is filtered and logged) but it is also a constraint that adding new outbound paths must respect.

### Neutral

- The agent's view of the API request is unchanged. From OpenClaw's perspective, it sees a placeholder credential and successful upstream responses; the substitution is invisible at the application layer.

## Alternatives considered

**(A) Environment-variable inheritance, with the credential present in the agent's container.** The OpenClaw default. Rejected for the reasons in the *Context* section.

**(B) File-mounted credential, with the agent reading from a shared volume.** Marginally better than (A) — the credential is at least not in the process environment — but still inside the agent's filesystem view. A read-capable compromise (most prompt-injection scenarios) still exfiltrates it.

**(C) HashiCorp Vault sidecar (or equivalent).** Architecturally similar to this design's `vault-proxy`, but built on a much larger trust dependency. HashiCorp Vault is operationally rich — token lifecycles, audit logging, dynamic secrets — but adds tens of megabytes of dependencies, an authentication-token bootstrap problem, and a non-trivial operational learning curve for non-developer users. The mitmproxy-based proxy is smaller, has the substitution semantics built in, and is already on the egress path for allowlist enforcement.

**(D) Per-tool credential broker.** Have each tool that needs the credential request it from a broker process. Rejected because OpenClaw's tools call upstream APIs through the runtime's HTTP client; intercepting at the per-tool level would require modifying every tool.

**(E) OS-keyring integration.** Have the agent retrieve the credential at runtime from the OS keyring (macOS Keychain, GNOME Keyring, KWallet). Rejected because this puts the credential in the agent's process memory at runtime — same exposure surface as (A) — and adds an OS-specific implementation surface the perimeter currently does not have.

## References

- Companion architecture document: [`docs/trifecta.md`](../trifecta.md) §4.4 (vault-proxy as the egress gateway)
- Whitepaper: [`docs/whitepaper.md`](../whitepaper.md) §1 (the "credential-adjacent" anti-pattern), §3.3, §4.1 layer 2
- Verification: 24-point startup check 7 ("API keys absent from environment") in [`components/opencli-container/scripts/verify.sh`](../../components/opencli-container/scripts/verify.sh)
- Implementation: [`components/opencli-container/proxy/vault-proxy.py`](../../components/opencli-container/proxy/vault-proxy.py)
- Empirical motivation: ClawHavoc study (2026-Q1); Moltbook database breach (2026-01); CVE-2026-25253
