"""End-to-end test of generated SSE streaming methods.

Uses ``httpx.MockTransport`` so the test does not need a running server,
but exercises the *generated* client classes (sync + async) to make sure
the codegen wiring is correct end-to-end.
"""

from __future__ import annotations

import json

import httpx
import pytest

from tests.package_imports import AsyncClient, Client, MyapiModelOutputPet as Pet
from reflectapi_runtime import ApplicationError


def _sse(items: list[dict]) -> bytes:
    return ("".join(f"data: {json.dumps(it)}\n\n" for it in items)).encode()


@pytest.fixture
def pet_payloads() -> list[dict]:
    # Pet on the wire from the demo schema: {"name": str, "kind": ..., "behaviors": [...]}.
    # Use minimal values that pass Pydantic validation.
    return [
        {
            "name": "fido",
            "kind": {"type": "dog", "breed": "lab"},
            "behaviors": [],
            "updated_at": "2026-01-01T00:00:00Z",
        },
        {
            "name": "whiskers",
            "kind": {"type": "cat", "lives": 9},
            "behaviors": [],
            "updated_at": "2026-01-02T00:00:00Z",
        },
    ]


def _async_client(handler) -> AsyncClient:
    return AsyncClient(
        "http://test",
        client=httpx.AsyncClient(transport=httpx.MockTransport(handler)),
    )


def _sync_client(handler) -> Client:
    return Client(
        "http://test",
        client=httpx.Client(transport=httpx.MockTransport(handler)),
    )


@pytest.mark.asyncio
async def test_async_cdc_events_stream(pet_payloads):
    captured: dict[str, str] = {}

    def handler(request: httpx.Request) -> httpx.Response:
        captured["accept"] = request.headers.get("accept", "")
        captured["path"] = request.url.path
        captured["method"] = request.method
        return httpx.Response(200, content=_sse(pet_payloads))

    client = _async_client(handler)
    received = [pet async for pet in client.pets.cdc_events()]

    assert captured["accept"] == "text/event-stream"
    assert captured["path"] == "/pets.cdc-events"
    assert captured["method"] == "POST"
    assert all(isinstance(p, Pet) for p in received)
    assert [p.name for p in received] == ["fido", "whiskers"]
    await client.aclose()


def test_sync_cdc_events_stream(pet_payloads):
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, content=_sse(pet_payloads))

    client = _sync_client(handler)
    received = list(client.pets.cdc_events())
    assert [p.name for p in received] == ["fido", "whiskers"]
    client.close()


@pytest.mark.asyncio
async def test_async_cdc_events_4xx_raises():
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(401, json={"reason": "denied"})

    client = _async_client(handler)
    with pytest.raises(ApplicationError) as ei:
        async for _ in client.pets.cdc_events():
            break
    assert ei.value.status_code == 401
    await client.aclose()


@pytest.mark.asyncio
async def test_async_cdc_events_partial_consumption_closes_response():
    pets_lots = [
        {
            "name": f"p{i}",
            "kind": {"type": "dog", "breed": "x"},
            "behaviors": [],
            "updated_at": "2026-01-01T00:00:00Z",
        }
        for i in range(50)
    ]

    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, content=_sse(pets_lots))

    client = _async_client(handler)
    stream = client.pets.cdc_events()
    first = await stream.__anext__()
    assert first.name == "p0"
    # closing the generator triggers the runtime's `finally: response.aclose()`.
    await stream.aclose()
    await client.aclose()
