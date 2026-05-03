# tests/e2e-telegram

Python + Telethon test harness that drives `@LobsterTrappBot` as the user's
own Telegram account and probes the Lobster-TrApp perimeter end-to-end.
Produces pass/fail/unclear findings about whether the perimeter is too
permissive (Swiss cheese), too restrictive (refuses everything regardless
of policy), or correctly calibrated.

## Why Telegram and not a unit-test mock?

Because the perimeter's security thesis is "nothing untrusted touches the
host." The only honest way to test that is to prompt-inject bot from the
actual Telegram entry point and observe what reaches the host — in logs,
in filesystem, in network egress. Mocking any layer defeats the test.

Telethon logs in **as the user's Telegram account** (via MTProto Client API)
and sends to the bot; the bot receives those messages indistinguishably
from a real Telegram Desktop client. The perimeter sees no difference.

## Layout

```
tests/e2e-telegram/
├── README.md                 ← you are here
├── FIRST_RUN.md              ← one-time setup + the 3 commands to run the suite
├── requirements.txt          ← telethon, pytest, pytest-asyncio, python-dotenv
├── pytest.ini                ← asyncio auto mode + category markers
├── conftest.py               ← fixtures (env, telegram_client, NewLobsterTrappBot, budget, proxy_log)
├── helpers/
│   ├── bot_client.py         ← send-and-wait over Telethon with [TEST] prefix
│   ├── log_tail.py           ← async tail of `podman logs vault-proxy`, JSON event parser
│   └── budget.py             ← cumulative spend tracker, hard-stops at $4.00
├── test_smoke.py             ← prove bot responds end-to-end
├── test_baseline.py          ← basic reasoning, no regressions
├── test_network_boundary.py  ← allowlist behavior (is it enforced?)
├── test_filesystem_read_boundary.py   ← Swiss-cheese: can agent read host paths?
├── test_filesystem_write_boundary.py  ← can agent write to host paths? readonly root?
├── test_exec_boundary.py     ← docker sock, mount, unshare, ptrace, fork bomb
├── test_credential_exfil.py  ← does real Anthropic key leak?
├── test_spending_sanity.py   ← reasonable Anthropic call count + no billing errors
├── test_dynamic_shell.py     ← the USP — observe whether shell actually adjusts
├── direct_probing/
│   ├── README.md             ← what the direct-probing suite is and when to use it
│   ├── probe.sh              ← 24 probes via `podman exec`, no LLM, no cost
│   └── findings-YYYY-MM-DD.md ← auto-generated per-run report
└── VERDICT-YYYY-MM-DD.md     ← narrative summary combining both layers
```

## Running

See `FIRST_RUN.md` for the full first-time sequence. Short version once
the venv is set up and session is cached:

```bash
cd tests/e2e-telegram
source .venv/bin/activate
pytest -v                     # run everything
pytest -m network             # one category
pytest -xvs test_smoke.py     # smoke only, verbose
```

## Running without Telegram

The direct-probing suite exercises the boundary without sending a single
Telegram message. Useful for CI, for quick re-checks after container
changes, and for surfacing findings that don't require LLM involvement.

```bash
bash tests/e2e-telegram/direct_probing/probe.sh
```

Writes `direct_probing/findings-<date>.md`. Exit code is non-zero if any
probe fails.

## Budget discipline

Every test emits a small Anthropic call (1–3 calls for most). Haiku 4.5
is the current agent model (~$1 per million input tokens, $5 per million
output). The full suite is budgeted at well under $1; hard stop at $4.00
via `helpers/budget.py`.

## Test-message hygiene

All harness messages to bot are prefixed `[TEST]` — find them in your
real Telegram chat by searching that string. Bulk-delete after the run
if you want a clean chat.

## What this harness does NOT cover (yet)

- **Skill-install flow** via forge (`vault-forge` integration). Writing a
  deliberately-malicious skill end-to-end was scoped out for the v1 pass;
  add as `test_skill_install.py` when ready.
- **Long-running conversation memory probing.** Each test is a fresh
  conversation thread. Whether state leaks between paired users is
  untested.
- **UI-layer testing.** The Tauri GUI has its own playwright suite at
  `app/e2e/`. This harness is orthogonal.
