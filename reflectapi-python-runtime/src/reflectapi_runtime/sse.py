"""Minimal Server-Sent Events parser.

Parses the SSE wire format described in
https://html.spec.whatwg.org/multipage/server-sent-events.html

Only the subset emitted by the reflectapi server is required:
events consist of one or more ``data:`` lines terminated by a blank
line. Other fields (``event:``, ``id:``, ``retry:``) are tracked but
not exposed, so consumers can still receive heartbeats or named events
without breaking parsing.
"""

from __future__ import annotations

from collections.abc import AsyncIterable, AsyncIterator, Iterable, Iterator
from dataclasses import dataclass


@dataclass(frozen=True)
class SseEvent:
    """A dispatched server-sent event."""

    data: str
    event: str = "message"
    id: str | None = None
    retry: int | None = None


class _SseAccumulator:
    """Stateful line-by-line SSE parser.

    Feed lines via :meth:`feed` and read out completed events via
    :meth:`drain`. ``feed_eof`` flushes any pending event when the
    upstream connection closes.
    """

    def __init__(self) -> None:
        self._data: list[str] = []
        self._event: str | None = None
        self._id: str | None = None
        self._retry: int | None = None
        self._pending: list[SseEvent] = []

    def feed(self, line: str) -> None:
        # The HTTP transport hands us lines without their trailing
        # newline. Be defensive about a stray CR — some transports keep
        # CRLF intact. A blank line marks event dispatch.
        if line.endswith("\r"):
            line = line[:-1]
        if line == "":
            self._dispatch()
            return
        # Lines starting with ":" are comments / heartbeats.
        if line.startswith(":"):
            return

        if ":" in line:
            field, _, value = line.partition(":")
            if value.startswith(" "):
                value = value[1:]
        else:
            field, value = line, ""

        if field == "data":
            self._data.append(value)
        elif field == "event":
            self._event = value
        elif field == "id":
            # The spec says NULs invalidate the id; we just drop it.
            if "\x00" not in value:
                self._id = value
        elif field == "retry":
            try:
                self._retry = int(value)
            except ValueError:
                pass
        # Unknown fields are ignored per the spec.

    def feed_eof(self) -> None:
        # The spec says a trailing event without a blank line is
        # discarded, but in practice servers occasionally close without
        # a final blank, so we flush anything still pending.
        if self._data:
            self._dispatch()

    def drain(self) -> list[SseEvent]:
        out, self._pending = self._pending, []
        return out

    def _dispatch(self) -> None:
        if not self._data:
            # Empty event (e.g. a stray blank line) — reset name only.
            self._event = None
            return
        data = "\n".join(self._data)
        self._pending.append(
            SseEvent(
                data=data,
                event=self._event or "message",
                id=self._id,
                retry=self._retry,
            )
        )
        self._data = []
        self._event = None
        # id and retry persist across events per the spec.


def parse_sse(lines: Iterable[str]) -> Iterator[SseEvent]:
    """Parse SSE events from a synchronous iterable of lines."""
    parser = _SseAccumulator()
    for line in lines:
        parser.feed(line)
        for event in parser.drain():
            yield event
    parser.feed_eof()
    yield from parser.drain()


async def aparse_sse(lines: AsyncIterable[str]) -> AsyncIterator[SseEvent]:
    """Parse SSE events from an async iterable of lines."""
    parser = _SseAccumulator()
    async for line in lines:
        parser.feed(line)
        for event in parser.drain():
            yield event
    parser.feed_eof()
    for event in parser.drain():
        yield event
