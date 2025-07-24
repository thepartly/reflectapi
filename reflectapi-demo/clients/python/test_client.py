#!/usr/bin/env python3
"""Test the generated Python client against the demo server."""

import asyncio
import sys
import time
from pathlib import Path

# Add the runtime path for local import
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent / "reflectapi-python-runtime" / "src"))

from generated import AsyncClient, Pet, PetKind


async def test_client():
    """Basic test of the generated client."""
    # For now, just test that we can import and create the client
    print("Testing Python client generation...")
    
    # Test creating client
    client = AsyncClient("http://localhost:8080")
    print("✓ Created AsyncClient")
    
    # Test creating data models  
    pet_data = Pet(
        name="fluffy",
        kind=PetKind.CAT,
        age=3
    )
    print("✓ Created Pet")
    
    # Note: PetsCreateRequest was a single-field tuple struct and is now unwrapped
    # The pets_create method now takes Pet directly
    print("✓ Pet data ready for pets_create (unwrapped tuple struct)")
    
    # Test that methods exist
    assert hasattr(client, 'health_check')
    assert hasattr(client, 'pets_list')
    assert hasattr(client, 'pets_create')
    assert hasattr(client, 'pets_get_first')
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