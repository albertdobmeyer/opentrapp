# Decision: Telegram Bot Token Stays in the Container

**Date:** 2026-03-30
**Phase:** 5b
**Decision:** Keep the Telegram bot token in the vault container environment. Do NOT move it to proxy injection.

## Context

Our architecture injects API keys at the proxy level — the real key never enters the vault container. The question: should we do the same for the Telegram bot token?

## Analysis

**How API key injection works (current):**
- Container has a placeholder key in auth-profiles.json
- Proxy intercepts requests to `api.anthropic.com`
- Proxy replaces the `x-api-key` HTTP header with the real key
- Clean: header replacement is a standard proxy operation

**How Telegram bot token works:**
- The token is part of the URL path: `https://api.telegram.org/bot<TOKEN>/sendMessage`
- OpenClaw reads `TELEGRAM_BOT_TOKEN` from the environment to construct the URL
- The token is NOT in an HTTP header — it's embedded in the request path

**What proxy injection would require:**
- Give the container a placeholder token (e.g., `PLACEHOLDER_TOKEN`)
- Proxy intercepts requests to `api.telegram.org`
- Proxy rewrites the URL path: `/botPLACEHOLDER_TOKEN/sendMessage` → `/bot<REAL_TOKEN>/sendMessage`
- URL rewriting is more fragile than header replacement
- Any change in Telegram API URL structure breaks the rewriting
- Edge cases: URL encoding, path parameters, query strings

## Decision: Don't Move It

**Reasons:**
1. **Complexity vs benefit:** URL rewriting is fragile. Header replacement is standard. The benefit (consistency) doesn't justify the risk (breaking Telegram).
2. **Lower risk:** A Telegram bot token is revocable via @BotFather in seconds. An API key compromise has billing implications. Different threat levels.
3. **Already documented:** The compose.yml and CLAUDE.md both note that the bot token is the one secret that enters the container, and why.
4. **Defense-in-depth still holds:** Even if the container is compromised and the bot token leaked, the attacker gets control of a Telegram bot — not API billing, not user files, not system access. The bot can be revoked instantly.

## What We DO Instead

- Document the bot token as the one accepted exception to "no secrets in container"
- Ensure the token is not logged (proxy already redacts sensitive data)
- Recommend users create a dedicated bot for vault use (not their personal bot)
- Include bot token rotation in the kill switch documentation
