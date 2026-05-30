# Decision: Feed Scanning Integration Deferred

**Date:** 2026-03-30
**Phase:** 5c
**Decision:** Defer feed scanning integration until Moltbook domains are added to the allowlist.

## Context

OpenAgent-Social has a feed scanner (25 injection patterns) that detects prompt injection in agent social network content. When Hum eventually interacts with Moltbook, this scanner should protect against malicious feed content.

**Integration spec:** `openagent-social/docs/specs/2026-04-04-vault-integration-design.md` (written 2026-04-04, covers pattern export format, proxy integration, blocking policy).

## Current State

- Moltbook domains are NOT in the proxy allowlist (intentional)
- The agent CANNOT reach Moltbook from any shell level
- Feed scanning integration has no effect until Moltbook access is enabled
- Pioneer's feed scanner exists and works as a standalone host-side tool

## When This Becomes Relevant

Feed scanning integration becomes relevant when:
1. Soft Shell is designed and implements broader domain access
2. Moltbook API domains are added to the allowlist
3. The agent can actually interact with Moltbook

## Planned Approach (For Future Implementation)

**Recommended: proxy-level content inspection.**
- vault-proxy.py intercepts responses from Moltbook API domains
- Extracts post/comment content from JSON responses
- Runs pioneer's injection patterns against the content
- Flags or blocks responses containing injection patterns based on severity: CRITICAL patterns block the response, HIGH and MEDIUM patterns are logged but allowed through
- Logs all findings to the proxy log

This keeps the scanning on the HOST side (trusted), not inside the container (untrusted). The agent never sees flagged content.

## Why Not Now

Building this now would be untestable — we can't verify feed scanning works without Moltbook access enabled. Following our principle: never build what we can't test and validate.
