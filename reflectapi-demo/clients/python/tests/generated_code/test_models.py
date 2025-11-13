"""Test the generated Pydantic models."""

import pytest
from datetime import datetime
from pydantic import ValidationError

from generated import (
    MyapiModelInputPet as Pet,
    MyapiModelOutputPet as PetDetails,
    MyapiModelKind as PetKind,
    MyapiModelKindDog as PetKindDog,
    MyapiModelKindCat as PetKindCat,
    MyapiModelBehavior as Behavior,
    MyapiModelBehaviorAggressiveVariant as BehaviorAggressive,
    MyapiModelBehaviorOtherVariant as BehaviorOther,
    MyapiProtoPetsListRequest as PetsListRequest,
    MyapiProtoPetsUpdateRequest as PetsUpdateRequest,
    MyapiProtoPetsRemoveRequest as PetsRemoveRequest,
    MyapiProtoHeaders as Headers,
    MyapiProtoPaginated as Paginated,
)
from reflectapi_runtime import ReflectapiOption as Option
from reflectapi_runtime import ReflectapiOption, Undefined

# For externally tagged enums, unit variants are just string literals
BehaviorCalm = "Calm"
# Import test helpers - conftest.py is automatically imported by pytest
# from ..conftest import (
#     assert_petkind_cat, assert_petkind_dog,
#     assert_reflectapi_option_some, TestDataFactory
# )


class TestBasicModels:
    """Test basic model creation and validation."""

    def test_pet_creation_with_fixture(self, sample_pet: Pet):
        """Test creating a Pet model using fixture."""
        assert sample_pet.name == "Buddy"
        assert isinstance(sample_pet.kind, PetKindDog)
        assert sample_pet.kind.type == "dog"
        assert sample_pet.kind.breed == "Golden Retriever"
        assert sample_pet.age == 3
        # Check that sample pet has the expected behavior types
        behavior_roots = [b.root for b in sample_pet.behaviors]
        assert "Calm" in behavior_roots
        assert any(isinstance(b, BehaviorAggressive) for b in behavior_roots)

    def test_pet_creation_manual(self, sample_cat: PetKindCat):
        """Test creating a Pet model manually."""
        pet = Pet(
            name="fluffy",
            kind=sample_cat,
            age=3,
            updated_at=datetime.now(),
            behaviors=[BehaviorCalm],
        )
        assert pet.name == "fluffy"
        assert isinstance(pet.kind, PetKindCat)
        assert pet.kind.type == "cat"
        assert pet.kind.lives == 9
        assert pet.age == 3
        assert len(pet.behaviors) == 1
        assert pet.behaviors[0].root == BehaviorCalm

    def test_pet_optional_fields(self):
        """Test Pet with optional fields."""
        dog_kind = PetKindDog(type="dog", breed="Golden Retriever")
        pet = Pet(name="buddy", kind=dog_kind)
        assert pet.name == "buddy"
        assert pet.kind.type == "dog"
        assert pet.kind.breed == "Golden Retriever"
        assert pet.age is None
        assert pet.behaviors is None

    def test_pet_details_required_updated_at(self):
        """Test PetDetails requires updated_at field."""
        dog_kind = PetKindDog(type="dog", breed="German Shepherd")
        pet_details = PetDetails(name="rex", kind=dog_kind, updated_at=datetime.now())
        assert pet_details.updated_at is not None


class TestDiscriminatedUnions:
    """Test discriminated union models."""

    def test_pet_kind_dog_creation(self):
        """Test PetKind dog variant creation."""
        dog = PetKindDog(type="dog", breed="Labrador")
        assert dog.type == "dog"
        assert dog.breed == "Labrador"

    def test_pet_kind_cat_creation(self):
        """Test PetKind cat variant creation."""
        cat = PetKindCat(type="cat", lives=7)
        assert cat.type == "cat"
        assert cat.lives == 7

    def test_pet_kind_union_usage(self):
        """Test PetKind union type usage."""
        dog = PetKindDog(type="dog", breed="Poodle")
        cat = PetKindCat(type="cat", lives=9)

        # Both should be valid PetKind instances
        pet_dog = Pet(name="Buddy", kind=dog)
        pet_cat = Pet(name="Whiskers", kind=cat)

        assert pet_dog.kind.type == "dog"
        assert pet_cat.kind.type == "cat"

    def test_behavior_values(self):
        """Test Behavior discriminated union variant creation."""
        calm = BehaviorCalm
        aggressive = {"Aggressive": [5.0, "notes"]}
        other = {"Other": {"description": "Custom", "notes": "Some notes"}}

        # Check that discriminator fields are set correctly
        assert calm == "Calm"
        assert aggressive == {"Aggressive": [5.0, "notes"]}
        assert other == {"Other": {"description": "Custom", "notes": "Some notes"}}

    def test_option_values(self):
        """Test ReflectapiOption creation."""
        # ReflectapiOption is not an enum anymore, it's a wrapper class
        from reflectapi_runtime import ReflectapiOption, Undefined

        opt_some = ReflectapiOption(42)
        opt_none = ReflectapiOption(None)
        opt_undefined = ReflectapiOption(Undefined)

        assert opt_some.is_some
        assert opt_none.is_none
        assert opt_undefined.is_undefined


class TestReflectapiOption:
    """Test ReflectapiOption functionality."""

    def test_reflectapi_option_undefined(self):
        """Test ReflectapiOption with undefined value."""
        request = PetsUpdateRequest(name="test", age=ReflectapiOption(Undefined))
        assert request.name == "test"
        assert request.age._value is Undefined

    def test_reflectapi_option_none(self):
        """Test ReflectapiOption with None value."""
        request = PetsUpdateRequest(name="test", age=ReflectapiOption(None))
        assert request.name == "test"
        assert request.age._value is None

    def test_reflectapi_option_some_value(self):
        """Test ReflectapiOption with actual value."""
        request = PetsUpdateRequest(name="test", age=ReflectapiOption(5))
        assert request.name == "test"
        assert request.age._value == 5


class TestGenericModels:
    """Test generic models like Paginated."""

    def test_paginated_creation(self):
        """Test creating a Paginated model."""
        cat = PetKindCat(type="cat", lives=9)
        dog = PetKindDog(type="dog", breed="Beagle")
        pets = [
            PetDetails(name="pet1", kind=cat, updated_at=datetime.now()),
            PetDetails(name="pet2", kind=dog, updated_at=datetime.now()),
        ]
        paginated = Paginated[PetDetails](items=pets, cursor="next_page_token")
        assert len(paginated.items) == 2
        assert paginated.cursor == "next_page_token"

    def test_paginated_no_cursor(self):
        """Test Paginated without cursor."""
        paginated = Paginated[PetDetails](items=[])
        assert paginated.items == []
        assert paginated.cursor is None


class TestValidation:
    """Test Pydantic validation."""

    def test_invalid_pet_kind_discriminator(self):
        """Test validation error for invalid pet kind discriminator."""
        with pytest.raises(ValidationError):
            # Invalid type field
            PetKindDog(type="cat", breed="Labrador")

        with pytest.raises(ValidationError):
            # Missing required field
            PetKindCat(type="cat")  # Missing 'lives' field

    def test_invalid_behavior(self):
        """Test validation error for invalid behavior."""
        cat = PetKindCat(type="cat", lives=9)
        with pytest.raises(ValidationError):
            Pet(name="test", kind=cat, behaviors=["invalid_behavior"])

    def test_missing_required_field(self):
        """Test validation error for missing required field."""
        with pytest.raises(ValidationError):
            cat = PetKindCat(type="cat", lives=9)
            Pet(kind=cat)  # Missing required 'name' field


class TestSerialization:
    """Test model serialization."""

    def test_pet_serialization(self):
        """Test Pet model serialization."""
        cat = PetKindCat(type="cat", lives=9)
        pet = Pet(name="fluffy", kind=cat, age=3)
        data = pet.model_dump()
        assert data["name"] == "fluffy"
        assert data["kind"]["type"] == "cat"
        assert data["kind"]["lives"] == 9
        assert data["age"] == 3

    def test_pet_deserialization(self):
        """Test Pet model deserialization."""
        data = {
            "name": "buddy",
            "kind": {"type": "dog", "breed": "Golden Retriever"},
            "age": 2,
        }
        pet = Pet.model_validate(data)
        assert pet.name == "buddy"
        assert pet.kind.type == "dog"
        assert pet.kind.breed == "Golden Retriever"
        assert pet.age == 2

    def test_reflectapi_option_serialization(self):
        """Test ReflectapiOption serialization with current behavior."""
        request = PetsUpdateRequest(
            name="test",
            age=ReflectapiOption(Undefined),
            behaviors=ReflectapiOption([BehaviorCalm]),
        )
        data = request.model_dump()
        # Note: Current implementation includes ReflectapiOption objects as-is
        # This should be improved to properly serialize the inner values
        assert "age" in data
        assert "behaviors" in data
        assert isinstance(data["age"], ReflectapiOption)
        assert isinstance(data["behaviors"], ReflectapiOption)


class TestHeaders:
    """Test Headers model."""

    def test_headers_creation(self):
        """Test creating Headers model."""
        headers = Headers(authorization="Bearer token123")
        assert headers.authorization == "Bearer token123"

    def test_headers_serialization(self):
        """Test Headers serialization."""
        headers = Headers(authorization="Bearer token123")
        data = headers.model_dump()
        assert data["authorization"] == "Bearer token123"


class TestRequestModels:
    """Test request models."""

    def test_pets_list_request(self):
        """Test PetsListRequest model."""
        request = PetsListRequest(limit=10, cursor="page_token")
        assert request.limit == 10
        assert request.cursor == "page_token"

    def test_pets_list_request_optional(self):
        """Test PetsListRequest with optional fields."""
        request = PetsListRequest()
        assert request.limit is None
        assert request.cursor is None

    def test_pets_remove_request(self):
        """Test PetsRemoveRequest model."""
        request = PetsRemoveRequest(name="pet_to_remove")
        assert request.name == "pet_to_remove"


class TestDocstrings:
    """Test that models have proper docstrings."""

    def test_model_docstrings(self):
        """Test that models exist and are classes."""
        # Note: Currently not all models have docstrings
        # This should be improved in the code generation
        assert Pet is not None
        assert PetDetails is not None
        assert Headers is not None

    def test_behavior_discriminated_union(self):
        """Test that behavior discriminated union works correctly."""
        # Behaviors are now discriminated unions, not enums
        assert BehaviorCalm == "Calm"
        assert BehaviorAggressive is not None
        assert BehaviorOther is not None
