#!/bin/sh
# OpenCli-Container Security Patch: Fix Telegram Proxy Bypass
#
# Target: openclaw@2026.2.26 (dist/send-DslMV0Oj.js)
# Issue: https://github.com/openclaw/openclaw/issues/30338
# Upstream fix: PR #30367 (not included in 2026.2.x releases)
#
# Problem: applyTelegramNetworkWorkarounds() unconditionally replaces the
# global undici dispatcher with a plain Agent, destroying any ProxyAgent
# that was set for HTTP_PROXY/HTTPS_PROXY. This causes Telegram API calls
# to bypass the vault proxy, breaking our security model.
#
# Fix: When HTTP_PROXY or HTTPS_PROXY is set, use ProxyAgent instead of
# Agent for the global dispatcher. This preserves proxy routing for ALL
# network traffic, including Telegram.
#
# This patch can be removed when OpenClaw ships a version that includes
# the upstream fix (PR #30367).

set -eu

TARGET="/usr/local/lib/node_modules/openclaw/dist/send-DslMV0Oj.js"

if [ ! -f "$TARGET" ]; then
    echo "[patch] ERROR: Target file not found: $TARGET" >&2
    echo "[patch] This patch targets openclaw@2026.2.26. Version mismatch?" >&2
    exit 1
fi

# Verify we're patching the right file (check for the exact broken pattern)
if ! grep -q 'import { Agent, setGlobalDispatcher } from "undici"' "$TARGET"; then
    echo "[patch] ERROR: Expected import pattern not found in $TARGET" >&2
    echo "[patch] File may already be patched or OpenClaw version changed." >&2
    exit 1
fi

echo "[patch] Applying Telegram proxy fix to $TARGET"

# 1. Add ProxyAgent to the undici import
sed -i 's/import { Agent, setGlobalDispatcher } from "undici"/import { Agent, ProxyAgent, setGlobalDispatcher } from "undici"/' "$TARGET"

# 2. Replace the plain Agent with a proxy-aware conditional
# Before: setGlobalDispatcher(new Agent({ connect: {
#             autoSelectFamily: ..., autoSelectFamilyAttemptTimeout: 300 } }));
# After:  Check for HTTPS_PROXY/HTTP_PROXY and use ProxyAgent if set
sed -i 's/setGlobalDispatcher(new Agent({ connect: {/const __proxyUrl = process.env.HTTPS_PROXY || process.env.HTTP_PROXY; setGlobalDispatcher(__proxyUrl ? new ProxyAgent({ uri: __proxyUrl, connect: { autoSelectFamily: autoSelectDecision.value, autoSelectFamilyAttemptTimeout: 300 } }) : new Agent({ connect: {/' "$TARGET"

# Verify the patch was applied
if grep -q 'ProxyAgent' "$TARGET" && grep -q '__proxyUrl' "$TARGET"; then
    echo "[patch] SUCCESS: Telegram proxy fix applied."
    echo "[patch] All undici traffic will now respect HTTP_PROXY/HTTPS_PROXY."
else
    echo "[patch] ERROR: Patch verification failed." >&2
    exit 1
fi
