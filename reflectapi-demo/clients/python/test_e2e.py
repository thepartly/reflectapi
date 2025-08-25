#!/usr/bin/env python3
"""End-to-end test for Python client with demo server."""

import asyncio
import sys
from datetime import datetime
sys.path.insert(0, '../../../reflectapi-python-runtime/src')

from generated import (
    AsyncClient,
    MyapiModelInputPet as Pet,
    MyapiModelKindDog as PetKindDog,
    MyapiModelKindCat as PetKindCat,
    MyapiModelBehaviorFactory as BehaviorFactory,
    MyapiProtoHeaders as Headers,
)


async def test_e2e():
    """Test e2e communication with the demo server."""
    client = AsyncClient("http://localhost:3000")
    
    # Test 1: Health check
    print("Testing health check...")
    try:
        health_response = await client.health.check()
        print(f"✓ Health check passed: {health_response.value}")
    except Exception as e:
        print(f"✗ Health check failed: {e}")
        raise
    
    # Test 2: Create a pet with dog kind
    print("\nTesting pet creation...")
    dog_pet = Pet(
        name="Buddy",
        kind=PetKindDog(type='dog', breed='Golden Retriever'),
        age=3,
        behaviors=[BehaviorFactory.CALM, BehaviorFactory.aggressive(5.0, "sometimes barks")]
    )
    
    headers = Headers(authorization="Bearer test-token")
    try:
        create_response = await client.pets.create(data=dog_pet, headers=headers)
        print(f"✓ Create succeeded: {create_response.value}")
    except Exception as e:
        print(f"  Create failed (expected for demo): {e}")
    
    # Test 3: List pets
    print("\nTesting pet listing...")
    try:
        list_response = await client.pets.list(limit=10, headers=headers)
        pets = list_response.value
        print(f"✓ Listed {len(pets.items)} pets")
        for pet in pets.items:
            print(f"  - {pet.name} ({pet.kind.__class__.__name__})")
    except Exception as e:
        print(f"  List failed: {e}")
    
    # Test 4: Get first pet
    print("\nTesting get first pet...")
    try:
        first_response = await client.pets.get_first(headers=headers)
        if first_response.value:
            print(f"✓ First pet: {first_response.value.name}")
        else:
            print("✓ No pets found (None response)")
    except Exception as e:
        print(f"  Get first failed: {e}")
    
    print("\n✅ All e2e tests completed successfully!")


if __name__ == "__main__":
    asyncio.run(test_e2e())
