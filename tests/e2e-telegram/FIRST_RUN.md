# First-run instructions (morning of 2026-04-24)

**Total time: ~5 minutes of your attention, then the harness runs on its own.**

## Prerequisites (already done, verify only)

- [x] `.env.test` populated at repo root (api_id, api_hash, phone, bot handle, session path)
- [x] `~/.opentrapp/test-sessions/` exists with mode 700
- [x] `podman ps` shows 5 containers up (vault-proxy, vault-skills, vault-social, vault-agent, vault-egress)
- [x] `@LogoTrappBot` paired to your Telegram user id (happened last night)
- [x] Anthropic credits loaded ($5)
- [x] vault-proxy patched to redact bot tokens in logs (submodule 4f5b560, parent 0ac3e9e)

If any of the above is stale, run `bash tests/e2e-telegram/direct_probing/probe.sh` — it re-validates most of it in 30 seconds.

## The 3 commands

### 1. Create the venv and install deps (~1 min)

```bash
cd ~/Repositories/opentrapp/tests/e2e-telegram
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

The .venv is gitignored. ~100 MB disk.

### 2. First-ever Telethon login (~2 min, interactive)

```bash
pytest -xvs test_smoke.py
```

What to expect:

1. Telethon connects to Telegram.
2. It prints: `Please enter the code you received:` (or similar)
3. **Check your Telegram app.** You'll get a message from "Telegram" (the official account — not from bot). The message contains a 5-digit login code.
4. Paste that code at the pytest prompt, press Enter.
5. If your account has 2FA password, Telethon asks for that too — paste it.
6. The session file is written to `~/.opentrapp/test-sessions/harness.session` (SQLite). All future runs skip this prompt.
7. `test_smoke.py` runs 2 tests: "does bot reply to ping" and "did the request touch the proxy." Both should pass in ~5s.

If smoke passes, you're unblocked. Proceed to step 3.

### 3. Full probing suite (~5–10 min, automated)

```bash
pytest -v
```

Runs 9 test files covering: baseline, network egress, filesystem read/write boundaries, exec boundaries, credential exfiltration, spending sanity, dynamic-shell observation. All prefixed `[TEST]` in your Telegram chat with bot.

Expected run cost: **~$0.15–$0.40** against your $5 Anthropic credit. The `BudgetTracker` hard-stops at $4.00 as a safety cap.

Expected duration: 5–10 min (each test makes 1–3 messages, each round-trip is 2–10s).

After the run: `tests/e2e-telegram/VERDICT-<date>.md` gets appended with findings from the automated suite (direct-probing findings from last night are in `VERDICT-2026-04-23.md`).

## Troubleshooting

### "Telethon says: The phone number is already connected"
This is normal if you restarted and have an existing session. Telethon will continue without re-prompting.

### "bot isn't replying — test_smoke timed out"
Run `podman ps` to confirm all five containers are up. If vault-agent is missing, `podman compose up -d` from repo root. See last night's commit `0ac3e9e` for what changed.

### Anthropic billing error
`podman exec vault-proxy python3 -c 'import os, urllib.request, json; key=os.environ["ANTHROPIC_API_KEY"]; print(urllib.request.urlopen(urllib.request.Request("https://api.anthropic.com/v1/messages", data=json.dumps({"model":"claude-haiku-4-5","max_tokens":5,"messages":[{"role":"user","content":"hi"}]}).encode(), headers={"x-api-key":key,"anthropic-version":"2023-06-01","content-type":"application/json"}), timeout=10).status)'`
Expect HTTP 200. Anything else → check https://console.anthropic.com/settings/plans for credits.

### "asyncio_mode already registered" or fixture errors
Make sure you `source .venv/bin/activate` before every pytest run.

### Test messages cluttering your Telegram chat
They're all prefixed `[TEST]`. Search "[TEST]" in the chat to find+bulk-delete later.

## After the suite passes

- Read `VERDICT-<date>.md` for the writeup
- Commit any green-to-red findings that are actionable
- Phase 5 (report) is auto-generated; Phase 4 (this suite) has run
