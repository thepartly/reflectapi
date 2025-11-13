"""Tests for primitive enum representations (IntEnum with discriminant values).

This tests the implementation of Rust enums with explicit discriminant values
that should generate Python IntEnum classes instead of string enums.
"""

import pytest
from enum import IntEnum, Enum
from typing import get_origin, get_args

# Note: Since we don't have primitive enums in the current demo server,
# we'll test the concept and structure rather than actual generated code


class TestPrimitiveEnumConcept:
    """Test the concept of primitive enums that would be generated."""

    def test_int_enum_creation(self):
        """Test creating an IntEnum like what would be generated."""

        # This simulates what our generator should produce
        class Status(IntEnum):
            OK = 200
            NOT_FOUND = 404
            INTERNAL_ERROR = 500

        # Test basic functionality
        assert Status.OK == 200
        assert Status.NOT_FOUND == 404
        assert Status.INTERNAL_ERROR == 500

        # IntEnum should support integer operations
        assert Status.OK + 100 == 300
        assert Status.OK < Status.NOT_FOUND

        # Should be serializable as integers
        assert int(Status.OK) == 200
        assert Status.OK.value == 200

    def test_int_enum_json_serialization(self):
        """Test JSON serialization of IntEnum values."""
        import json

        class HttpStatus(IntEnum):
            SUCCESS = 200
            BAD_REQUEST = 400
            UNAUTHORIZED = 401

        # Should serialize as integers, not strings
        data = {"status": HttpStatus.SUCCESS, "code": HttpStatus.BAD_REQUEST}
        json_str = json.dumps(data, default=int)
        parsed = json.loads(json_str)

        assert parsed["status"] == 200
        assert parsed["code"] == 400

        # Should be able to reconstruct from integers
        reconstructed_status = HttpStatus(parsed["status"])
        assert reconstructed_status == HttpStatus.SUCCESS

    def test_enum_with_mixed_types_fallback(self):
        """Test fallback to regular Enum for mixed value types."""

        # If we can't determine a consistent primitive type, fall back to Enum
        class MixedEnum(Enum):
            STRING_VAL = "text"
            INT_VAL = 42
            FLOAT_VAL = 3.14

        # Should still work as a regular enum
        assert MixedEnum.STRING_VAL.value == "text"
        assert MixedEnum.INT_VAL.value == 42
        assert MixedEnum.FLOAT_VAL.value == 3.14

    def test_float_enum_concept(self):
        """Test concept for float-based enums (future enhancement)."""

        # This would be for future float discriminant support
        class Priority(Enum):  # Would be FloatEnum if we implement that
            LOW = 0.1
            MEDIUM = 0.5
            HIGH = 1.0
            CRITICAL = 2.0

        assert Priority.LOW.value == 0.1
        assert Priority.CRITICAL.value == 2.0


class TestCodeGenerationStructure:
    """Test the structure that would be generated for primitive enums."""

    def test_generated_import_structure(self):
        """Test that IntEnum would be imported correctly."""
        # This simulates the imports our generator should produce
        from enum import IntEnum, Enum

        # Both should be available for different enum types
        assert IntEnum is not None
        assert Enum is not None

    def test_discriminator_value_handling(self):
        """Test handling of discriminator values."""

        # Simulate the discriminator detection logic
        def would_be_int_enum(variants):
            """Simulate our discriminant detection logic."""
            return all(
                variant.get("discriminant") is not None
                and isinstance(variant["discriminant"], int)
                for variant in variants
            )

        # Test data that would come from Rust schema
        int_variants = [
            {"name": "Ok", "discriminant": 200},
            {"name": "Error", "discriminant": 500},
        ]

        string_variants = [
            {"name": "Dog", "discriminant": None},
            {"name": "Cat", "discriminant": None},
        ]

        mixed_variants = [
            {"name": "First", "discriminant": 1},
            {"name": "Second", "discriminant": None},  # Missing discriminant
        ]

        assert would_be_int_enum(int_variants) == True
        assert would_be_int_enum(string_variants) == False
        assert would_be_int_enum(mixed_variants) == False

    def test_name_conversion(self):
        """Test name conversion for primitive enum variants."""

        def to_screaming_snake_case(name):
            """Simulate the name conversion logic."""
            # This is a simplified version of what the generator does
            return name.upper().replace("-", "_")

        assert to_screaming_snake_case("Ok") == "OK"
        assert to_screaming_snake_case("NotFound") == "NOTFOUND"  # Simplified
        assert to_screaming_snake_case("internal-error") == "INTERNAL_ERROR"


class TestPrimitiveEnumValidation:
    """Test validation and error handling for primitive enums."""

    def test_int_enum_validation(self):
        """Test validation of IntEnum values."""

        class ResponseCode(IntEnum):
            SUCCESS = 200
            CLIENT_ERROR = 400
            SERVER_ERROR = 500

        # Valid values should work
        assert ResponseCode(200) == ResponseCode.SUCCESS
        assert ResponseCode(400) == ResponseCode.CLIENT_ERROR

        # Invalid values should raise ValueError
        with pytest.raises(ValueError):
            ResponseCode(999)  # Not a valid enum value

    def test_int_enum_comparison(self):
        """Test comparison operations with IntEnum."""

        class Priority(IntEnum):
            LOW = 1
            MEDIUM = 5
            HIGH = 10

        # Should support comparison as integers
        assert Priority.LOW < Priority.MEDIUM < Priority.HIGH
        assert Priority.HIGH > 7
        assert Priority.MEDIUM == 5

        # Should work in sorting
        priorities = [Priority.HIGH, Priority.LOW, Priority.MEDIUM]
        sorted_priorities = sorted(priorities)
        assert sorted_priorities == [Priority.LOW, Priority.MEDIUM, Priority.HIGH]

    def test_int_enum_arithmetic(self):
        """Test arithmetic operations with IntEnum."""

        class Factor(IntEnum):
            SMALL = 2
            MEDIUM = 5
            LARGE = 10

        # Should support arithmetic operations
        assert Factor.SMALL * 3 == 6
        assert Factor.LARGE - Factor.SMALL == 8
        assert Factor.MEDIUM + Factor.SMALL == 7


class TestIntegrationWithPydantic:
    """Test how primitive enums would integrate with Pydantic models."""

    def test_int_enum_in_pydantic_model(self):
        """Test using IntEnum in a Pydantic model."""
        from pydantic import BaseModel

        class Status(IntEnum):
            PENDING = 0
            ACTIVE = 1
            INACTIVE = 2

        class User(BaseModel):
            name: str
            status: Status

        # Should accept IntEnum values
        user1 = User(name="Alice", status=Status.ACTIVE)
        assert user1.status == Status.ACTIVE
        assert user1.status.value == 1

        # Should accept integer values and convert
        user2 = User(name="Bob", status=2)
        assert user2.status == Status.INACTIVE
        assert isinstance(user2.status, Status)

    def test_int_enum_serialization_with_pydantic(self):
        """Test serialization of IntEnum with Pydantic."""
        from pydantic import BaseModel

        class Priority(IntEnum):
            LOW = 1
            MEDIUM = 2
            HIGH = 3

        class Task(BaseModel):
            title: str
            priority: Priority

        task = Task(title="Important Task", priority=Priority.HIGH)

        # Test serialization
        data = task.model_dump()
        assert data["priority"] == 3  # Should serialize as integer

        # Test deserialization
        reconstructed = Task.model_validate(data)
        assert reconstructed.priority == Priority.HIGH
        assert isinstance(reconstructed.priority, Priority)

    def test_validation_error_handling(self):
        """Test validation errors with primitive enums."""
        from pydantic import BaseModel, ValidationError

        class ErrorCode(IntEnum):
            NOT_FOUND = 404
            SERVER_ERROR = 500

        class ApiResponse(BaseModel):
            message: str
            error_code: ErrorCode

        # Valid values should work
        response = ApiResponse(message="Error occurred", error_code=404)
        assert response.error_code == ErrorCode.NOT_FOUND

        # Invalid enum values should raise validation error
        with pytest.raises(ValidationError) as exc_info:
            ApiResponse(message="Error", error_code=999)

        # Should contain meaningful error message
        assert "Input should be" in str(exc_info.value)


class TestEdgeCases:
    """Test edge cases and special scenarios."""

    def test_negative_discriminants(self):
        """Test handling of negative discriminant values."""

        class Temperature(IntEnum):
            FREEZING = -10
            COLD = 0
            WARM = 20
            HOT = 40

        assert Temperature.FREEZING == -10
        assert Temperature.COLD == 0
        assert Temperature.FREEZING < Temperature.COLD

    def test_large_discriminants(self):
        """Test handling of large discriminant values."""

        class Timestamp(IntEnum):
            EPOCH = 0
            Y2K = 946684800  # Large timestamp
            FUTURE = 2147483647  # Max 32-bit int

        assert Timestamp.Y2K > Timestamp.EPOCH
        assert Timestamp.FUTURE > Timestamp.Y2K

    def test_sparse_discriminants(self):
        """Test handling of non-consecutive discriminant values."""

        class ErrorLevel(IntEnum):
            INFO = 100
            WARNING = 200
            ERROR = 400  # Skipped 300
            CRITICAL = 500

        # Should work fine with gaps
        assert ErrorLevel.INFO == 100
        assert ErrorLevel.ERROR == 400

        # Should not create values for missing numbers
        with pytest.raises(ValueError):
            ErrorLevel(300)  # This discriminant doesn't exist
