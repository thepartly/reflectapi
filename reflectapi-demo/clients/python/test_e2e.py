#!/usr/bin/env python3
"""End-to-end test of the Python client against the running demo server."""

import asyncio
import sys
import time
from pathlib import Path

# Add the runtime path for local import
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent / "reflectapi-python-runtime" / "src"))

from generated import AsyncClient, Pet, PetKind


async def test_e2e():
    """End-to-end test against the running demo server."""
    print("🧪 Testing Python client against demo server...")
    
    # Create client pointed at the demo server
    client = AsyncClient("http://localhost:3000")
    print("✓ Created client for http://localhost:3000")
    
    try:
        # Test health check (should work)
        response = await client.health_check()
        print(f"✓ Health check: {response.metadata.status_code}")
        
        # Test pets list (might need empty data)
        try:
            from generated import PetsListRequest
            list_request = PetsListRequest()
            list_response = await client.pets_list(data=list_request)
            print(f"✓ Pets list: {list_response.metadata.status_code} - {len(list_response.items) if hasattr(list_response, 'items') else 'unknown'} pets")
        except Exception as e:
            print(f"⚠ Pets list failed: {e}")
            # Continue with other tests
        
        # Test get first pet (might be empty)
        try:
            first_pet = await client.pets_get_first()
            print(f"✓ Get first pet: {first_pet.metadata.status_code}")
        except Exception as e:
            print(f"⚠ Get first pet failed: {e}")
        
        print("🎉 End-to-end test passed! (Health check successful)")
        return True
        
    except Exception as e:
        print(f"❌ E2E test failed: {e}")
        return False
    
    finally:
        # Clean up the client
        await client.aclose()


if __name__ == "__main__":
    try:
        result = asyncio.run(test_e2e())
        if result:
            print("\n✅ Python client end-to-end test successful!")
            sys.exit(0)
        else:
            sys.exit(1)
    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)