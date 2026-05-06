"""Real-network end-to-end SSE test against the demo server.

Run with:

    RUN_DEMO_E2E=1 uv run pytest test_streaming_e2e.py -p no:cacheprovider \\
        --override-ini=testpaths=.

Skipped unless ``RUN_DEMO_E2E=1`` is set.
"""

from __future__ import annotations

import asyncio
import os
import uuid

import pytest

if os.environ.get("RUN_DEMO_E2E") != "1":  # pragma: no cover
    pytest.skip("set RUN_DEMO_E2E=1 to run", allow_module_level=True)

from generated import (  # noqa: E402
    AsyncClient,
    MyapiModelInputPet as Pet,
    MyapiModelKindDog as Dog,
    MyapiProtoHeaders as Headers,
    MyapiProtoPetsRemoveRequest as RemoveRequest,
)


@pytest.mark.asyncio
async def test_cdc_events_receives_created_pet():
    """Subscribe to the CDC stream, create a pet, and confirm we observe it."""
    client = AsyncClient("http://localhost:3000")
    headers = Headers(authorization="Bearer test")
    pet_name = f"sse-test-{uuid.uuid4().hex[:8]}"

    received: list[str] = []
    cancel = asyncio.Event()

    async def consumer() -> None:
        async for pet in client.pets.cdc_events(headers=headers):
            received.append(pet.name)
            if cancel.is_set() or pet.name == pet_name:
                break

    task = asyncio.create_task(consumer())

    # Give the subscriber a moment to register before publishing.
    await asyncio.sleep(0.2)

    create = await client.pets.create(
        data=Pet(name=pet_name, kind=Dog(type="dog", breed="lab")),
        headers=headers,
    )
    assert create.metadata.status_code == 200

    try:
        await asyncio.wait_for(task, timeout=5.0)
    except asyncio.TimeoutError:
        cancel.set()
        task.cancel()
        raise
    finally:
        # Cleanup: remove the test pet.
        try:
            await client.pets.remove(
                data=RemoveRequest(name=pet_name), headers=headers
            )
        except Exception:
            pass
        await client.aclose()

    assert pet_name in received
