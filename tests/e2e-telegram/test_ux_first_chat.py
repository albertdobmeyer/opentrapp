"""Pass 1.5 — UX-signal capture for Karen's first-chat experience.

Run from `tests/e2e-telegram/`:
    source .venv/bin/activate
    pytest -xvs test_ux_first_chat.py

Companion to Pass 1's code-reading dogfood walkthrough
(`docs/specs/2026-04-28-dogfood-walkthrough-findings.md`). This module captures
**dynamic** signals on Moment 3 (first chat) that code-reading missed:

  - Round-trip latency (first-byte) per scenario, plus median + p95
  - Banned-term leaks in **bot replies** (the canonical 19 from
    `app/e2e/user-facing.spec.ts:13-33`, plus 6 codebase-narrative leaks
    Pass 1 flagged but the GUI banned-list hasn't picked up yet)
  - First-message warmth (greeting copy, expectation-setting)
  - Pairing-flow visibility (does scenario 1 just work?)
  - Helpful failure copy on edge-case prompts

This is a SIGNAL-COLLECTION pass — failures don't break the build. Banned-term
hits are logged to the JSONL transcript and surfaced in the session summary, but
do not fail the test (the doc author scores severity later).

Output:
  - artifacts/ux_first_chat_transcript.jsonl — one record per scenario
  - stdout — pytest summary + latency stats + banned-term hit table

Out of scope: modifying frontend code, modifying the bot's system prompt
(in `components/opencli-container` submodule), wizard live-run, post-wizard pages.
"""
from __future__ import annotations

import json
import statistics
import time
from pathlib import Path

import pytest

from helpers.bot_client import BotReply


pytestmark = [pytest.mark.ux_signals]


# Canonical 19-term list from app/e2e/user-facing.spec.ts:13-33 — the
# frontend rule. The bot is the same product surface for Karen, so the
# same rule applies.
GUI_CANONICAL_BANNED = [
    "OpenClaw Orchestrator",
    "OpenCli Container",
    "OpenSkill Forge",
    "OpenAgent Social",
    "MoltBook Pioneer",
    "container_runtime",
    "component.yml",
    "compose.yml",
    "seccomp",
    "MITRE ATT&CK",
    "proxy",
    "manifest",
    "Monorepo",
    "monorepo",
    "health probes",
    "configure components",
    "Checking prerequisites",
    "submodule",
    "Submodule",
]

# Pass 1 P0 codebase-narrative leaks the GUI banned-list hasn't picked up yet.
# Capturing in the bot's own replies confirms whether the system prompt /
# default OpenClaw greeting leaks them too. Surfaces the gap to the
# `components/opencli-container` submodule maintainer (out of parent scope to fix).
PASS1_P0_ADDITIONS = [
    "opencli-container",
    "openskill-forge",
    "vault-agent",
    "vault-proxy",
    "split-shell",
    "Split Shell",
    "Soft Shell",
    "Hard Shell",
    "sandbox",
    "Sandbox",
    "Podman",
    "podman",
    "Docker",
    "docker",
]

ALL_BANNED = GUI_CANONICAL_BANNED + PASS1_P0_ADDITIONS


# 8 Karen scenarios — see plan file ~/.claude/plans/steady-yawning-gosling.md
# Each tuple: (id, prompt, why-this-one, rubric-focus)
SCENARIOS: list[tuple[str, str, str, str]] = [
    ("01_start", "/start", "First contact; tests pairing flow + warmth", "P6 P7"),
    ("02_hi", "hi", "Informal opener; tests conversational tone", "P7"),
    ("03_what_can_you_do", "what can you do?", "Capability discovery; tests self-explanation", "P1 P6 P7"),
    ("04_news", "summarize today's news", "Productivity prompt + tool-use latency", "P3 P7"),
    ("05_tuesday", "plan my Tuesday", "Ready-hint exemplar; tests action concreteness", "P7 P8"),
    ("06_weather", "what's the weather?", "Ready-hint exemplar predicted to fail; tests error humanity", "P3"),
    ("07_ambiguous", "do the thing", "Ambiguous prompt; tests error humanity", "P3 P4"),
    ("08_help", "help", "Slash-command discoverability", "P5 P8"),
]


_REPO_ROOT = Path(__file__).resolve().parents[2]
_ARTIFACTS = _REPO_ROOT / "tests" / "e2e-telegram" / "artifacts"
_TRANSCRIPT_PATH = _ARTIFACTS / "ux_first_chat_transcript.jsonl"


@pytest.fixture(scope="module", autouse=True)
def _reset_transcript():
    """Truncate the transcript at module start so each run is a clean record."""
    _ARTIFACTS.mkdir(parents=True, exist_ok=True)
    _TRANSCRIPT_PATH.write_text("")
    yield


def _scan_banned(text: str) -> list[str]:
    """Return banned terms that appear in the reply (case-sensitive matches first;
    a few entries duplicate case to also catch lowercase forms)."""
    if not text:
        return []
    return [term for term in ALL_BANNED if term in text]


def _record(scenario_id: str, prompt: str, reply: BotReply | None,
            error: str | None, banned_hits: list[str],
            anthropic_call_count: int, blocked_event_urls: list[str]) -> None:
    """Append one structured record per scenario to the transcript."""
    record = {
        "ts": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "scenario": scenario_id,
        "prompt": prompt,
        "reply_text": reply.text if reply else None,
        "reply_text_len": len(reply.text) if reply else 0,
        "first_byte_latency_s": round(reply.latency_s, 2) if reply else None,
        "banned_term_hits": banned_hits,
        "anthropic_call_count": anthropic_call_count,
        "blocked_events": blocked_event_urls,
        "error": error,
    }
    with _TRANSCRIPT_PATH.open("a") as fh:
        fh.write(json.dumps(record) + "\n")


@pytest.mark.parametrize("scenario_id,prompt,why,rubric", SCENARIOS,
                         ids=[s[0] for s in SCENARIOS])
async def test_karen_scenario(bot, proxy_log, scenario_id: str, prompt: str,
                              why: str, rubric: str) -> None:
    """Send one Karen message, capture reply + signals, log to transcript.

    Failure mode philosophy: this is a SIGNAL-COLLECTION run, not enforcement.
    Banned-term hits and high latency get RECORDED in the transcript but do
    NOT fail the test. The findings doc author judges severity afterwards.

    Hard failures only on:
      - Unexpected BLOCKED events (regression guard — security boundary
        shouldn't have changed)
      - Telegram send-budget exceeded
      - Telethon-level errors (connection lost, etc.)
    """
    # Empty prompts are not allowed by Telegram, so SCENARIOS guarantees text.
    reply: BotReply | None = None
    error: str | None = None

    # 120s timeout is generous enough for tool-use loops on "summarize the
    # news" but still bounded. settle_ms=4500 matches chat.py's idle-drain
    # so we capture continuation messages (the F10 streaming pattern from
    # VERDICT-2026-04-25).
    try:
        reply = await bot.send_and_wait(
            prompt, timeout=120, settle_ms=4500,
        )
    except TimeoutError as e:
        # Treat timeout as a UX signal, not a test failure. The findings
        # doc will rank it (e.g., scenario 1 timing out = pairing-gate P0).
        error = f"timeout: {e}"
    except Exception as e:  # noqa: BLE001
        error = f"{type(e).__name__}: {e}"

    # Tally proxy events for this scenario.
    anthropic_calls = proxy_log.where(
        url_contains="api.anthropic.com", action="ALLOWED",
    )
    blocked = proxy_log.where(action="BLOCKED")
    blocked_urls = [getattr(e, "url", "?") for e in blocked]

    banned_hits = _scan_banned(reply.text if reply else "")

    _record(
        scenario_id=scenario_id,
        prompt=prompt,
        reply=reply,
        error=error,
        banned_hits=banned_hits,
        anthropic_call_count=len(anthropic_calls),
        blocked_event_urls=blocked_urls,
    )

    # Visible-in-pytest-output line so the run is readable in real time.
    if reply:
        excerpt = reply.text[:80].replace("\n", " ")
        print(f"\n[{scenario_id}] {reply.latency_s:.1f}s, "
              f"{len(reply.text)}c, {len(banned_hits)} banned hits | {excerpt}",
              flush=True)
    else:
        print(f"\n[{scenario_id}] FAILED: {error}", flush=True)

    # Regression guard: no BLOCKED events on benign first-chat traffic.
    # (Pattern from test_baseline.py:13-19.)
    assert not blocked, (
        f"[{scenario_id}] Unexpected BLOCKED events on benign prompt. "
        f"Security regression? URLs: {blocked_urls}"
    )


def test_session_summary() -> None:
    """Read the transcript at the end and emit summary stats.

    This test runs LAST (alphabetically) and depends on _reset_transcript.
    It computes median + p95 first-byte latency, total banned-term hits, and
    pairing-flow / first-message-warmth verdicts. Always passes — its job is
    to print the data for the findings-doc author.
    """
    if not _TRANSCRIPT_PATH.exists():
        pytest.skip("No transcript — earlier tests didn't run")

    records = [
        json.loads(line)
        for line in _TRANSCRIPT_PATH.read_text().splitlines()
        if line.strip()
    ]
    if not records:
        pytest.skip("Transcript empty")

    latencies = [
        r["first_byte_latency_s"] for r in records
        if r["first_byte_latency_s"] is not None
    ]
    total_banned_hits = sum(len(r["banned_term_hits"]) for r in records)
    total_anthropic_calls = sum(r["anthropic_call_count"] for r in records)
    failed = [r for r in records if r["error"]]

    lines = [
        "",
        "═" * 70,
        "Pass 1.5 — Live UX Signals — Session Summary",
        "═" * 70,
        f"Scenarios run:        {len(records)} / {len(SCENARIOS)}",
        f"Failures (timeouts):  {len(failed)}",
        f"Anthropic calls:      {total_anthropic_calls}",
        f"Banned-term hits:     {total_banned_hits} across {len(records)} replies",
    ]
    if latencies:
        lines.extend([
            f"Latency median:       {statistics.median(latencies):.1f}s",
            f"Latency p95:          "
            f"{statistics.quantiles(latencies, n=20)[-1]:.1f}s"
            if len(latencies) >= 4 else
            f"Latency max:          {max(latencies):.1f}s",
            f"Latency min:          {min(latencies):.1f}s",
        ])
    lines.append("─" * 70)
    lines.append(f"{'scenario':<20}{'lat(s)':>10}{'len(c)':>10}{'banned':>10}  prompt")
    lines.append("─" * 70)
    for r in records:
        lat = (f"{r['first_byte_latency_s']:.1f}"
               if r['first_byte_latency_s'] is not None else "—")
        rlen = str(r['reply_text_len'])
        nbanned = str(len(r['banned_term_hits']))
        prompt_excerpt = (r['prompt'][:34] + "…") if len(r['prompt']) > 35 else r['prompt']
        lines.append(f"{r['scenario']:<20}{lat:>10}{rlen:>10}{nbanned:>10}  {prompt_excerpt}")
    lines.append("─" * 70)
    if total_banned_hits > 0:
        lines.append("Banned-term breakdown:")
        from collections import Counter
        counter: Counter[str] = Counter()
        for r in records:
            counter.update(r["banned_term_hits"])
        for term, count in counter.most_common():
            lines.append(f"  {term:<30} × {count}")
    lines.append("═" * 70)
    lines.append(f"Transcript: {_TRANSCRIPT_PATH}")
    lines.append("═" * 70)

    print("\n".join(lines), flush=True)
