"""Test error handling in generated code."""

import pytest
from pydantic import ValidationError

from generated import (
    MyapiModelInputPet as Pet,
    MyapiModelKind as PetKind,
    MyapiModelKindDog as PetKindDog,
    MyapiModelKindCat as PetKindCat,
    MyapiModelBehavior as Behavior,
    MyapiProtoPetsCreateError as PetsCreateError,
    MyapiProtoPetsListError as PetsListError,
    MyapiProtoPetsUpdateError as PetsUpdateError,
    MyapiProtoPetsRemoveError as PetsRemoveError
)


class TestModelValidation:
    """Test validation errors in generated models."""

    def test_pet_missing_required_fields(self):
        """Test that missing required fields raise ValidationError."""
        with pytest.raises(ValidationError) as exc_info:
            Pet()  # Missing name and kind

        errors = exc_info.value.errors()
        assert len(errors) >= 2
        field_names = {error['loc'][0] for error in errors}
        assert 'name' in field_names
        assert 'kind' in field_names

    def test_pet_invalid_field_types(self):
        """Test that invalid field types raise ValidationError."""
        with pytest.raises(ValidationError) as exc_info:
            Pet(
                name=123,  # Should be string
                kind="not_a_dict",  # Should be PetKind object
                age="not_an_int"  # Should be int
            )

        errors = exc_info.value.errors()
        assert len(errors) >= 3

    def test_petkind_dog_missing_breed(self):
        """Test that PetKindDog requires breed field."""
        with pytest.raises(ValidationError) as exc_info:
            PetKindDog(type='dog')  # Missing breed

        errors = exc_info.value.errors()
        assert any(error['loc'] == ('breed',) for error in errors)

    def test_petkind_cat_missing_lives(self):
        """Test that PetKindCat requires lives field."""
        with pytest.raises(ValidationError) as exc_info:
            PetKindCat(type='cat')  # Missing lives

        errors = exc_info.value.errors()
        assert any(error['loc'] == ('lives',) for error in errors)

    def test_petkind_invalid_discriminator(self):
        """Test that invalid discriminator values are rejected."""
        with pytest.raises(ValidationError) as exc_info:
            PetKindDog(type='cat', breed='Golden Retriever')  # Wrong type

        errors = exc_info.value.errors()
        assert any('literal' in error['type'] for error in errors)

    def test_behavior_invalid_enum_value(self):
        """Test that invalid enum values are rejected."""
        with pytest.raises(ValidationError) as exc_info:
            Pet(
                name="Test",
                kind=PetKindDog(type='dog', breed='Labrador'),
                behaviors=["INVALID_BEHAVIOR"]
            )

        errors = exc_info.value.errors()
        assert len(errors) >= 1


class TestErrorEnums:
    """Test error enum types."""

    def test_pets_create_error_values(self):
        """Test PetsCreateError factory-created variants."""
        from generated import MyapiProtoPetsCreateErrorFactory as PetsCreateErrorFactory
        conflict = PetsCreateErrorFactory.CONFLICT
        assert conflict.root == "Conflict"
        not_auth = PetsCreateErrorFactory.NOTAUTHORIZED
        assert not_auth.root == "NotAuthorized"
        invalid = PetsCreateErrorFactory.invalid_identity("test")
        assert hasattr(invalid, 'message')
        assert invalid.message == "test"

    def test_pets_list_error_values(self):
        """Test PetsListError enum values."""
        assert PetsListError.INVALID_CURSOR == "InvalidCursor"
        assert PetsListError.UNAUTHORIZED == "Unauthorized"

        # Test enum completeness
        expected_values = {"InvalidCursor", "Unauthorized"}
        actual_values = {error.value for error in PetsListError}
        assert actual_values == expected_values

    def test_pets_update_error_values(self):
        """Test PetsUpdateError enum values."""
        assert PetsUpdateError.NOT_FOUND == "NotFound"
        assert PetsUpdateError.NOT_AUTHORIZED == "NotAuthorized"

    def test_pets_remove_error_values(self):
        """Test PetsRemoveError enum values."""
        assert PetsRemoveError.NOT_FOUND == "NotFound"
        assert PetsRemoveError.NOT_AUTHORIZED == "NotAuthorized"


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_pet_empty_name(self):
        """Test pet with empty name."""
        pet = Pet(
            name="",  # Empty but valid string
            kind=PetKindDog(type='dog', breed='Labrador')
        )
        assert pet.name == ""

    def test_pet_very_long_name(self):
        """Test pet with very long name."""
        long_name = "a" * 1000
        pet = Pet(
            name=long_name,
            kind=PetKindDog(type='dog', breed='Labrador')
        )
        assert pet.name == long_name

    def test_pet_unicode_name(self):
        """Test pet with unicode characters in name."""
        unicode_name = "🐕 Wölfgang 犬"
        pet = Pet(
            name=unicode_name,
            kind=PetKindDog(type='dog', breed='Labrador')
        )
        assert pet.name == unicode_name

    def test_cat_zero_lives(self):
        """Test cat with zero lives."""
        cat = PetKindCat(type='cat', lives=0)
        assert cat.lives == 0

    def test_cat_negative_lives(self):
        """Test cat with negative lives."""
        cat = PetKindCat(type='cat', lives=-1)
        assert cat.lives == -1

    def test_pet_empty_behaviors_list(self):
        """Test pet with empty behaviors list."""
        pet = Pet(
            name="Test",
            kind=PetKindDog(type='dog', breed='Labrador'),
            behaviors=[]
        )
        assert pet.behaviors == []

    def test_pet_duplicate_behaviors(self):
        """Test pet with duplicate behaviors."""
        from generated import MyapiModelBehaviorFactory as BehaviorFactory
        pet = Pet(
            name="Test",
            kind=PetKindDog(type='dog', breed='Labrador'),
            behaviors=[BehaviorFactory.CALM, BehaviorFactory.CALM, BehaviorFactory.aggressive(3.0, "test")]
        )
        # Duplicates should be preserved - check root values for externally tagged enums
        roots = [b.root if b.root == "Calm" else "Aggressive" if hasattr(b.root, 'field_0') else str(b.root) for b in pet.behaviors]
        assert roots == ["Calm", "Calm", "Aggressive"]


class TestSerialization:
    """Test serialization and deserialization edge cases."""

    def test_round_trip_serialization(self, sample_pet):
        """Test that models can be serialized and deserialized."""
        # Serialize to dict
        pet_dict = sample_pet.model_dump()

        # Deserialize back to model
        reconstructed_pet = Pet.model_validate(pet_dict)

        assert reconstructed_pet.name == sample_pet.name
        assert reconstructed_pet.kind.type == sample_pet.kind.type
        assert reconstructed_pet.age == sample_pet.age
        assert reconstructed_pet.behaviors == sample_pet.behaviors

    def test_json_serialization(self, sample_pet):
        """Test JSON serialization and deserialization."""
        import json

        # Serialize to JSON
        pet_json = sample_pet.model_dump_json()
        pet_dict = json.loads(pet_json)

        # Verify structure
        assert 'name' in pet_dict
        assert 'kind' in pet_dict
        assert pet_dict['kind']['type'] in ['dog', 'cat']

        # Deserialize from JSON
        reconstructed_pet = Pet.model_validate_json(pet_json)
        assert reconstructed_pet.name == sample_pet.name

    def test_partial_model_updates(self):
        """Test updating models with partial data."""
        original_pet = Pet(
            name="Original",
            kind=PetKindDog(type='dog', breed='Labrador'),
            age=5
        )

        # Create updated version with model_copy
        updated_pet = original_pet.model_copy(update={'name': 'Updated', 'age': 6})

        assert updated_pet.name == 'Updated'
        assert updated_pet.age == 6
        assert updated_pet.kind.breed == 'Labrador'  # Unchanged
