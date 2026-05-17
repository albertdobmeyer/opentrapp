# Dogfood Test Rig — Full Karen Arc

A test rig that simulates a non-technical user ("Karen") downloading the OpenTrApp desktop app, going through the wizard, and getting jobs done by chatting with the OpenClaw Telegram bot. Runs from a source build today; will run against a real binary once `v0.3.2` ships.

## What's here

| File | Role |
|---|---|
| [`../docs/specs/2026-05-05-dogfood-full-arc-spec.md`](../../docs/specs/2026-05-05-dogfood-full-arc-spec.md) | **The *what*.** Spec: persona, four tiers, 27 scenarios, success criteria, cost envelope. |
| [`test_full_arc.py`](test_full_arc.py) | **The *how (programmatic)*.** Telethon harness — drives Tier A & B chat scenarios, records JSON artefacts per scenario, scans replies for banned terms. |
| [`CHECKLIST.md`](CHECKLIST.md) | **The *how (manual)*.** Operator-facing checklist for the human-only parts (pre-flight, Tier C state-machine clicks, Tier D lifecycle exits, screenshots). |
| [`findings-template.md`](findings-template.md) | **The *where (results land)*.** Empty findings shell — copy to `docs/specs/2026-05-DD-dogfood-full-arc-findings.md` when running. |
| `corpus/` | Test fixtures (meeting notes, messy CSV, prompt-injection trap text) the harness asks the operator to attach. |
| `artifacts/` | Per-scenario JSON output written by the harness; gitignored. |

## How to run

The full arc takes roughly **70 minutes wall-clock** and **~$0.40** of Anthropic spend (cap $0.50). It can be run in pieces:

```bash
# Activate the existing e2e-telegram venv
cd tests/e2e-telegram && source .venv/bin/activate

# Run a single tier
cd ../dogfood
pytest -m dogfood_tier_a -xvs    # 5 happy-path scenarios, ~35 min, ~$0.30
pytest -m dogfood_tier_b -xvs    # 8 adversarial scenarios, ~10 min, ~$0.10
pytest -m dogfood_tier_c -xvs    # 7 AssistantStatus state scenarios, $0
pytest -m dogfood_tier_d -xvs    # 7 termination-path scenarios, $0

# Or all 27 in arc order
pytest -m dogfood_full -xvs
```

**Most scenarios in Tiers C and D are operator-driven** (skipped by the harness with a pointer at the relevant `CHECKLIST.md` section). The harness tells you what to do; you do it; you screenshot the result; you record in the findings.

**Tier A — A1, A4, A5 require manual file attachments** to the Telegram chat before the harness sends the prompt. The harness skips with a pointer when the prerequisite isn't met; the operator does the attach and re-runs that scenario alone.

## How to read the output

For each completed scenario, `artifacts/<scenario_id>.json` contains:

```json
{
  "scenario_id": "a4_install_skill_from_clawhub",
  "recorded_at": "2026-05-DD-T-Z",
  "prompt": "Find a CSV-formatting skill on ClawHub and install it for me.",
  "reply_text": "...",
  "reply_latency_s": 4.2,
  "wall_clock_s": 87.3,
  "banned_term_hits": []
}
```

The findings doc rolls these up into per-tier verdicts, the rubric re-score, and the ship/no-ship recommendation. See `findings-template.md` for the structure.

## What this rig **doesn't** do

- **Doesn't run the wizard.** That's the operator's job; the wizard is GUI-driven (Tauri + React) and not yet wrapped in an automated driver. The Telethon harness picks up *after* the wizard is complete and the bot is paired.
- **Doesn't replace the existing per-boundary tests** in `../e2e-telegram/`. Tier B references those (e.g. `test_credential_exfil.py` covers B1 in more depth); the dogfood rig adds an *arc-level* signal, it doesn't supersede the per-boundary work.
- **Doesn't enforce subjective UX failures.** Banned-term leaks, credential leaks, and architecture invariants (`verify.sh` start = end) are hard asserts. Awkward bot copy, slow latency, missed action items are *recorded* but don't fail the run; the operator scores severity in the findings doc. This is the same signal-collection model Pass 1.5 used.

## Why this exists

Pass 1.5 ([`../docs/specs/2026-04-29-live-signal-first-chat.md`](../../docs/specs/2026-04-29-live-signal-first-chat.md)) covered first-chat only — 8 scenarios, 79 seconds, ~$0.04. It moved Moment-3's score from 5.5 → 8.0/10 and surfaced banned-term leaks that closed in Pass 7. **Other Karen surfaces had not been exercised against a live perimeter under realistic load.** This rig closes that gap.

The user's direction on 2026-05-05 was explicit: *the most unconservative settings — push every defensive layer to its limits, real ClawHub installations, real Anthropic API, adversarial prompt-injection attempts, edge cases.*

## Hand-off pointer

The current state of the rig is `feat(test): full-arc Karen dogfood test rig` (see commit history). The findings doc is empty pending a run-session. See [`../docs/handoff.md`](../../docs/handoff.md) for the run-session mission statement.
