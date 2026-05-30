#!/bin/sh
# OpenCli-Container entrypoint wrapper
#
# Handles both volatile (tmpfs/Hard Shell) and persistent (volume/Split Shell) modes.
# 1. Install or preserve OpenClaw config
# 2. Wait for mitmproxy CA cert
# 3. Install or preserve auth profile
# 4. Lock config read-only (prevent agent self-modification)
# 5. Install container constraints documentation
# 5.5. Verify installed skill integrity (abort if any skill lacks clearance or hash mismatches)
# 6. Exec into OpenClaw

CONFIG_SRC="/opt/openclaw-hardening.json5"
CONFIG_DST="/home/vault/.openclaw/openclaw.json"
AUTH_DIR="/home/vault/.openclaw/agents/main/agent"
AUTH_FILE="$AUTH_DIR/auth-profiles.json"
CERT="/opt/proxy-ca/mitmproxy-ca-cert.pem"

# --- 1. Config installation ---
# On tmpfs (Hard Shell): always copy — tmpfs is empty on start.
# On persistent volume (Split Shell): only copy on first run.
# The tool-control.sh script writes config directly to the volume when switching shell levels.
mkdir -p "$(dirname "$CONFIG_DST")"

if [ -f "$CONFIG_DST" ]; then
    echo "[vault] Existing config preserved (persistent volume)"
else
    if [ -f "$CONFIG_SRC" ]; then
        cp "$CONFIG_SRC" "$CONFIG_DST"
        echo "[vault] Config installed from image (first run)"
    else
        echo "[vault] ERROR: No config found at $CONFIG_SRC or $CONFIG_DST" >&2
        exit 1
    fi
fi

# --- 2. Wait for mitmproxy CA cert ---
echo "[vault] Waiting for proxy CA certificate..."
for i in $(seq 1 60); do
    if [ -f "$CERT" ] && grep -q "END CERTIFICATE" "$CERT" 2>/dev/null; then
        echo "[vault] CA certificate found."
        break
    fi
    sleep 1
done
if [ ! -f "$CERT" ] || ! grep -q "END CERTIFICATE" "$CERT" 2>/dev/null; then
    echo "[vault] ERROR: Proxy CA cert not found or incomplete after 60s. Aborting." >&2
    exit 1
fi

# --- 3. Auth profile ---
# Placeholder API key — the real key is injected by the proxy sidecar.
# Only create if it doesn't exist (preserves across restarts on persistent volume).
if [ ! -f "$AUTH_FILE" ]; then
    mkdir -p "$AUTH_DIR"
    cat > "$AUTH_FILE" << 'AUTHEOF'
{
  "profiles": {
    "anthropic:api": {
      "provider": "anthropic",
      "type": "api_key",
      "key": "sk-ant-api03-placeholder-vault-proxy-will-inject-real-key-placeholder"
    }
  },
  "order": {
    "anthropic": ["anthropic:api"]
  }
}
AUTHEOF
    echo "[vault] Auth profile installed (placeholder key — real key injected by proxy)"
else
    echo "[vault] Auth profile preserved (persistent volume)"
fi

# --- 4. Lock config read-only ---
# Prevent the agent from modifying its own security policy.
# OpenClaw reads the config fine with 444 permissions (tested 2026-03-31).
# The config is only writable during this entrypoint setup phase and via
# tool-control.sh --apply (which restarts the container).
#
# Why this matters: if an attacker gains code execution inside the container
# (bypassing tool policy), they could modify openclaw.json to disable deny
# lists, enable elevated access, or set exec to "full". OpenClaw hot-reloads
# config changes, so the escalation would take effect immediately.
# chmod 444 prevents this — the vault user (uid 1000) cannot write to files
# they don't have write permission on, and no-new-privileges + dropped
# capabilities prevent escalation to root.
chmod 444 "$CONFIG_DST" 2>/dev/null && \
    echo "[vault] Config locked read-only (self-modification prevented)" || \
    echo "[vault] WARNING: Could not lock config read-only"

# --- 5. Install container constraints documentation ---
# Tells the agent its ACTUAL capabilities inside this hardened container.
# Without this, the LLM hallucinates about having full system access.
CONSTRAINTS_FILE="/home/vault/.openclaw/workspace/CONSTRAINTS.md"
if [ ! -f "$CONSTRAINTS_FILE" ]; then
    mkdir -p "$(dirname "$CONSTRAINTS_FILE")"
    cat > "$CONSTRAINTS_FILE" << 'CONSTRAINTSEOF'
# CONSTRAINTS.md — Your Actual Capabilities in This Container

**READ THIS FIRST.** You're running in a protected environment your user set up so you can help them safely. Your capabilities are deliberately restricted. Do NOT claim capabilities you don't have.

## What You CAN Do
- Read, write, and edit files inside the user's workspace (`/home/vault/.openclaw/workspace/`)
- Run a small set of safe text/file commands (see safeBins below)
- Search your memory files
- Communicate via Telegram
- Analyze images with vision

## What You CANNOT Do (these are hard limits)
- See or touch any file outside the user's workspace
- Run network tools (curl, wget) or interpreters (python, node, bash) — they're not installed
- Delete files — `rm` is not installed
- Reach the open internet directly — your outbound traffic is filtered to a small list of trusted destinations
- Access the user's personal accounts, keys, passwords, or keyring
- Modify your own security settings
- Spawn sub-agents or background sessions
- Browse websites
- Send email
- Post to social media

## Safe Commands (safeBins)
These are the ONLY shell commands you can run:
cat, echo, mkdir, cp, mv, touch, date, head, tail, wc, sort, uniq, tr, cut, jq, tee

In Soft Shell, additional commands are available:
grep, sed, awk, diff, xargs, basename, dirname, env, test, printf

Everything else requires explicit user approval via Telegram.

## Network Access
You can only reach a small list of trusted destinations (currently the AI service that runs your reasoning, and Telegram). Everything else is blocked. You cannot reach the user's home network or other private networks.

## Why These Restrictions Exist
The user trusts you to work safely. These restrictions ensure that even if something goes wrong — a bug, a prompt injection, or a malicious skill — you cannot harm the user's system, steal their data, or run up their API bill beyond what the spending cap allows.

## When The User Asks You To Find A Skill

If the user asks you to *find* or *look for* a skill — *"find me something that handles CSV files"*, *"is there a skill for calendar stuff?"* — that is a perfectly reasonable request. Do not refuse it.

You cannot search the skill library yourself from inside this environment. But the user's desktop app can, safely. When asked, reply along these lines:

> *"I can't browse the skill library from in here, but your desktop app can. Open it and use **Browse the Skill Library** with `<their words>` as the search term. It'll show me a list of clean, vetted candidates and I'll help you pick one."*

After they run the search and pick a candidate, the desktop app handles the install (download → safety scan → rebuild → delivery to your workspace). When the new skill arrives in your workspace, confirm to the user that you can see it and offer to try it.

The reason for the hand-off: the desktop app's library-browse action runs inside a separate safety layer that can vet skills before they reach you. Letting it do the discovery — and letting the user confirm the pick — is how the system stays safe without making the user memorise skill names.

## When A New User First Messages You

When the user sends you a message and you have no prior conversation history with them on this chat (no past assistant responses in your session), this is their very first interaction with you.

Respond with EXACTLY this welcome message and inline keyboard, formatted as a single message:

> Hey! I'm your assistant. Here's what I can do — tap one to try:
>
>   [📅 Plan my Tuesday from this list of tasks]
>   [✉️ Draft an email to my landlord]
>   [📄 Summarize a PDF I send you]
>
> Or just type whatever you need help with. I'm here.

The three options are inline-keyboard buttons. Each button's text is exactly as shown; tapping a button sends that text as the user's next message. Use Telegram's \`reply_markup.inline_keyboard\` field. Use \`callback_data\` matching the button text so your handler treats it identically to a typed message.

Do NOT send this welcome on subsequent messages. Detect "first message" by checking your session log for any prior assistant turn with this user; if none, this is the first.

If the user's first message contains a request (not a /start), still send the welcome first, then process their request after.

## How To Talk About This With The User

The user is not a developer. When you explain why you can't do something, or how you keep their files safe, use plain everyday language. Reach for the words a person uses to describe their own home, not the words an engineer uses to describe a server.

Vocabulary guidance:

- When refusing access to a file outside the workspace, frame it as: *"I can only see files inside your workspace"* or *"that file isn't in the folder you've shared with me."*
- When explaining your network limits, frame it as: *"my outbound network is filtered to a small list of trusted destinations"* or *"I can only reach a few specific services — the AI service that runs my reasoning, and Telegram."*
- When explaining your overall safety posture, frame it as: *"there's a security layer around me, set up by your installation, so that even if something goes wrong I can't reach your personal files or run up your bill."*
- When explaining a refusal, name *what you can't do* and *why* (for the user's safety). Don't list internal techniques.

The user's mental model is "the assistant can do X but not Y." Don't replace it with technical-sounding labels — just explain in their language what's allowed and what's not.

**When reporting your capabilities to the user, be accurate. Refer to this file. Use the vocabulary in this section.**
CONSTRAINTSEOF
    echo "[vault] Constraints documentation installed"
else
    echo "[vault] Constraints documentation preserved"
fi

# --- 5.5. Verify installed skill integrity ---
# Abort startup if any skill in the workspace lacks a .trust record or has a hash mismatch.
# install-skill.sh writes a .trust file containing the SHA-256 of SKILL.md at install time.
# This check ensures no skill has been tampered with between install and container start,
# and that no skill was dropped in without going through install-skill.sh (which requires clearance).
SKILLS_DIR="/home/vault/.openclaw/workspace/skills"
if [ -d "$SKILLS_DIR" ]; then
    skill_fail=0

    # Check every skill directory that has a SKILL.md
    for skill_dir in "$SKILLS_DIR"/*/; do
        [ -d "$skill_dir" ] || continue
        [ -f "${skill_dir}SKILL.md" ] || continue
        sname=$(basename "$skill_dir")
        trust_file="${skill_dir}.trust"

        if [ ! -f "$trust_file" ]; then
            echo "[vault] ERROR: Skill '$sname' has no clearance record (.trust missing). Refusing to start." >&2
            skill_fail=1
            continue
        fi

        stored=$(grep '^VERIFY_HASH=' "$trust_file" 2>/dev/null | cut -d= -f2)
        if [ -z "$stored" ]; then
            echo "[vault] ERROR: Skill '$sname' .trust file has no VERIFY_HASH entry. Refusing to start." >&2
            skill_fail=1
            continue
        fi

        current="sha256:$(sha256sum "${skill_dir}SKILL.md" | cut -d' ' -f1)"
        if [ "$current" != "$stored" ]; then
            echo "[vault] ERROR: Skill '$sname' hash mismatch — possible tampering. Refusing to start." >&2
            skill_fail=1
        fi
    done

    if [ "$skill_fail" -eq 1 ]; then
        echo "[vault] Startup aborted: unverified skills detected." >&2
        echo "[vault] Remove affected skills or reinstall via: bash scripts/install-skill.sh --clearance <report>" >&2
        exit 1
    fi

    echo "[vault] Skill integrity verified."
fi

# --- 6. Start OpenClaw ---
echo "[vault] Starting OpenClaw..."
exec "$@"
