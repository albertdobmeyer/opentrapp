"""Async tail of vault-proxy events. Parses the structured JSONL stream the
proxy writes and surfaces parsed events for test assertions.

Source: we tail the requests.jsonl file on the named-volume host path
(discovered at runtime via `podman volume inspect`). This is more reliable
than `podman logs -f`, which had observable delivery latency in practice
(events could land in `podman logs` but not reach our asyncio subprocess
reader in time for a 10s assertion window — see commit 74d642c postmortem).

Python's logging FileHandler flushes after each emit, so the file is a
real-time view of proxy activity. `tail -F` on the host path gives
line-at-a-time streaming with no buffering.
"""
from __future__ import annotations

import asyncio
import contextlib
import json
import os
import time
from dataclasses import dataclass, field


PROXY_LOGS_VOLUME = "opentrapp_vault-proxy-logs"
LOG_FILE_NAME = "requests.jsonl"


@dataclass
class ProxyEvent:
    action: str  # ALLOWED / BLOCKED / EXFIL_BLOCKED / RESPONSE / KEY_REFLECTED / LARGE_RESPONSE_BLOCKED
    url: str = ""
    method: str = ""
    host: str = ""
    status: int | None = None
    request_bytes: int = 0
    response_bytes: int = 0
    reason: str = ""
    timestamp: str = ""
    raw: dict = field(default_factory=dict)

    @classmethod
    def from_json(cls, obj: dict) -> "ProxyEvent":
        return cls(
            action=obj.get("action", ""),
            url=obj.get("url", ""),
            method=obj.get("method", ""),
            host=obj.get("host", ""),
            status=obj.get("status"),
            request_bytes=obj.get("request_bytes", 0),
            response_bytes=obj.get("response_bytes", 0),
            reason=obj.get("reason", ""),
            timestamp=obj.get("timestamp", ""),
            raw=obj,
        )


class ProxyLogTail:
    """Streams events from `podman logs -f vault-proxy` starting at attach time.

    Usage:
        async with ProxyLogTail() as tail:
            # ... do test work ...
            events = tail.events  # all ProxyEvents captured in the window
    """

    def __init__(self, container: str = "vault-proxy") -> None:
        self.container = container  # kept for API compatibility; unused in file-tail mode
        self.events: list[ProxyEvent] = []
        self._proc: asyncio.subprocess.Process | None = None
        self._reader_task: asyncio.Task | None = None
        self._started_at: float = 0.0
        self._log_file_path: str = ""

    async def _discover_log_path(self) -> str:
        """Find the path the proxy is actually writing to, INSIDE the container.

        The proxy writes to /var/log/vault-proxy/requests.jsonl when its log
        volume is writable, else falls back to /tmp/vault-proxy-requests.jsonl.
        On rootless podman the proxy-logs named volume is owned by
        container-root while mitmproxy runs as a non-root user, so the proxy
        always falls back to the in-container tmpfs (findings 2026-05-20). That
        tmpfs path is NOT host-visible, so we probe both in-container paths and
        tail whichever exists via `podman exec`.
        """
        candidates = [
            "/var/log/vault-proxy/requests.jsonl",
            "/tmp/vault-proxy-requests.jsonl",
        ]
        for path in candidates:
            proc = await asyncio.create_subprocess_exec(
                "podman", "exec", self.container, "test", "-f", path,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            await proc.wait()
            if proc.returncode == 0:
                return path
        raise RuntimeError(
            f"No proxy requests.jsonl found in {self.container} at {candidates}. "
            f"Is the perimeter running?"
        )

    async def __aenter__(self) -> "ProxyLogTail":
        self._log_file_path = await self._discover_log_path()
        # `tail -n 0 -F` — skip backlog, follow by name (survives log rotation).
        # Run INSIDE the container via `podman exec` because the proxy's actual
        # log file (often the /tmp fallback) lives on an in-container tmpfs that
        # is not visible on the host volume mountpoint.
        self._proc = await asyncio.create_subprocess_exec(
            "podman", "exec", self.container,
            "tail", "-n", "0", "-F", self._log_file_path,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.DEVNULL,
        )
        self._started_at = time.time()
        self._reader_task = asyncio.create_task(self._consume(self._proc.stdout))
        # Brief wait for the tail subprocess to establish before tests send.
        await asyncio.sleep(0.2)
        return self

    async def __aexit__(self, exc_type, exc, tb) -> None:
        if self._proc and self._proc.returncode is None:
            self._proc.terminate()
            with contextlib.suppress(asyncio.TimeoutError):
                await asyncio.wait_for(self._proc.wait(), timeout=2.0)
        if self._reader_task:
            self._reader_task.cancel()
            with contextlib.suppress(asyncio.CancelledError, Exception):
                await self._reader_task

    async def _consume(self, stream: asyncio.StreamReader | None) -> None:
        if stream is None:
            return
        # File contains one JSON object per line, nothing else. No prefix
        # stripping needed (unlike the podman-logs stdout which prepended
        # "[vault-proxy] " or mitmproxy's own "[timestamp] ").
        async for raw in stream:
            try:
                line = raw.decode("utf-8", errors="replace").strip()
            except Exception:
                continue
            if not line.startswith("{"):
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue
            if "action" not in obj:
                continue
            self.events.append(ProxyEvent.from_json(obj))

    def where(self, **filters) -> list[ProxyEvent]:
        """Filter events by any combination of fields. `url_contains=str` is
        supported as a convenience; all other keys are equality checks on the
        event field.
        """
        return _filter_events(self.events, **filters)

    async def wait_for(self, *, url_contains: str | None = None, action: str | None = None,
                       timeout: float = 15.0) -> ProxyEvent | None:
        """Poll for an event matching criteria. Returns None if timeout."""
        return await _wait_for(lambda: self.events, url_contains=url_contains,
                               action=action, timeout=timeout)

    def view_from_now(self) -> "ProxyLogView":
        """Return a per-test view over this session-scoped tail. The view only
        surfaces events appended AFTER this call, so negative assertions like
        "no BLOCKED events this test" remain meaningful across tests that run
        before it.
        """
        return ProxyLogView(self, marker=len(self.events))


def _filter_events(events: list[ProxyEvent], **filters) -> list[ProxyEvent]:
    out = []
    url_contains = filters.pop("url_contains", None)
    for ev in events:
        if url_contains and url_contains not in ev.url:
            continue
        if all(getattr(ev, k, None) == v for k, v in filters.items()):
            out.append(ev)
    return out


async def _wait_for(events_fn, *, url_contains=None, action=None, timeout=15.0):
    deadline = time.time() + timeout
    while time.time() < deadline:
        filters = {}
        if action is not None:
            filters["action"] = action
        matches = _filter_events(events_fn(), **filters)
        if url_contains:
            matches = [e for e in matches if url_contains in e.url]
        if matches:
            return matches[-1]
        await asyncio.sleep(0.25)
    return None


class ProxyLogView:
    """Per-test slice of a session-scoped ProxyLogTail. Events before the
    view's marker are hidden; events after it are live-updated as the
    underlying tail accumulates. Supports .events, .where(), .wait_for(),
    .clear() — and .clear() only clears the view's window, not session state.
    """

    def __init__(self, tail: "ProxyLogTail", marker: int) -> None:
        self._tail = tail
        self._marker = marker

    @property
    def events(self) -> list[ProxyEvent]:
        return self._tail.events[self._marker:]

    def where(self, **filters) -> list[ProxyEvent]:
        return _filter_events(self.events, **filters)

    async def wait_for(self, *, url_contains: str | None = None, action: str | None = None,
                       timeout: float = 15.0) -> ProxyEvent | None:
        return await _wait_for(lambda: self.events, url_contains=url_contains,
                               action=action, timeout=timeout)

    def clear(self) -> None:
        """Advance the marker to the current end, effectively emptying the
        view. Used by tests that compare before/after within a single test.
        """
        self._marker = len(self._tail.events)
