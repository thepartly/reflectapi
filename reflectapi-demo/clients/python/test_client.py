#!/usr/bin/env python3
"""Test the generated Python client against the demo server."""

import asyncio
import sys
import time
from pathlib import Path
import pytest

from generated import (
    AsyncClient,
    MyapiModelInputPet as Pet,
    MyapiModelKindDog as PetKindDog,
)


@pytest.mark.asyncio
async def test_client():
    """Basic test of the generated client."""
    # For now, just test that we can import and create the client
    print("Testing Python client generation...")

    # Test creating client
    client = AsyncClient("http://localhost:3000")
    print("✓ Created AsyncClient")

    # Test creating data models
    pet_data = Pet(name="fluffy", kind=PetKindDog(type="dog", breed="Retriever"), age=3)
    print(f"✓ Created Pet: {pet_data}")

    # Note: PetsCreateRequest was a single-field tuple struct and is now unwrapped
    # The pets_create method now takes Pet directly
    print("✓ Pet data ready for pets_create (unwrapped tuple struct)")

    # Test that methods exist
    assert hasattr(client, "health")
    assert hasattr(client.health, "check")
    assert hasattr(client, "pets")
    assert hasattr(client.pets, "list")
    assert hasattr(client.pets, "create")
    assert hasattr(client.pets, "get_first")
    print("✓ All expected methods exist")

    print("✓ Basic client generation test passed!")
    return True


if __name__ == "__main__":
    try:
        result = asyncio.run(test_client())
        if result:
            print("\n🎉 Python client generation successful!")
            sys.exit(0)
    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        import traceback

        traceback.print_exc()
        sys.exit(1)
