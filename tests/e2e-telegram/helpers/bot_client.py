"""Thin wrapper over a Telethon client that sends to the clawbot under test
(handle configured via BOT_HANDLE in .env.test) and waits for its reply.
Every test message is prefixed with `[TEST]` so the real Telegram chat stays
legible and filterable.

Class name `BotClient` is historical — the product identity is the
OpenClaw Clawbot, but any Telegram bot handle can be plugged in via
config. Error messages use the actual handle dynamically so test
output reflects reality.

Includes a per-session send-count budget. The Telegram account this harness
uses is shared across multiple projects (~50 usages/day soft cap on the
account); we hard-stop within the harness to leave headroom for the user's
other projects. Budget is configurable via TELEGRAM_DAILY_SEND_BUDGET in
.env.test (default 35).
"""
from __future__ import annotations

import asyncio
import os
import time
from dataclasses import dataclass

from telethon import TelegramClient, events


class SendBudgetExceeded(RuntimeError):
    pass


@dataclass
class BotReply:
    text: str
    received_at: float
    latency_s: float


class BotClient:
    """One instance per test session. Bound to a fixed bot handle."""

    def __init__(self, telegram_client: TelegramClient, bot_handle: str) -> None:
        self.client = telegram_client
        self.bot_handle = bot_handle
        self._bot_entity = None
        self.send_count: int = 0
        # Conservative default of 35: full suite is ~30 sends, leaves headroom
        # for one retry without blowing the shared 50/day account budget.
        self.daily_send_budget: int = int(os.environ.get("TELEGRAM_DAILY_SEND_BUDGET", "35"))

    async def _resolve_bot(self):
        if self._bot_entity is None:
            self._bot_entity = await self.client.get_entity(self.bot_handle)
        return self._bot_entity

    async def send_and_wait(
        self,
        message: str,
        *,
        timeout: float = 60.0,
        prefix: str = "[TEST] ",
        settle_ms: int = 500,
    ) -> BotReply:
        """Send a message to bot and return its reply.

        timeout: overall deadline. bot may take 2-10s to respond for simple
            questions, longer if it does multi-tool reasoning. Default 60s
            is safe for Haiku at default Claude speeds.
        prefix: prepended to the message. `[TEST] ` by default so chat history
            remains filterable. Set to "" to send a pristine message (useful
            for tests that need to simulate non-test traffic).
        settle_ms: after the first reply arrives, wait this long for follow-up
            messages (bot sometimes sends a second bubble with continuation).
            If another message arrives in that window, it's concatenated.
        """
        if self.send_count >= self.daily_send_budget:
            raise SendBudgetExceeded(
                f"Send budget exhausted ({self.send_count}/{self.daily_send_budget}). "
                f"Stop here to leave headroom on the shared Telegram account. "
                f"Raise TELEGRAM_DAILY_SEND_BUDGET in .env.test if you want to continue."
            )

        bot = await self._resolve_bot()
        full = f"{prefix}{message}" if prefix else message

        received: list[tuple[str, float]] = []
        fut: asyncio.Future = asyncio.Future()

        @self.client.on(events.NewMessage(from_users=bot))
        async def _handler(event):  # noqa: ARG001
            received.append((event.message.message or "", time.time()))
            if not fut.done():
                fut.set_result(None)

        sent_at = time.time()
        await self.client.send_message(bot, full)
        self.send_count += 1
        try:
            await asyncio.wait_for(fut, timeout=timeout)
        except asyncio.TimeoutError:
            self.client.remove_event_handler(_handler)
            raise TimeoutError(
                f"{self.bot_handle} did not reply within {timeout}s to: {full!r}. "
                f"If this is the second+ message from an unpaired user, silence is "
                f"expected — OpenClaw's pairing gate blocks further replies until "
                f"`openclaw pairing approve telegram <code>` is run on the host."
            ) from None

        # Settle window: collect continuation messages.
        await asyncio.sleep(settle_ms / 1000)
        self.client.remove_event_handler(_handler)

        combined = "\n".join(msg for msg, _ in received)
        first_received_at = received[0][1]
        return BotReply(
            text=combined,
            received_at=first_received_at,
            latency_s=first_received_at - sent_at,
        )

    async def send_many_collect(
        self,
        messages: list[str],
        *,
        per_msg_timeout: float = 60.0,
        between_s: float = 1.0,
    ) -> list[BotReply]:
        """Send a sequence of messages, return bot's reply to each in order.
        Applies rate-limit spacing between sends.
        """
        results: list[BotReply] = []
        for i, msg in enumerate(messages):
            if i > 0:
                await asyncio.sleep(between_s)
            results.append(await self.send_and_wait(msg, timeout=per_msg_timeout))
        return results
