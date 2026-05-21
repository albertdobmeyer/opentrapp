# ADR-0012 — Subscription / OAuth authentication feasibility

**Status:** Proposed — research only, no implementation committed
**Decision date:** 2026-05-20
**Prompted by:** Karen full-arc dogfood (2026-05-20). A non-technical operator asked, reasonably: *"Why must I paste an API key? Why can't I use my Claude subscription?"* This ADR records whether subscription/OAuth auth can coexist with the perimeter's security model, and at what cost. It deliberately does **not** authorize a build.

---

## Context

OpenTrApp wraps OpenClaw, whose reasoning runs on a remote LLM API. Today the only supported credential is a **per-provider API key** (`ANTHROPIC_API_KEY`, optionally `OPENAI_API_KEY`), pasted by the user and held by `vault-proxy`, which injects it at the HTTP boundary so the agent never sees it ([ADR-0001](0001-proxy-side-api-key-injection.md)).

This is a real adoption barrier for the non-technical persona the GUI targets: a metered, pay-per-token API key is a developer artifact. Many such users already pay for a Claude Pro/Max subscription and expect to use it. Claude Code itself supports subscription login, so the expectation is not unreasonable.

## The technical question

Can a Claude Pro/Max **OAuth/subscription** login replace the pasted API key **without weakening** the perimeter?

### Findings (from code/submodule investigation, 2026-05-20)

1. **OpenClaw is API-key-only in our integration.** No `oauth` / `login` / `device-flow` / `subscription` paths exist in `components/opencli-container/` (entrypoint, proxy addon, config). The proxy (`proxy/vault-proxy.py`) handles exactly two credential shapes: Anthropic `x-api-key` header and OpenAI `Authorization: Bearer`.

2. **The egress allowlist would block the OAuth flow.** `components/opencli-container/proxy/allowlist.txt` permits `api.anthropic.com`, `api.openai.com`, `api.telegram.org` (+ `clawhub.ai` in Soft Shell). An OAuth device/browser flow redirects through `claude.ai` / `console.anthropic.com` login + token endpoints, which are **not** allowlisted — and per project policy the allowlist is intentionally minimal.

3. **The injection model is header-substitution, not token-bearing.** ADR-0001's value is that the *agent container holds no credential* — `vault-proxy` swaps a placeholder for the real key at egress. OAuth introduces (a) a browser-based interactive login, (b) a short-lived access token + refresh token that must be **stored and rotated somewhere**, and (c) a token endpoint round-trip. None of these fit cleanly behind a static header-injection proxy.

4. **Where would the token live?** The security win of ADR-0001 is *the agent never holds the secret*. To preserve that, the OAuth tokens (and refresh logic) would have to live in `vault-proxy`, and the proxy would have to perform the device-flow + refresh on the agent's behalf — i.e. the proxy becomes an OAuth client, not just a header rewriter. That is a materially larger trusted component.

## Options (not decided)

- **A — Stay API-key-only (status quo).** Document plainly in onboarding *why* a key is needed and what it costs. Zero engineering; resolves the *confusion* even if not the *friction*. Aligns with the prosumer reality of the product (a security wrapper for people already running open agents).
- **B — Proxy-mediated OAuth.** `vault-proxy` becomes an OAuth client: handles the device flow, stores access+refresh tokens, refreshes them, and injects the bearer at egress — preserving "agent never holds the secret." Requires: allowlisting the auth + token endpoints (narrowly, by exact host), a token store with rotation, refresh-failure UX, and a redesign of the injection layer. Largest scope; must be threat-modeled before any code.
- **C — Host-side broker.** The Tauri app (host) performs the OAuth login in the system browser, then hands a short-lived token to `vault-proxy` over the existing IPC, and re-brokers on refresh. Keeps the agent container credential-free and the proxy simpler, but moves a secret onto the host process (acceptable — the host already holds the API key today before writing `.env`).

## Cost / risk estimate

- **Engineering:** B is a multi-week effort touching the most security-sensitive component (the proxy) + the credential layer + onboarding UX. C is smaller but still a new flow + token lifecycle. Neither is a "setting."
- **Security:** any option that adds endpoints to the allowlist or stores rotating tokens expands the trusted surface; must be evaluated against the threat model (T1 agent compromise must still not yield a usable long-lived credential).
- **External dependency:** subscription-OAuth for third-party agent tooling depends on Anthropic's terms and the availability of a supported OAuth/device-flow for non-first-party clients — to be verified before committing. If unsupported for third-party clients, B and C are both blocked upstream.

## Recommendation (for a future decision, not this ADR)

Default to **Option A** until there is evidence of (1) Anthropic supporting subscription OAuth for third-party clients and (2) demand that justifies expanding the proxy's trusted surface. If pursued, prefer **Option C** (host-side broker) over B — it keeps `vault-proxy` a header-rewriter rather than a full OAuth client, minimizing growth of the most sensitive component. Either path requires its own spec + threat-model review per the opencli-container "spec-driven development" rule before code.

## Consequences

- Onboarding should, in the near term, **explain** the API-key requirement in plain language rather than imply it can be avoided (see the v0.5.1 onboarding-UX work).
- This ADR is a standing reference for the recurring "why a key?" question; it should be linked from the wizard's key-entry help.
