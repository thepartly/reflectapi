"""Tests for the minimal SSE parser and the streaming request flow."""

from __future__ import annotations

import asyncio
import json
import os
import sys
from typing import Any

import httpx
import pytest
from pydantic import BaseModel

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

from reflectapi_runtime.client import AsyncClientBase, ClientBase
from reflectapi_runtime.exceptions import ApplicationError
from reflectapi_runtime.sse import SseEvent, aparse_sse, parse_sse


# ---------------------------------------------------------------------------
# Pure parser
# ---------------------------------------------------------------------------


def _lines(blob: str) -> list[str]:
    # httpx.iter_lines splits on \n and strips the trailing newline, so do
    # the same for parser inputs here.
    if blob.endswith("\n"):
        blob = blob[:-1]
    return blob.split("\n")


def test_parse_single_event() -> None:
    events = list(parse_sse(_lines("data: hello\n\n")))
    assert events == [SseEvent(data="hello")]


def test_parse_multiline_data_joined_with_newline() -> None:
    events = list(parse_sse(_lines("data: line1\ndata: line2\n\n")))
    assert events == [SseEvent(data="line1\nline2")]


def test_parse_multiple_events() -> None:
    payload = "data: a\n\ndata: b\n\ndata: c\n\n"
    assert [e.data for e in parse_sse(_lines(payload))] == ["a", "b", "c"]


def test_parse_event_name_and_id_persist() -> None:
    payload = (
        "event: ping\n"
        "id: 1\n"
        "data: 1\n"
        "\n"
        "data: 2\n"
        "\n"
    )
    events = list(parse_sse(_lines(payload)))
    assert events[0] == SseEvent(data="1", event="ping", id="1")
    # event name resets per spec; id persists.
    assert events[1] == SseEvent(data="2", event="message", id="1")


def test_parse_ignores_comments_and_unknown_fields() -> None:
    payload = ": heartbeat\nfoo: bar\ndata: kept\n\n"
    events = list(parse_sse(_lines(payload)))
    assert events == [SseEvent(data="kept")]


def test_parse_field_without_colon_treated_as_empty_value() -> None:
    events = list(parse_sse(_lines("data\n\n")))
    assert events == [SseEvent(data="")]


def test_parse_flushes_trailing_event_on_eof() -> None:
    # Server closes without a final blank line.
    events = list(parse_sse(_lines("data: tail")))
    assert events == [SseEvent(data="tail")]


def test_parse_handles_crlf_line_endings() -> None:
    # iter_lines normally strips the \r, but be defensive in case it doesn't.
    events = list(parse_sse(["data: ok\r", "\r"]))
    assert events == [SseEvent(data="ok")]


@pytest.mark.asyncio
async def test_aparse_yields_events_lazily() -> None:
    async def gen():
        for line in _lines("data: 1\n\ndata: 2\n\n"):
            yield line

    events = [e async for e in aparse_sse(gen())]
    assert [e.data for e in events] == ["1", "2"]


# ---------------------------------------------------------------------------
# Full streaming flow against an in-process httpx MockTransport
# ---------------------------------------------------------------------------


class _Pet(BaseModel):
    name: str
    weight: float


class _SyncDemo(ClientBase):
    """Minimal concrete subclass — exposes _make_sse_request as a public method."""

    def stream_pets(self) -> Any:
        return self._make_sse_request(
            "POST", "pets", item_model=_Pet, error_model=None
        )


class _AsyncDemo(AsyncClientBase):
    def stream_pets(self) -> Any:
        return self._make_sse_request(
            "POST", "pets", item_model=_Pet, error_model=None
        )


def _sse_body(items: list[dict]) -> bytes:
    return ("".join(f"data: {json.dumps(it)}\n\n" for it in items)).encode()


def _make_sync_client(transport: httpx.MockTransport) -> _SyncDemo:
    client = _SyncDemo(
        base_url="http://test", client=httpx.Client(transport=transport)
    )
    return client


def _make_async_client(transport: httpx.MockTransport) -> _AsyncDemo:
    return _AsyncDemo(
        base_url="http://test", client=httpx.AsyncClient(transport=transport)
    )


def test_sync_streaming_yields_validated_models() -> None:
    pets = [{"name": "fido", "weight": 1.5}, {"name": "rex", "weight": 4.0}]

    def handler(request: httpx.Request) -> httpx.Response:
        assert request.headers.get("accept") == "text/event-stream"
        return httpx.Response(200, content=_sse_body(pets))

    client = _make_sync_client(httpx.MockTransport(handler))
    received = list(client.stream_pets())
    assert received == [_Pet(**p) for p in pets]
    client.close()


def test_sync_streaming_raises_application_error_on_4xx() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(401, json={"reason": "denied"})

    client = _make_sync_client(httpx.MockTransport(handler))
    with pytest.raises(ApplicationError) as ei:
        list(client.stream_pets())
    assert ei.value.status_code == 401
    client.close()


def test_sync_streaming_releases_connection_on_early_break() -> None:
    pets = [{"name": "fido", "weight": 1.5}, {"name": "rex", "weight": 4.0}]
    closed: list[bool] = []

    real_response_cls = httpx.Response

    class _TrackingResponse(real_response_cls):  # type: ignore[misc]
        def close(self) -> None:  # pragma: no cover - exercised below
            closed.append(True)
            super().close()

    def handler(request: httpx.Request) -> httpx.Response:
        return _TrackingResponse(200, content=_sse_body(pets))

    client = _make_sync_client(httpx.MockTransport(handler))
    stream = client.stream_pets()
    first = next(stream)
    assert first == _Pet(**pets[0])
    stream.close()  # generator close → finally → response.close()
    assert closed, "response should be closed after generator close()"
    client.close()


@pytest.mark.asyncio
async def test_async_streaming_yields_validated_models() -> None:
    pets = [{"name": "fido", "weight": 1.5}, {"name": "rex", "weight": 4.0}]

    def handler(request: httpx.Request) -> httpx.Response:
        assert request.headers.get("accept") == "text/event-stream"
        return httpx.Response(200, content=_sse_body(pets))

    client = _make_async_client(httpx.MockTransport(handler))
    received = [pet async for pet in client.stream_pets()]
    assert received == [_Pet(**p) for p in pets]
    await client.aclose()


@pytest.mark.asyncio
async def test_async_streaming_raises_application_error_on_4xx() -> None:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(401, json={"reason": "denied"})

    client = _make_async_client(httpx.MockTransport(handler))
    with pytest.raises(ApplicationError) as ei:
        async for _ in client.stream_pets():  # noqa: F841
            break
    assert ei.value.status_code == 401
    await client.aclose()


@pytest.mark.asyncio
async def test_async_streaming_supports_cancellation() -> None:
    pets = [{"name": f"p{i}", "weight": float(i)} for i in range(100)]

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, content=_sse_body(pets))

    client = _make_async_client(httpx.MockTransport(handler))
    stream = client.stream_pets()
    first = await stream.__anext__()
    assert first == _Pet(**pets[0])
    # Closing the async generator should clean up without raising.
    await stream.aclose()
    await client.aclose()


def test_sync_streaming_validation_error_surfaces() -> None:
    bad = [{"name": "fido"}]  # missing required weight

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, content=_sse_body(bad))

    from reflectapi_runtime.exceptions import ValidationError

    client = _make_sync_client(httpx.MockTransport(handler))
    with pytest.raises(ValidationError):
        list(client.stream_pets())
    client.close()


@pytest.mark.asyncio
async def test_async_streaming_unknown_event_terminates_stream_after_prior_items() -> None:
    """A malformed/unknown event raises ValidationError mid-stream.

    This documents the strict-validation contract: clients cannot silently
    skip events the server has added but the client doesn't yet know about.
    Items received before the bad event are still observed.
    """
    from reflectapi_runtime.exceptions import ValidationError

    payloads = [
        {"name": "fido", "weight": 1.5},
        {"name": "broken"},  # missing required `weight`
        {"name": "rex", "weight": 4.0},  # never reached
    ]

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, content=_sse_body(payloads))

    client = _make_async_client(httpx.MockTransport(handler))
    received: list[str] = []
    with pytest.raises(ValidationError):
        async for pet in client.stream_pets():
            received.append(pet.name)
    assert received == ["fido"]
    await client.aclose()


# ---------------------------------------------------------------------------
# Test harness wiring (asyncio mode)
# ---------------------------------------------------------------------------


@pytest.fixture(scope="session")
def event_loop():  # pragma: no cover - pytest-asyncio compatibility shim
    loop = asyncio.new_event_loop()
    yield loop
    loop.close()
