#!/usr/bin/env python3
"""End-to-end tests for client-server communication with tagged enums.

This tests the complete round-trip of data with discriminated unions
to ensure the client and server can communicate properly.
"""

import asyncio
import pytest
import pytest_asyncio
import sys
import json
from pathlib import Path
from datetime import datetime

# E2E gating handled centrally in tests/conftest.py


from generated import (
    AsyncClient,
    MyapiModelInputPet as Pet,
    MyapiModelOutputPet as PetDetails,
    MyapiModelKind as PetKind,
    MyapiModelKindDog as PetKindDog,
    MyapiModelKindCat as PetKindCat,
    MyapiModelBehavior as Behavior,
    MyapiModelBehaviorFactory as BehaviorFactory,
    MyapiModelInputPet as PetsCreateRequest,  # Create uses input model
    MyapiProtoPetsUpdateRequest as PetsUpdateRequest,
    MyapiProtoPetsRemoveRequest as PetsRemoveRequest,
    MyapiProtoPetsListRequest as PetsListRequest,
    MyapiProtoHeaders as Headers,
)
from reflectapi_runtime import ReflectapiOption, ApiError


@pytest.mark.asyncio
class TestClientServerIntegration:
    """Test client-server integration with tagged enums.

    Note: These tests require the demo server to be running.
    They are marked as integration tests and may be skipped in CI.
    """

    @pytest_asyncio.fixture
    async def client(self):
        """Create an async client for testing."""
        client = AsyncClient("http://localhost:3000")
        yield client
        await client.aclose()

    @pytest.fixture
    def auth_headers(self):
        """Create auth headers for requests."""
        return Headers(authorization="Bearer test-token")

    @pytest.mark.integration
    async def test_health_check(self, client):
        """Test basic health check endpoint."""
        response = await client.health.check()
        assert response.metadata.status_code == 200

    @pytest.mark.integration
    async def test_pets_list_empty(self, client):
        """Test listing pets when empty."""

        request = PetsListRequest(limit=10)
        response = await client.pets.list(
            limit=10, headers=Headers(authorization="Bearer test-token")
        )

        assert response.metadata.status_code == 200
        assert hasattr(response.value, "items")
        assert isinstance(response.value.items, list)

    @pytest.mark.integration
    async def test_create_pet_with_dog_kind(self, client, auth_headers):
        """Test creating a pet with dog kind (tagged enum)."""

        # Create a dog variant
        dog_kind = PetKindDog(type="dog", breed="Golden Retriever")
        pet = Pet(
            name=f"test_dog_{datetime.now().timestamp()}",
            kind=dog_kind,
            age=3,
            behaviors=[BehaviorFactory.CALM],
        )

        # Send create request
        response = await client.pets.create(data=pet, headers=auth_headers)

        # Should succeed or give meaningful error
        if response.metadata.status_code == 400:
            # This is the bug we fixed - should not happen with proper discriminated unions
            pytest.fail(
                "Got 400 Bad Request - discriminated union serialization may be broken"
            )
        else:
            assert response.metadata.status_code in [
                200,
                201,
                409,
            ]  # Created or conflict

    @pytest.mark.integration
    async def test_create_pet_with_cat_kind(self, client, auth_headers):
        """Test creating a pet with cat kind (tagged enum)."""
        # Create a cat variant
        cat_kind = PetKindCat(type="cat", lives=9)
        pet = Pet(
            name=f"test_cat_{datetime.now().timestamp()}",
            kind=cat_kind,
            age=2,
            behaviors=[BehaviorFactory.CALM, BehaviorFactory.other("Custom")],
        )

        # Send create request
        response = await client.pets.create(data=pet, headers=auth_headers)

        # Should succeed or give meaningful error
        if response.metadata.status_code == 400:
            # This is the bug we fixed - should not happen with proper discriminated unions
            pytest.fail(
                "Got 400 Bad Request - discriminated union serialization may be broken"
            )
        else:
            assert response.metadata.status_code in [
                200,
                201,
                409,
            ]  # Created or conflict

    @pytest.mark.integration
    async def test_update_pet_with_kind_change(self, client, auth_headers):
        """Test updating a pet with kind change (tagged enum)."""
        # Try to update a pet's kind
        new_kind = PetKindDog(type="dog", breed="Labrador")
        request = PetsUpdateRequest(
            name="existing_pet",  # Assume this exists or will fail gracefully
            kind=new_kind,
            age=ReflectapiOption(4),
        )

        try:
            response = await client.pets.update(data=request, headers=auth_headers)
            # If we get a response, it should be successful
            assert response.metadata.status_code == 200
        except ApiError as e:
            # Should be 404 (not found), but NOT 400 (bad request)
            if e.status_code == 400:
                pytest.fail(
                    "Got 400 Bad Request - discriminated union serialization may be broken"
                )
            elif e.status_code == 404:
                # This is expected for non-existent pet - test passes
                pass
            else:
                # Unexpected error
                pytest.fail(f"Unexpected error: {e.status_code} {e.message}")

    @pytest.mark.integration
    async def test_get_first_pet_with_tagged_enum(self, client, auth_headers):
        """Test getting first pet and validate tagged enum deserialization."""

        response = await client.pets.get_first(headers=auth_headers)

        if response.metadata.status_code == 200:
            # If we get a pet back, validate the kind structure
            if response.value is not None and hasattr(response.value, "kind"):
                kind = response.value.kind

                # Should have discriminator field
                assert hasattr(kind, "type")
                assert kind.type in ["dog", "cat"]

                # Should have appropriate variant fields
                if kind.type == "dog":
                    assert hasattr(kind, "breed")
                    assert isinstance(kind.breed, str)
                    assert not hasattr(kind, "lives")
                elif kind.type == "cat":
                    assert hasattr(kind, "lives")
                    assert isinstance(kind.lives, int)
                    assert not hasattr(kind, "breed")


class TestSerializationRoundTrip:
    """Test serialization round-trip without server interaction."""

    def test_dog_serialization_round_trip(self):
        """Test dog kind serialization and deserialization."""
        # Create original dog
        original_dog = PetKindDog(type="dog", breed="German Shepherd")
        original_pet = Pet(
            name="Rex", kind=original_dog, age=5, behaviors=[BehaviorFactory.CALM]
        )

        # Serialize to dict (like JSON)
        serialized = original_pet.model_dump()

        # Deserialize back
        deserialized_pet = Pet.model_validate(serialized)

        # Should be equivalent
        assert deserialized_pet.name == original_pet.name
        assert deserialized_pet.kind.type == original_pet.kind.type
        assert deserialized_pet.kind.breed == original_pet.kind.breed
        assert deserialized_pet.age == original_pet.age
        assert deserialized_pet.behaviors == original_pet.behaviors

    def test_cat_serialization_round_trip(self):
        """Test cat kind serialization and deserialization."""
        # Create original cat
        original_cat = PetKindCat(type="cat", lives=8)
        original_pet = Pet(
            name="Whiskers",
            kind=original_cat,
            age=3,
            behaviors=[BehaviorFactory.other("Custom")],
        )

        # Serialize to dict (like JSON)
        serialized = original_pet.model_dump()

        # Deserialize back
        deserialized_pet = Pet.model_validate(serialized)

        # Should be equivalent
        assert deserialized_pet.name == original_pet.name
        assert deserialized_pet.kind.type == original_pet.kind.type
        assert deserialized_pet.kind.lives == original_pet.kind.lives
        assert deserialized_pet.age == original_pet.age
        assert deserialized_pet.behaviors == original_pet.behaviors

    def test_mixed_pets_list_serialization(self):
        """Test serializing a list with mixed pet kinds."""
        now = datetime.now()

        dog = PetKindDog(type="dog", breed="Bulldog")
        cat = PetKindCat(type="cat", lives=9)

        pets = [
            PetDetails(name="Bruno", kind=dog, updated_at=now, age=4),
            PetDetails(name="Luna", kind=cat, updated_at=now, age=2),
        ]

        # Serialize each pet
        serialized_pets = [pet.model_dump() for pet in pets]

        # Deserialize back
        deserialized_pets = [
            PetDetails.model_validate(data) for data in serialized_pets
        ]

        # Validate first pet (dog)
        assert deserialized_pets[0].name == "Bruno"
        assert deserialized_pets[0].kind.type == "dog"
        assert deserialized_pets[0].kind.breed == "Bulldog"

        # Validate second pet (cat)
        assert deserialized_pets[1].name == "Luna"
        assert deserialized_pets[1].kind.type == "cat"
        assert deserialized_pets[1].kind.lives == 9

    def test_json_compatibility(self):
        """Test that tagged enums work with JSON serialization."""
        dog = PetKindDog(type="dog", breed="Poodle")
        pet = Pet(name="Buddy", kind=dog, age=6)

        # Convert to dict then JSON
        pet_dict = pet.model_dump()
        json_str = json.dumps(
            pet_dict, default=str
        )  # default=str for datetime handling

        # Parse back from JSON
        parsed_dict = json.loads(json_str)
        reconstructed_pet = Pet.model_validate(parsed_dict)

        # Should match original
        assert reconstructed_pet.name == pet.name
        assert reconstructed_pet.kind.type == pet.kind.type
        assert reconstructed_pet.kind.breed == pet.kind.breed
        assert reconstructed_pet.age == pet.age


class TestEdgeCases:
    """Test edge cases and error conditions."""

    def test_invalid_discriminator_in_json(self):
        """Test handling invalid discriminator in JSON data."""
        invalid_data = {
            "name": "Invalid Pet",
            "kind": {
                "type": "bird",  # Invalid discriminator
                "wingspan": 30,
            },
            "age": 2,
        }

        with pytest.raises(Exception):  # Should raise validation error
            Pet.model_validate(invalid_data)

    def test_missing_discriminator_field(self):
        """Test handling missing discriminator field."""
        invalid_data = {
            "name": "Invalid Pet",
            "kind": {
                # Missing "type" field
                "breed": "Unknown"
            },
            "age": 2,
        }

        with pytest.raises(Exception):  # Should raise validation error
            Pet.model_validate(invalid_data)

    def test_wrong_variant_fields(self):
        """Test validation when variant has wrong fields."""
        invalid_data = {
            "name": "Invalid Pet",
            "kind": {
                "type": "dog",
                "lives": 9,  # Cat field in dog variant
                # Missing breed field
            },
            "age": 2,
        }

        with pytest.raises(Exception):  # Should raise validation error
            Pet.model_validate(invalid_data)


@pytest.mark.asyncio
async def test_async_functionality():
    """Test that async client methods work correctly."""
    # This is a basic test that doesn't require server
    client = AsyncClient("http://example.com")

    # Client should be created successfully
    assert client is not None
    assert hasattr(client, "pets")
    assert hasattr(client, "health")

    # Clean up
    await client.aclose()
