# vault-agent Setup Guide

This guide walks you through setting up vault-agent from scratch. No terminal experience required — just follow the steps in order.

---

## What You'll Need

- A computer running **Linux**, **macOS**, or **Windows** (with WSL2)
- An **Anthropic API key** (get one at [console.anthropic.com](https://console.anthropic.com/))
- A **Telegram account** (download from [telegram.org](https://telegram.org/) if you don't have one)
- About **30 minutes** for first-time setup

## What You'll Get

A safely contained AI agent on your computer that you control via Telegram. The agent:
- Can answer questions and help with tasks
- Cannot access your files, passwords, or accounts
- Cannot reach unauthorized websites
- Requires your approval for every action
- Can be stopped instantly at any time

---

## Step 1: Install the Container Runtime

The vault runs inside a container — a sealed box on your computer. You need Podman (recommended) or Docker to create this box.

**On Ubuntu/Debian Linux:**
```bash
sudo apt update
sudo apt install -y podman podman-compose
```

**On macOS:**
```bash
brew install podman podman-compose
podman machine init
podman machine start
```

**On Windows:**
1. Install [WSL2](https://learn.microsoft.com/en-us/windows/wsl/install) first
2. Open a WSL terminal, then run the Ubuntu commands above

**Verify it works:**
```bash
podman --version
```
You should see something like `podman version 4.9.3`.

---

## Step 2: Download the Vault

```bash
git clone https://github.com/albertdobmeyer/opentrapp.git
cd opentrapp/workloads/agent
```

If you don't have `git` installed:
```bash
sudo apt install -y git    # Linux
brew install git            # macOS
```

---

## Step 3: Create a Dedicated API Key

**Important:** Do NOT use your main API key. Create a dedicated one with a spending limit.

1. Go to [console.anthropic.com](https://console.anthropic.com/)
2. Navigate to **API Keys**
3. Click **Create Key**
4. Name it something like "vault-test"
5. Go to **Plans & Billing** → **Usage Limits**
6. Set a **hard limit** (e.g., $5) — this caps your spending if anything goes wrong

**Why a separate key?** The vault protects your key (the agent never sees it), but a spending cap is defense-in-depth. If something unexpected happens, the damage is capped at $5.

---

## Step 4: Set Up Your API Key

Run the setup script:
```bash
bash scripts/setup.sh
```

It will ask for your API key. Paste it when prompted (the text won't be visible as you type — this is normal, it's a security feature).

**Or do it manually:**
```bash
echo 'ANTHROPIC_API_KEY=sk-ant-your-key-here' > .env
chmod 600 .env
```

The `.env` file is never uploaded anywhere — it stays on your computer and is only read by the proxy container.

---

## Step 5: Create Your Telegram Bot

Your AI agent communicates through a Telegram bot. Here's how to create one:

1. Open **Telegram** on your phone or computer
2. Search for **@BotFather** and start a chat
3. Send: `/newbot`
4. Choose a **display name** (e.g., "My OpenClaw Agent")
5. Choose a **username** ending in `_bot` (e.g., `my_openclaw_agent_bot`)
6. BotFather will give you a **token** — it looks like `123456789:ABCdefGHIjklMNOpqrsTUVwxyz`

**Add the token to your vault:**
```bash
echo 'TELEGRAM_BOT_TOKEN=your-token-here' >> .env
```

Replace `your-token-here` with the actual token from BotFather.

**About your bot:**
- The bot is completely separate from your personal Telegram account
- Nobody can access your personal messages through the bot
- You can delete the bot anytime via @BotFather (`/deletebot`)
- You can create as many bots as you want

---

## Step 6: Build and Start the Vault

```bash
podman build -t vault-agent -f Containerfile .
podman tag vault-agent vault-agent:latest
podman-compose up -d
```

The first build takes 2-5 minutes (it downloads and configures everything). Subsequent starts are much faster.

**Wait about 60 seconds** for everything to initialize, then check that it's running:
```bash
podman ps
```

You should see two containers running:
- `vault-agent` — the AI agent (safely contained)
- `vault-proxy` — the security firewall

---

## Step 7: Connect to Your Bot

1. Open **Telegram** on your phone
2. Search for your bot's username (the one you created in Step 5)
3. Send any message (e.g., "Hello")
4. The bot will respond with a **pairing code** — something like `YVBBRLB6`
5. Back in your terminal, run:

```bash
podman exec vault-agent openclaw pairing approve telegram YOUR_CODE_HERE
```

Replace `YOUR_CODE_HERE` with the actual code from Telegram.

6. You should see: `Approved telegram sender <your-id>.`

**That's it — your agent is ready!** Send it a message in Telegram and it will respond.

---

## Step 8: Verify Security

Run the security check to confirm everything is locked down:
```bash
bash scripts/verify.sh
```

You should see **24/24 checks passed**. This confirms:
- The agent cannot access your files
- The agent cannot reach unauthorized websites
- Your API key is not visible to the agent
- The agent runs as a restricted user (not admin/root)
- All network traffic goes through the security proxy

---

## Using Your Agent

### What You Can Do
- **Ask questions:** "What's the capital of France?"
- **Get help writing:** "Draft a short email to my boss about being late"
- **Brainstorm:** "Give me 5 ideas for a birthday gift for my mom"
- **Learn:** "Explain how solar panels work in simple terms"

### What the Agent Can't Do (By Design)
- Access your files, photos, or documents
- Read your passwords or accounts
- Send messages as you on other apps
- Install software on your computer
- Access any website not on the approved list
- Run commands on your computer

### Stopping the Agent

**Soft stop** (keeps the session for review):
```bash
bash scripts/kill.sh --soft
```

**Hard stop** (removes everything):
```bash
bash scripts/kill.sh --hard
```

**Starting again after a stop:**
```bash
cd /path/to/opentrapp/workloads/agent
podman-compose up -d
```

Wait about 60 seconds, then pair Telegram again (see Step 7).

**Starting again after a reboot:**
Same as above. The containers don't auto-start after a reboot — you must run `podman-compose up -d` manually.

### Startup Timing

| What happens | How long |
|-------------|----------|
| Containers created | ~4 seconds |
| Proxy generates security certificate | ~10 seconds |
| OpenClaw reads config and starts | ~30 seconds |
| Gateway ready, Telegram connected | **~60 seconds total** |

You'll know it's ready when the bot responds to your Telegram message.

### Why You Need to Re-Pair After Every Restart

In Hard Shell mode, the agent has **no persistent memory**. Every restart is a clean slate — including the Telegram pairing. This is a security feature: if the container were ever compromised, restarting it wipes all session data. (In Split Shell mode, memory and pairing persist across restarts via a persistent volume.)

This means:
- After every `podman-compose up -d`, send a message to your bot and approve the new pairing code
- The bot won't remember previous conversations after a restart
- Your API key and bot token are preserved (they're in `.env`, not in the container)

### Memory Usage

The vault uses approximately **500 MB of RAM** when running (two containers). On machines with less than 4 GB of total RAM, this may cause slowdowns. Stop the vault when not in use with `bash scripts/kill.sh --soft`.

---

## Important: The Agent May Claim False Capabilities

The AI agent may sometimes **claim it can do things it actually cannot**. For example, it might say "I ran a command and here are the results" when it actually has no ability to run commands in Hard Shell mode.

**This is a known behavior of smaller AI models.** The agent isn't being malicious — it's trying to be helpful and fills in gaps with plausible-sounding answers. But it can be misleading.

**How to verify:** If the agent claims it did something (ran a command, read a file, accessed a website), you can check the proxy logs:

```bash
podman exec vault-proxy cat /var/log/vault-proxy/requests.jsonl | tail -10
```

If the action doesn't appear in the logs, the agent fabricated it. In Hard Shell mode, the agent can ONLY have conversations — it cannot run commands, read files, or access websites beyond the approved list. In Split Shell mode, the agent can execute approved file commands, but every command requires your Telegram approval first.

**Rule of thumb:** Trust the proxy logs, not the agent's claims.

---

## Troubleshooting

### "Container not found" errors
Make sure both containers are running:
```bash
podman ps -a
```
If they exited, check the logs:
```bash
podman logs vault-agent
podman logs vault-proxy
```

### Telegram bot doesn't respond
1. Check the vault is running: `podman ps`
2. Check for errors: `podman logs vault-agent 2>&1 | grep -i error`
3. Make sure you approved the pairing code (Step 7)

### "Proxy CA cert not found" error
The proxy container might not have started yet. Wait 30 seconds and try:
```bash
podman-compose down
podman-compose up -d
```

### Build fails
Make sure you have enough disk space (`df -h`) and memory (`free -h`). The build needs about 1 GB of disk and 2 GB of RAM.

### Moved the project folder and containers won't start
If you moved the `opentrapp` directory to a different location, the existing containers have the old path baked in. Fix:
```bash
podman rm -f vault-proxy vault-agent
podman-compose up -d
```
This recreates the containers with the correct paths.

### Agent claims to run commands but nothing happens
This is normal in Hard Shell mode. The agent's tools are disabled — it cannot run commands, read files, or browse the web. If it claims otherwise, it's fabricating. See the "Important: The Agent May Claim False Capabilities" section above.

---

## Security Overview

The vault creates three layers of protection:

```
YOUR COMPUTER (safe — the agent can't touch this)
    |
    |  ┌─ Security Proxy (vault-proxy) ──────────────┐
    |  │  ✓ Blocks unauthorized websites              │
    |  │  ✓ Injects your API key (agent never sees it)│
    |  │  ✓ Logs every network request                │
    |  └──────────────────────────────────────────────┘
    |           |
    |  ┌─ Agent Container (vault-agent) ─────────────┐
    |  │  ✗ Cannot access your files                   │
    |  │  ✗ Cannot run programs on your computer       │
    |  │  ✗ Cannot install software                    │
    |  │  ✗ Cannot see your passwords or accounts      │
    |  │  ✗ Cannot reach unapproved websites           │
    |  │  ✓ Can answer questions via Telegram          │
    |  └──────────────────────────────────────────────┘
```

Your API key is stored only in the proxy, never in the agent container. Even if the agent were compromised, it cannot see or steal your key.

---

## Next Steps

- **Explore the settings:** Check `config/openclaw-hardening.json5` to see the security configuration
- **View proxy logs:** `podman exec vault-proxy cat /var/log/vault-proxy/requests.jsonl` shows every request the agent made
- **Learn about shell levels:** The vault supports three security levels — Hard Shell (conversation only), Split Shell (file commands with approval), and Soft Shell (broad autonomy: web search, scheduling, autonomous file work). Switch with `make hard-shell`, `make split-shell`, or `make soft-shell`.
