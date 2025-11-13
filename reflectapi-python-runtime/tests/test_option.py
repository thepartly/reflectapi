"""Tests for Option handling."""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

import pytest
from pydantic import BaseModel

from reflectapi_runtime import (
    Option,
    ReflectapiOption,
    Undefined,
    none,
    serialize_option_dict,
    some,
    undefined,
)


class TestUndefinedType:
    """Test the _UndefinedType sentinel."""

    def test_singleton_behavior(self):
        """Test that Undefined behaves as a singleton."""
        from reflectapi_runtime.option import _UndefinedType

        another_undefined = _UndefinedType()
        assert Undefined == another_undefined
        assert hash(Undefined) == hash(another_undefined)

    def test_string_representation(self):
        """Test string representations."""
        assert str(Undefined) == "Undefined"
        assert repr(Undefined) == "Undefined"

    def test_boolean_conversion(self):
        """Test boolean conversion."""
        assert not Undefined
        assert bool(Undefined) is False


class TestReflectapiOption:
    """Test ReflectapiOption functionality."""

    def test_default_constructor(self):
        """Test default constructor creates undefined option."""
        option = ReflectapiOption()
        assert option.is_undefined
        assert not option.is_none
        assert not option.is_some

    def test_undefined_constructor(self):
        """Test constructor with Undefined."""
        option = ReflectapiOption(Undefined)
        assert option.is_undefined
        assert not option.is_none
        assert not option.is_some

    def test_none_constructor(self):
        """Test constructor with None."""
        option = ReflectapiOption(None)
        assert not option.is_undefined
        assert option.is_none
        assert not option.is_some

    def test_value_constructor(self):
        """Test constructor with actual value."""
        option = ReflectapiOption(42)
        assert not option.is_undefined
        assert not option.is_none
        assert option.is_some

    def test_value_property_undefined(self):
        """Test value property with undefined option."""
        option = ReflectapiOption(Undefined)

        with pytest.raises(ValueError, match="Cannot access value of undefined option"):
            _ = option.value

    def test_value_property_none(self):
        """Test value property with None."""
        option = ReflectapiOption(None)
        assert option.value is None

    def test_value_property_some(self):
        """Test value property with actual value."""
        option = ReflectapiOption(42)
        assert option.value == 42

    def test_unwrap_undefined(self):
        """Test unwrap with undefined option."""
        option = ReflectapiOption(Undefined)

        with pytest.raises(ValueError, match="Cannot unwrap undefined option"):
            option.unwrap()

    def test_unwrap_none(self):
        """Test unwrap with None."""
        option = ReflectapiOption(None)

        with pytest.raises(ValueError, match="Cannot unwrap None option"):
            option.unwrap()

    def test_unwrap_some(self):
        """Test unwrap with actual value."""
        option = ReflectapiOption(42)
        assert option.unwrap() == 42

    def test_unwrap_or_undefined(self):
        """Test unwrap_or with undefined option."""
        option = ReflectapiOption(Undefined)
        assert option.unwrap_or(99) == 99

    def test_unwrap_or_none(self):
        """Test unwrap_or with None."""
        option = ReflectapiOption(None)
        assert option.unwrap_or(99) == 99

    def test_unwrap_or_some(self):
        """Test unwrap_or with actual value."""
        option = ReflectapiOption(42)
        assert option.unwrap_or(99) == 42

    def test_map_undefined(self):
        """Test map with undefined option."""
        option = ReflectapiOption(Undefined)
        result = option.map(lambda x: x * 2)

        assert result.is_undefined

    def test_map_none(self):
        """Test map with None."""
        option = ReflectapiOption(None)
        result = option.map(lambda x: x * 2)

        assert result.is_none

    def test_map_some(self):
        """Test map with actual value."""
        option = ReflectapiOption(42)
        result = option.map(lambda x: x * 2)

        assert result.is_some
        assert result.unwrap() == 84

    def test_filter_undefined(self):
        """Test filter with undefined option."""
        option = ReflectapiOption(Undefined)
        result = option.filter(lambda x: x > 10)

        assert result.is_undefined

    def test_filter_none(self):
        """Test filter with None."""
        option = ReflectapiOption(None)
        result = option.filter(lambda x: x > 10)

        assert result.is_undefined

    def test_filter_some_true(self):
        """Test filter with value that passes predicate."""
        option = ReflectapiOption(42)
        result = option.filter(lambda x: x > 10)

        assert result.is_some
        assert result.unwrap() == 42

    def test_filter_some_false(self):
        """Test filter with value that fails predicate."""
        option = ReflectapiOption(5)
        result = option.filter(lambda x: x > 10)

        assert result.is_undefined

    def test_equality_options(self):
        """Test equality between options."""
        option1 = ReflectapiOption(42)
        option2 = ReflectapiOption(42)
        option3 = ReflectapiOption(99)
        option4 = ReflectapiOption(None)
        option5 = ReflectapiOption(Undefined)

        assert option1 == option2
        assert option1 != option3
        assert option1 != option4
        assert option1 != option5
        assert option5 == ReflectapiOption(Undefined)

    def test_equality_values(self):
        """Test equality with raw values."""
        option = ReflectapiOption(42)

        assert option == 42
        assert option != 99
        assert option is not None
        assert option != Undefined

    def test_string_representations(self):
        """Test string representations."""
        undefined_option = ReflectapiOption(Undefined)
        none_option = ReflectapiOption(None)
        some_option = ReflectapiOption(42)

        assert str(undefined_option) == "Undefined"
        assert str(none_option) == "None"
        assert str(some_option) == "42"

        assert repr(undefined_option) == "ReflectapiOption(Undefined)"
        assert repr(none_option) == "ReflectapiOption(None)"
        assert repr(some_option) == "ReflectapiOption(42)"

    def test_boolean_conversion(self):
        """Test boolean conversion."""
        assert not ReflectapiOption(Undefined)
        assert not ReflectapiOption(None)
        assert not ReflectapiOption(0)
        assert not ReflectapiOption("")
        assert not ReflectapiOption([])
        assert ReflectapiOption(42)
        assert ReflectapiOption("hello")
        assert ReflectapiOption([1, 2, 3])

    def test_hash(self):
        """Test hash functionality."""
        option1 = ReflectapiOption(42)
        option2 = ReflectapiOption(42)
        option3 = ReflectapiOption(99)

        assert hash(option1) == hash(option2)
        assert hash(option1) != hash(option3)

        # Test that options can be used in sets
        option_set = {option1, option2, option3}
        assert len(option_set) == 2  # option1 and option2 are equal


class TestConvenienceFunctions:
    """Test convenience functions."""

    def test_some_function(self):
        """Test some() function."""
        option = some(42)
        assert option.is_some
        assert option.unwrap() == 42

    def test_none_function(self):
        """Test none() function."""
        option = none()
        assert option.is_none
        assert option.value is None

    def test_undefined_function(self):
        """Test undefined() function."""
        option = undefined()
        assert option.is_undefined


class TestSerializeOptionDict:
    """Test serialize_option_dict function."""

    def test_simple_dict_no_options(self):
        """Test serialization of dict without options."""
        data = {"name": "test", "value": 42}
        result = serialize_option_dict(data)

        assert result == data

    def test_simple_dict_with_undefined(self):
        """Test serialization excluding undefined options."""
        data = {"name": "test", "age": ReflectapiOption(Undefined), "value": 42}
        result = serialize_option_dict(data)

        expected = {"name": "test", "value": 42}
        assert result == expected

    def test_simple_dict_with_none(self):
        """Test serialization including None options."""
        data = {"name": "test", "age": ReflectapiOption(None), "value": 42}
        result = serialize_option_dict(data)

        expected = {"name": "test", "age": None, "value": 42}
        assert result == expected

    def test_simple_dict_with_some(self):
        """Test serialization including Some options."""
        data = {"name": "test", "age": ReflectapiOption(25), "value": 42}
        result = serialize_option_dict(data)

        expected = {"name": "test", "age": 25, "value": 42}
        assert result == expected

    def test_nested_dict(self):
        """Test serialization of nested dictionaries."""
        data = {
            "user": {
                "name": "test",
                "age": ReflectapiOption(25),
                "email": ReflectapiOption(Undefined),
            },
            "metadata": {"version": ReflectapiOption(None), "enabled": True},
        }
        result = serialize_option_dict(data)

        expected = {
            "user": {"name": "test", "age": 25},
            "metadata": {"version": None, "enabled": True},
        }
        assert result == expected

    def test_list_with_options(self):
        """Test serialization of lists containing options."""
        data = {
            "items": [
                ReflectapiOption(1),
                ReflectapiOption(None),
                ReflectapiOption(Undefined),
                ReflectapiOption(3),
                42,  # Non-option value
            ]
        }
        result = serialize_option_dict(data)

        expected = {
            "items": [1, None, 3, 42]  # Undefined option filtered out
        }
        assert result == expected

    def test_complex_nested_structure(self):
        """Test serialization of complex nested structure."""
        data = {
            "users": [
                {
                    "name": "Alice",
                    "age": ReflectapiOption(30),
                    "email": ReflectapiOption(Undefined),
                },
                {
                    "name": "Bob",
                    "age": ReflectapiOption(None),
                    "email": "bob@example.com",
                },
            ],
            "config": {
                "debug": ReflectapiOption(True),
                "timeout": ReflectapiOption(Undefined),
                "retries": 3,
            },
        }
        result = serialize_option_dict(data)

        expected = {
            "users": [
                {"name": "Alice", "age": 30},
                {"name": "Bob", "age": None, "email": "bob@example.com"},
            ],
            "config": {"debug": True, "retries": 3},
        }
        assert result == expected


class TestPydanticIntegration:
    """Test integration with Pydantic models."""

    def test_pydantic_model_with_options(self):
        """Test Option handling with Pydantic models."""

        class UserModel(BaseModel):
            name: str
            age: ReflectapiOption[int] = ReflectapiOption(Undefined)
            email: ReflectapiOption[str] = ReflectapiOption(Undefined)

        # Test model creation
        user = UserModel(name="Alice", age=some(30), email=none())

        assert user.name == "Alice"
        assert user.age.is_some
        assert user.age.unwrap() == 30
        assert user.email.is_none

        # Test serialization
        model_dict = user.model_dump() if hasattr(user, "model_dump") else user.dict()
        processed_dict = serialize_option_dict(model_dict)

        expected = {"name": "Alice", "age": 30, "email": None}
        assert processed_dict == expected

    def test_pydantic_model_with_undefined_fields(self):
        """Test Pydantic model with undefined fields."""

        class UpdateRequest(BaseModel):
            name: str
            age: ReflectapiOption[int] = ReflectapiOption(Undefined)
            email: ReflectapiOption[str] = ReflectapiOption(Undefined)

        # Only provide name, leave others undefined
        request = UpdateRequest(name="Alice")

        assert request.name == "Alice"
        assert request.age.is_undefined
        assert request.email.is_undefined

        # Serialization should exclude undefined fields
        model_dict = (
            request.model_dump() if hasattr(request, "model_dump") else request.dict()
        )
        processed_dict = serialize_option_dict(model_dict)

        expected = {"name": "Alice"}
        assert processed_dict == expected
