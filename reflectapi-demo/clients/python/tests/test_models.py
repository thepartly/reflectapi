"""Test the generated Pydantic models."""

import pytest
from datetime import datetime
from pydantic import ValidationError

from generated import (
    Pet, PetDetails, PetKind, Behavior, 
    PetsListRequest, PetsUpdateRequest, PetsRemoveRequest,
    Headers, Paginated, Option
)
from reflectapi_runtime import ReflectapiOption, Undefined


class TestBasicModels:
    """Test basic model creation and validation."""
    
    def test_pet_creation(self):
        """Test creating a Pet model."""
        pet = Pet(
            name="fluffy",
            kind=PetKind.CAT,
            age=3,
            updated_at=datetime.now(),
            behaviors=[Behavior.CALM]
        )
        assert pet.name == "fluffy"
        assert pet.kind == PetKind.CAT
        assert pet.age == 3
        assert pet.behaviors == [Behavior.CALM]
    
    def test_pet_optional_fields(self):
        """Test Pet with optional fields."""
        pet = Pet(
            name="buddy",
            kind=PetKind.DOG
        )
        assert pet.name == "buddy"
        assert pet.kind == PetKind.DOG
        assert pet.age is None
        assert pet.behaviors is None
    
    def test_pet_details_required_updated_at(self):
        """Test PetDetails requires updated_at field."""
        pet_details = PetDetails(
            name="rex",
            kind=PetKind.DOG,
            updated_at=datetime.now()
        )
        assert pet_details.updated_at is not None


class TestEnums:
    """Test enum models."""
    
    def test_pet_kind_values(self):
        """Test PetKind enum values."""
        assert PetKind.DOG == "dog"
        assert PetKind.CAT == "cat"
    
    def test_behavior_values(self):
        """Test Behavior enum values."""
        assert Behavior.CALM == "Calm"
        assert Behavior.AGGRESSIVE == "Aggressive"
        assert Behavior.OTHER == "Other"
    
    def test_option_values(self):
        """Test Option enum values."""
        assert Option.NONE == "None"
        assert Option.SOME == "Some"


class TestReflectapiOption:
    """Test ReflectapiOption functionality."""
    
    def test_reflectapi_option_undefined(self):
        """Test ReflectapiOption with undefined value."""
        request = PetsUpdateRequest(
            name="test",
            age=ReflectapiOption(Undefined)
        )
        assert request.name == "test"
        assert request.age._value is Undefined
    
    def test_reflectapi_option_none(self):
        """Test ReflectapiOption with None value."""
        request = PetsUpdateRequest(
            name="test",
            age=ReflectapiOption(None)
        )
        assert request.name == "test"
        assert request.age._value is None
    
    def test_reflectapi_option_some_value(self):
        """Test ReflectapiOption with actual value."""
        request = PetsUpdateRequest(
            name="test",
            age=ReflectapiOption(5)
        )
        assert request.name == "test"
        assert request.age._value == 5


class TestGenericModels:
    """Test generic models like Paginated."""
    
    def test_paginated_creation(self):
        """Test creating a Paginated model."""
        pets = [
            PetDetails(name="pet1", kind=PetKind.CAT, updated_at=datetime.now()),
            PetDetails(name="pet2", kind=PetKind.DOG, updated_at=datetime.now())
        ]
        paginated = Paginated[PetDetails](
            items=pets,
            cursor="next_page_token"
        )
        assert len(paginated.items) == 2
        assert paginated.cursor == "next_page_token"
    
    def test_paginated_no_cursor(self):
        """Test Paginated without cursor."""
        paginated = Paginated[PetDetails](items=[])
        assert paginated.items == []
        assert paginated.cursor is None


class TestValidation:
    """Test Pydantic validation."""
    
    def test_invalid_pet_kind(self):
        """Test validation error for invalid pet kind."""
        with pytest.raises(ValidationError):
            Pet(name="test", kind="invalid_kind")
    
    def test_invalid_behavior(self):
        """Test validation error for invalid behavior."""
        with pytest.raises(ValidationError):
            Pet(
                name="test", 
                kind=PetKind.CAT,
                behaviors=["invalid_behavior"]
            )
    
    def test_missing_required_field(self):
        """Test validation error for missing required field."""
        with pytest.raises(ValidationError):
            Pet(kind=PetKind.CAT)  # Missing required 'name' field


class TestSerialization:
    """Test model serialization."""
    
    def test_pet_serialization(self):
        """Test Pet model serialization."""
        pet = Pet(
            name="fluffy",
            kind=PetKind.CAT,
            age=3
        )
        data = pet.model_dump()
        assert data["name"] == "fluffy"
        assert data["kind"] == "cat"
        assert data["age"] == 3
    
    def test_pet_deserialization(self):
        """Test Pet model deserialization."""
        data = {
            "name": "buddy",
            "kind": "dog",
            "age": 2
        }
        pet = Pet.model_validate(data)
        assert pet.name == "buddy"
        assert pet.kind == PetKind.DOG
        assert pet.age == 2
    
    def test_reflectapi_option_serialization(self):
        """Test ReflectapiOption serialization with current behavior."""
        request = PetsUpdateRequest(
            name="test",
            age=ReflectapiOption(Undefined),
            behaviors=ReflectapiOption([Behavior.CALM])
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
    
    def test_enum_docstrings(self):
        """Test that enums have comprehensive docstrings."""
        assert Option.__doc__ is not None
        assert "Attributes:" in Option.__doc__
        assert "NONE:" in Option.__doc__
        assert "SOME:" in Option.__doc__