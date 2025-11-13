"""End-to-end tests for tagged enum (discriminated union) functionality."""

import pytest
from datetime import datetime
from typing import Union
from pydantic import ValidationError

from generated import (
    MyapiModelInputPet as Pet,
    MyapiModelOutputPet as PetDetails,
    MyapiModelKind as PetKind,
    MyapiModelKindDog as PetKindDog,
    MyapiModelKindCat as PetKindCat,
    MyapiModelBehavior as Behavior,
    MyapiModelBehaviorOtherVariant as BehaviorOther,
    MyapiProtoPetsUpdateRequest as PetsUpdateRequest,
    MyapiProtoPetsListRequest as PetsListRequest,
    MyapiProtoPetsRemoveRequest as PetsRemoveRequest,
    MyapiProtoHeaders as Headers,
    MyapiProtoPaginated as Paginated,
)
from reflectapi_runtime import ReflectapiOption

# For externally tagged enums, unit variants are just string literals
BehaviorCalm = "Calm"


class TestTaggedEnumCreation:
    """Test creation and validation of tagged enum variants."""

    def test_dog_variant_creation(self):
        """Test creating a dog variant with all required fields."""
        dog = PetKindDog(type="dog", breed="German Shepherd")
        assert dog.type == "dog"
        assert dog.breed == "German Shepherd"

    def test_cat_variant_creation(self):
        """Test creating a cat variant with all required fields."""
        cat = PetKindCat(type="cat", lives=9)
        assert cat.type == "cat"
        assert cat.lives == 9

    def test_dog_variant_validation(self):
        """Test validation of dog variant fields."""
        # Should fail with wrong discriminator
        with pytest.raises(ValidationError) as exc_info:
            PetKindDog(type="cat", breed="Husky")
        assert "Input should be 'dog'" in str(exc_info.value)

        # Should fail with missing breed
        with pytest.raises(ValidationError) as exc_info:
            PetKindDog(type="dog")
        assert "Field required" in str(exc_info.value)

    def test_cat_variant_validation(self):
        """Test validation of cat variant fields."""
        # Should fail with wrong discriminator
        with pytest.raises(ValidationError) as exc_info:
            PetKindCat(type="dog", lives=7)
        assert "Input should be 'cat'" in str(exc_info.value)

        # Should fail with missing lives
        with pytest.raises(ValidationError) as exc_info:
            PetKindCat(type="cat")
        assert "Field required" in str(exc_info.value)


class TestTaggedEnumSerialization:
    """Test serialization and deserialization of tagged enums."""

    def test_dog_serialization(self):
        """Test serializing a dog variant."""
        dog = PetKindDog(type="dog", breed="Labrador")
        data = dog.model_dump()

        expected = {"type": "dog", "breed": "Labrador"}
        assert data == expected

    def test_cat_serialization(self):
        """Test serializing a cat variant."""
        cat = PetKindCat(type="cat", lives=8)
        data = cat.model_dump()

        expected = {"type": "cat", "lives": 8}
        assert data == expected

    def test_dog_deserialization(self):
        """Test deserializing a dog variant."""
        data = {"type": "dog", "breed": "Golden Retriever"}
        dog = PetKindDog.model_validate(data)
        assert dog.type == "dog"
        assert dog.breed == "Golden Retriever"

    def test_cat_deserialization(self):
        """Test deserializing a cat variant."""
        data = {"type": "cat", "lives": 9}
        cat = PetKindCat.model_validate(data)
        assert cat.type == "cat"
        assert cat.lives == 9

    def test_invalid_deserialization(self):
        """Test that invalid data fails to deserialize."""
        # Wrong type for dog
        with pytest.raises(ValidationError):
            PetKindDog.model_validate({"type": "cat", "breed": "Poodle"})

        # Wrong type for cat
        with pytest.raises(ValidationError):
            PetKindCat.model_validate({"type": "dog", "lives": 9})


class TestTaggedEnumInModels:
    """Test using tagged enums within other models."""

    def test_pet_with_dog_kind(self):
        """Test Pet model with dog kind."""
        dog = PetKindDog(type="dog", breed="Border Collie")
        pet = Pet(name="Rex", kind=dog, age=5, behaviors=[BehaviorCalm])

        assert pet.name == "Rex"
        assert pet.kind.type == "dog"
        assert pet.kind.breed == "Border Collie"
        assert pet.age == 5

    def test_pet_with_cat_kind(self):
        """Test Pet model with cat kind."""
        cat = PetKindCat(type="cat", lives=7)
        pet = Pet(
            name="Whiskers",
            kind=cat,
            age=3,
            behaviors=[{"Other": {"description": "Custom", "notes": "Test"}}],
        )

        assert pet.name == "Whiskers"
        assert pet.kind.type == "cat"
        assert pet.kind.lives == 7
        assert pet.age == 3

    def test_pet_serialization_with_tagged_enum(self):
        """Test serializing Pet with tagged enum."""
        dog = PetKindDog(type="dog", breed="Bulldog")
        pet = Pet(name="Bruno", kind=dog, age=4)

        data = pet.model_dump()
        expected_kind = {"type": "dog", "breed": "Bulldog"}

        assert data["name"] == "Bruno"
        assert data["kind"] == expected_kind
        assert data["age"] == 4

    def test_pet_deserialization_with_tagged_enum(self):
        """Test deserializing Pet with tagged enum."""
        data = {
            "name": "Luna",
            "kind": {"type": "cat", "lives": 9},
            "age": 2,
            "behaviors": ["Calm"],
        }

        pet = Pet.model_validate(data)
        assert pet.name == "Luna"
        assert pet.kind.type == "cat"
        assert pet.kind.lives == 9
        assert pet.age == 2
        assert len(pet.behaviors) == 1
        assert pet.behaviors[0].root == "Calm"


class TestTaggedEnumInRequests:
    """Test tagged enums in request models."""

    def test_update_request_with_dog_kind(self):
        """Test PetsUpdateRequest with dog kind."""
        dog = PetKindDog(type="dog", breed="Retriever")
        request = PetsUpdateRequest(name="Buddy", kind=dog, age=ReflectapiOption(6))

        assert request.name == "Buddy"
        assert request.kind.type == "dog"
        assert request.kind.breed == "Retriever"
        assert request.age._value == 6

    def test_update_request_serialization_with_tagged_enum(self):
        """Test serializing update request with tagged enum."""
        cat = PetKindCat(type="cat", lives=8)
        request = PetsUpdateRequest(name="Mittens", kind=cat)

        data = request.model_dump()
        assert data["name"] == "Mittens"
        assert data["kind"]["type"] == "cat"
        assert data["kind"]["lives"] == 8


class TestTaggedEnumCompatibility:
    """Test backwards compatibility and edge cases."""

    def test_mixed_pet_types_in_paginated(self):
        """Test paginated results with mixed pet types."""
        now = datetime.now()

        dog = PetKindDog(type="dog", breed="Husky")
        cat = PetKindCat(type="cat", lives=9)

        pets = [
            PetDetails(name="Max", kind=dog, updated_at=now),
            PetDetails(name="Fluffy", kind=cat, updated_at=now),
        ]

        paginated = Paginated[PetDetails](items=pets)

        assert len(paginated.items) == 2
        assert paginated.items[0].kind.type == "dog"
        assert paginated.items[0].kind.breed == "Husky"
        assert paginated.items[1].kind.type == "cat"
        assert paginated.items[1].kind.lives == 9

    def test_union_type_annotation(self):
        """Test that PetKind is properly typed as Union."""
        # This is more of a static analysis test, but we can verify runtime behavior
        dog = PetKindDog(type="dog", breed="Pug")
        cat = PetKindCat(type="cat", lives=7)

        # Both should be acceptable as PetKind
        def accept_pet_kind(kind: PetKind) -> str:
            return kind.type

        assert accept_pet_kind(dog) == "dog"
        assert accept_pet_kind(cat) == "cat"

    def test_discriminator_field_access(self):
        """Test accessing discriminator field across variants."""
        dog = PetKindDog(type="dog", breed="Dalmatian")
        cat = PetKindCat(type="cat", lives=6)

        # Both variants should have the 'type' discriminator field
        assert hasattr(dog, "type")
        assert hasattr(cat, "type")
        assert dog.type == "dog"
        assert cat.type == "cat"

        # But only the appropriate variant should have breed/lives
        assert hasattr(dog, "breed")
        assert not hasattr(dog, "lives")
        assert hasattr(cat, "lives")
        assert not hasattr(cat, "breed")


class TestTaggedEnumErrorHandling:
    """Test error handling and validation edge cases."""

    def test_invalid_discriminator_values(self):
        """Test validation with invalid discriminator values."""
        # Invalid type for dog
        with pytest.raises(ValidationError) as exc_info:
            PetKindDog.model_validate({"type": "bird", "breed": "Canary"})
        assert "Input should be 'dog'" in str(exc_info.value)

        # Invalid type for cat
        with pytest.raises(ValidationError) as exc_info:
            PetKindCat.model_validate({"type": "fish", "lives": 1})
        assert "Input should be 'cat'" in str(exc_info.value)

    def test_missing_variant_specific_fields(self):
        """Test validation when variant-specific fields are missing."""
        # Dog missing breed
        with pytest.raises(ValidationError) as exc_info:
            PetKindDog.model_validate({"type": "dog"})
        assert "breed" in str(exc_info.value)

        # Cat missing lives
        with pytest.raises(ValidationError) as exc_info:
            PetKindCat.model_validate({"type": "cat"})
        assert "lives" in str(exc_info.value)

    def test_extra_fields_ignored(self):
        """Test that extra fields are ignored with the model config."""
        # Dog with extra field should work (extra="ignore")
        dog_data = {
            "type": "dog",
            "breed": "Beagle",
            "extra_field": "should_be_ignored",
        }
        dog = PetKindDog.model_validate(dog_data)
        assert dog.type == "dog"
        assert dog.breed == "Beagle"
        assert not hasattr(dog, "extra_field")
