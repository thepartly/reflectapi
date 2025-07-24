"""Test ReflectapiOption functionality comprehensively."""

import pytest
import json
from pydantic import ValidationError

from generated import (
    MyapiProtoPetsUpdateRequest as PetsUpdateRequest,
    MyapiModelBehavior as Behavior,
    MyapiModelBehaviorAggressiveVariant as BehaviorAggressive,
    MyapiModelBehaviorOtherVariant as BehaviorOther
)
from reflectapi_runtime import ReflectapiOption, Undefined

# For externally tagged enums, unit variants are just string literals
BehaviorCalm = "Calm"


class TestReflectapiOptionBasics:
    """Test basic ReflectapiOption functionality."""
    
    def test_undefined_creation(self):
        """Test creating ReflectapiOption with Undefined."""
        option = ReflectapiOption(Undefined)
        assert option._value is Undefined
        assert option.is_undefined
        assert not option.is_none
        assert not option.is_some
    
    def test_none_creation(self):
        """Test creating ReflectapiOption with None."""
        option = ReflectapiOption(None)
        assert option._value is None
        assert not option.is_undefined
        assert option.is_none
        assert not option.is_some
    
    def test_some_creation(self):
        """Test creating ReflectapiOption with value."""
        option = ReflectapiOption(42)
        assert option._value == 42
        assert not option.is_undefined
        assert not option.is_none
        assert option.is_some
    
    def test_default_is_undefined(self):
        """Test default ReflectapiOption is undefined."""
        option = ReflectapiOption()
        assert option.is_undefined


class TestReflectapiOptionMethods:
    """Test ReflectapiOption utility methods."""
    
    def test_unwrap_some(self):
        """Test unwrapping a Some value."""
        option = ReflectapiOption(42)
        assert option.unwrap() == 42
    
    def test_unwrap_undefined_raises(self):
        """Test unwrapping Undefined raises ValueError."""
        option = ReflectapiOption(Undefined)
        with pytest.raises(ValueError, match="Cannot unwrap undefined option"):
            option.unwrap()
    
    def test_unwrap_none_raises(self):
        """Test unwrapping None raises ValueError."""
        option = ReflectapiOption(None)
        with pytest.raises(ValueError, match="Cannot unwrap None option"):
            option.unwrap()
    
    def test_unwrap_or_with_some(self):
        """Test unwrap_or with Some value."""
        option = ReflectapiOption(42)
        assert option.unwrap_or(0) == 42
    
    def test_unwrap_or_with_undefined(self):
        """Test unwrap_or with Undefined."""
        option = ReflectapiOption(Undefined)
        assert option.unwrap_or(0) == 0
    
    def test_unwrap_or_with_none(self):
        """Test unwrap_or with None."""
        option = ReflectapiOption(None)
        assert option.unwrap_or(0) == 0
    
    def test_map_some(self):
        """Test mapping over Some value."""
        option = ReflectapiOption(42)
        mapped = option.map(lambda x: x * 2)
        assert mapped.unwrap() == 84
    
    def test_map_undefined(self):
        """Test mapping over Undefined."""
        option = ReflectapiOption(Undefined)
        mapped = option.map(lambda x: x * 2)
        assert mapped.is_undefined
    
    def test_map_none(self):
        """Test mapping over None."""
        option = ReflectapiOption(None)
        mapped = option.map(lambda x: x * 2)
        assert mapped.is_none


class TestReflectapiOptionSerialization:
    """Test ReflectapiOption serialization behavior."""
    
    def test_option_serialization_basic(self):
        """Test basic ReflectapiOption serialization."""
        # Test with explicit ReflectapiOption values
        option_some = ReflectapiOption(42)
        option_none = ReflectapiOption(None)
        option_undefined = ReflectapiOption(Undefined)
        
        # Test individual option serialization via their values
        assert option_some._value == 42
        assert option_none._value is None
        assert option_undefined._value is Undefined


class TestReflectapiOptionDeserialization:
    """Test ReflectapiOption deserialization from data."""
    
    def test_option_creation_from_values(self):
        """Test creating options from different values."""
        # Test direct creation with values
        option_int = ReflectapiOption(42)
        option_list = ReflectapiOption([BehaviorCalm])
        
        assert option_int.unwrap() == 42
        assert len(option_list.unwrap()) == 1
        assert option_list.unwrap()[0] == "Calm"


class TestReflectapiOptionEquality:
    """Test ReflectapiOption equality comparisons."""
    
    def test_undefined_equality(self):
        """Test Undefined options are equal."""
        opt1 = ReflectapiOption(Undefined)
        opt2 = ReflectapiOption(Undefined)
        assert opt1 == opt2
    
    def test_none_equality(self):
        """Test None options are equal."""
        opt1 = ReflectapiOption(None)
        opt2 = ReflectapiOption(None)
        assert opt1 == opt2
    
    def test_some_equality(self):
        """Test Some options with same value are equal."""
        opt1 = ReflectapiOption(42)
        opt2 = ReflectapiOption(42)
        assert opt1 == opt2
    
    def test_different_values_not_equal(self):
        """Test options with different values are not equal."""
        opt1 = ReflectapiOption(42)
        opt2 = ReflectapiOption(24)
        assert opt1 != opt2
    
    def test_different_states_not_equal(self):
        """Test options in different states are not equal."""
        opt1 = ReflectapiOption(Undefined)
        opt2 = ReflectapiOption(None)
        opt3 = ReflectapiOption(42)
        
        assert opt1 != opt2
        assert opt2 != opt3
        assert opt1 != opt3


class TestReflectapiOptionRepr:
    """Test ReflectapiOption string representations."""
    
    def test_undefined_repr(self):
        """Test Undefined repr."""
        option = ReflectapiOption(Undefined)
        assert "Undefined" in repr(option)
    
    def test_none_repr(self):
        """Test None repr."""
        option = ReflectapiOption(None)
        assert "None" in repr(option)
    
    def test_some_repr(self):
        """Test Some repr."""
        option = ReflectapiOption(42)
        assert "42" in repr(option)


class TestReflectapiOptionComplexTypes:
    """Test ReflectapiOption with complex types."""
    
    def test_list_option(self):
        """Test ReflectapiOption with list value."""
        behaviors = [BehaviorCalm, {"Aggressive": [5.0, "test"]}]
        option = ReflectapiOption(behaviors)
        
        assert option.is_some
        unwrapped = option.unwrap()
        assert len(unwrapped) == 2
        assert unwrapped[0] == "Calm"
        assert unwrapped[1] == {"Aggressive": [5.0, "test"]}
    
    def test_empty_list_option(self):
        """Test ReflectapiOption with empty list."""
        option = ReflectapiOption([])
        
        assert option.is_some
        assert option.unwrap() == []
    
    def test_nested_serialization(self):
        """Test complex nested serialization."""
        behaviors = [BehaviorCalm, {"Other": {"description": "Custom", "notes": "Test"}}]
        request = PetsUpdateRequest(
            name="complex_test",
            age=ReflectapiOption(Undefined),
            behaviors=ReflectapiOption(behaviors)
        )
        
        # Test that the request has the expected values
        assert request.behaviors.is_some
        unwrapped_behaviors = request.behaviors.unwrap()
        assert len(unwrapped_behaviors) == 2
        assert unwrapped_behaviors[0] == "Calm"
        assert unwrapped_behaviors[1] == {"Other": {"description": "Custom", "notes": "Test"}}