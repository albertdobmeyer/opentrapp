"""Pytest fixtures for the dogfood arc — extends tests/e2e-telegram/.

The dogfood test (test_full_arc.py) reuses the Telethon harness, BotClient,
BudgetTracker, and ProxyLogTail from tests/e2e-telegram/. Pytest does not
auto-discover conftest.py files in *sibling* directories, so this conftest
re-exports the e2e-telegram fixtures by importing the fixture functions
directly. Each fixture below delegates to its e2e-telegram twin so we keep
exactly one source of truth.
"""
from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest
import pytest_asyncio
from dotenv import load_dotenv
from telethon import TelegramClient

# Make the e2e-telegram helpers importable.
_E2E_DIR = Path(__file__).resolve().parents[1] / "e2e-telegram"
sys.path.insert(0, str(_E2E_DIR))

from helpers.bot_client import BotClient  # noqa: E402
from helpers.budget import BudgetTracker  # noqa: E402
from helpers.log_tail import ProxyLogTail  # noqa: E402

_REPO_ROOT = Path(__file__).resolve().parents[2]
load_dotenv(_REPO_ROOT / ".env.test", override=True)


REQUIRED_ENV = [
    "TELEGRAM_API_ID",
    "TELEGRAM_API_HASH",
    "TELEGRAM_PHONE",
    "BOT_HANDLE",
    "TELEGRAM_SESSION_PATH",
]


@pytest.fixture(scope="session")
def env() -> dict[str, str]:
    missing = [k for k in REQUIRED_ENV if not os.environ.get(k)]
    if missing:
        pytest.fail(
            f".env.test missing required keys: {missing}. "
            f"Expected at {_REPO_ROOT / '.env.test'}"
        )
    return {k: os.environ[k] for k in REQUIRED_ENV}


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def telegram_client(env) -> TelegramClient:
    client = TelegramClient(
        env["TELEGRAM_SESSION_PATH"],
        int(env["TELEGRAM_API_ID"]),
        env["TELEGRAM_API_HASH"],
    )
    await client.start(phone=env["TELEGRAM_PHONE"])
    yield client
    await client.disconnect()


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def bot(telegram_client, env) -> BotClient:
    return BotClient(telegram_client, env["BOT_HANDLE"])


@pytest.fixture(scope="session")
def budget() -> BudgetTracker:
    # BudgetTracker has no cap parameter — it uses a class-level HARD_STOP_USD
    # ($4.00) and SOFT_WARN_USD ($1.00) defined as ClassVars. The dogfood arc
    # is expected to spend ~$0.40, so the default $4 stop gives 10× headroom.
    return BudgetTracker()


@pytest_asyncio.fixture(scope="session", loop_scope="session")
async def _session_proxy_tail():
    """Session-scoped raw tail. Mirrors the e2e-telegram canonical fixture."""
    async with ProxyLogTail() as tail:
        yield tail


@pytest_asyncio.fixture
async def proxy_log(_session_proxy_tail):
    """Per-test view over the session-scoped tail. Only surfaces events that
    appear AFTER the test starts, so negative assertions stay meaningful."""
    import asyncio
    view = _session_proxy_tail.view_from_now()
    yield view
    await asyncio.sleep(0.5)


def pytest_configure(config):
    config.addinivalue_line("markers", "dogfood_tier_a: Tier A — happy path")
    config.addinivalue_line("markers", "dogfood_tier_b: Tier B — adversarial")
    config.addinivalue_line("markers", "dogfood_tier_c: Tier C — AssistantStatus state coverage")
    config.addinivalue_line("markers", "dogfood_tier_d: Tier D — termination paths")
    config.addinivalue_line("markers", "dogfood_full: full 27-scenario arc")
    config.addinivalue_line(
        "markers",
        "serial_attachments: scenario uploads files; pinned to serial execution "
        "so multi-bubble replies are not misattributed across scenarios",
    )


def pytest_collection_modifyitems(config, items):
    """Keep file-attaching scenarios from interleaving (ZONE 6b).

    Within a single session loop the dogfood tests already run one at a time, so
    the per-scenario `bot.reset_chat()` drain is the primary guard. Under
    pytest-xdist, additionally pin every `serial_attachments` scenario to one
    worker so a late multi-bubble reply cannot be misattributed across workers.
    The xdist_group marker is only applied when xdist is loaded, so it does not
    trip --strict-markers in the normal single-process run.
    """
    if not config.pluginmanager.hasplugin("xdist"):
        return
    for item in items:
        if item.get_closest_marker("serial_attachments"):
            item.add_marker(pytest.mark.xdist_group("serial_attachments"))
