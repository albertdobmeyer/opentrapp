# Secondary-Telegram-account setup

**Why this exists:** The harness drives bot via Telegram's MTProto Client
API using a real Telegram user account. Telegram's anti-abuse can
(rarely, but non-zero) flag automated behavior. To protect the owner's
primary account, a secondary account dedicated to testing is
recommended.

## One-time setup (~15 min)

### 1. Get a second phone number

- **US easiest:** [Google Voice](https://voice.google.com) — free, SMS-capable, permanent
- **Other:** prepaid SIM, family member's line, eSIM

Keep the number somewhere safe; losing it locks you out of the account.

### 2. Create a secondary Telegram account

- On mobile: Settings → Add Account → New Number
- On Telegram Desktop: sidebar hamburger → Add Account
- Register with the new phone number
- Name it distinctly (e.g. "LogoTest") so you don't confuse accounts in the chat switcher

### 3. Register a new API app under the secondary account

1. Open https://my.telegram.org in a browser
2. Log in with the **secondary** phone number (NOT your primary)
3. API Development Tools → Create new application
4. Fields:
   - App title: `OpenTrApp Test Harness`
   - Short name: `logotest2` (or similar, lowercase)
   - URL: optional, `https://opentrapp.com` or blank
   - Platform: Other
5. Copy `api_id` (integer) and `api_hash` (32-char hex string)

### 4. Pair the secondary account with bot

bot doesn't recognize the new Telegram user_id yet. Trigger the pairing
flow:

1. From the secondary account in Telegram, search `@LogoTrappBot`
2. Send any message (e.g. `hi`)
3. bot replies with a pairing code plus the new user_id
4. Copy both values

On the host (one-time):

```bash
podman exec vault-agent openclaw pairing approve telegram <PAIRING_CODE>
```

After this, bot will treat the secondary account as authorized.

### 5. Update `.env.test`

Edit `/home/albertd/Repositories/opentrapp/.env.test`:

```
TELEGRAM_API_ID=<new integer from step 3>
TELEGRAM_API_HASH=<new hash from step 3>
TELEGRAM_PHONE=<new phone in +country-code format>
BOT_HANDLE=@LogoTrappBot
TELEGRAM_SESSION_PATH=/home/albertd/.opentrapp/test-sessions/harness

# Optional: cap daily sends to share the account budget across projects.
# Default 35; full opentrapp suite is ~30 sends.
TELEGRAM_DAILY_SEND_BUDGET=35
```

The bot handle stays the same (same bot, just a different user talks to it).

### 6. Run the suite

```bash
cd tests/e2e-telegram
source .venv/bin/activate
pytest -xvs test_smoke.py
```

Telethon will prompt for a login code — this time it arrives in the
**secondary** account's Telegram app. Paste it. Session is cached under
that account. All subsequent runs are non-interactive.

## Ongoing hygiene

- **Do not log into the secondary account from random devices.** Keep its
  session surface tight so Telegram doesn't flag "unusual access."
- **The Telegram app registration is shared across multiple personal
  projects** (one app per Telegram account; my.telegram.org doesn't
  permit a second one without support intervention). Treat the api_hash
  like a password reused across projects: leak in one project = leak in
  all. Logo-Trapp's `.env.test` keeps it gitignored; future projects
  must do the same.
- **Daily usage cap on the shared account: ~50/day.** Logo-Trapp's
  full suite is ~30 sends; the harness hard-stops at the
  `TELEGRAM_DAILY_SEND_BUDGET` env var (default 35) to leave headroom
  for other projects. Logo-Trapp has priority right now per user
  decision 2026-04-24; if another project needs the budget, raise the
  cap explicitly or schedule runs apart.
- **Do not use the secondary account for anything else.** No personal
  chats, no groups, no channel subscriptions. Pure test harness. Reduces
  attack surface if Telegram ever does ban it.
- **If the secondary account gets banned**, the session file becomes
  invalid. Repeat step 1 with a new number and try again.

## Ban risk assessment

Despite Telethon's generic warning on every login, this workload
(low-volume, self-directed, consistent home IP, no scraping) is not what
Telegram's ToS targets. The secondary account is defense-in-depth against
false-positive enforcement, not a response to an actual policy violation.
