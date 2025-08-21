"""Option type handling for reflectapi code generation.

This module provides proper handling of Rust's Option<T> types in Python,
distinguishing between undefined, null, and actual values.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Generic, TypeVar

if TYPE_CHECKING:
    from pydantic import GetCoreSchemaHandler

from pydantic_core import core_schema

T = TypeVar("T")


class _UndefinedType:
    """Sentinel type for undefined values in reflectapi Option types.

    This represents the state where a field was not provided at all,
    as opposed to being explicitly set to None/null.
    """

    def __repr__(self) -> str:
        return "Undefined"

    def __str__(self) -> str:
        return "Undefined"

    def __bool__(self) -> bool:
        return False

    def __eq__(self, other: Any) -> bool:
        return isinstance(other, _UndefinedType)

    def __hash__(self) -> int:
        return hash("_UndefinedType")


# Global singleton instance
Undefined = _UndefinedType()


class ReflectapiOption(Generic[T]):
    """Proper representation of Rust's Option<T> type in Python.

    This class encapsulates the three possible states:
    - Undefined: Field was not provided
    - None: Field was explicitly set to null
    - Some(value): Field has an actual value

    Example:
        ```python
        # Field not provided
        option = ReflectapiOption()  # or ReflectapiOption(Undefined)

        # Field explicitly set to null
        option = ReflectapiOption(None)

        # Field has a value
        option = ReflectapiOption(42)
        ```
    """

    def __init__(self, value: T | None | _UndefinedType = Undefined):
        self._value = value

    @classmethod
    def __get_pydantic_core_schema__(
        cls, source_type: Any, handler: GetCoreSchemaHandler
    ) -> core_schema.CoreSchema:
        """Generate Pydantic V2 core schema for ReflectapiOption."""
        from typing import get_args, get_origin

        # Extract the generic type argument if available
        origin = get_origin(source_type)
        args = get_args(source_type)

        def validate_option(value: Any) -> ReflectapiOption[Any]:
            if isinstance(value, cls):
                return value
            return cls(value)

        def serialize_option(option_value: ReflectapiOption[Any]) -> Any:
            """Serialize ReflectapiOption handling all three states correctly."""
            if isinstance(option_value, cls):
                if option_value.is_undefined:
                    # Return None for undefined to avoid pydantic undefined serialization issues
                    return None
                # Return the actual value (including None for explicit null)
                return option_value._value
            # Fallback for non-ReflectapiOption values
            return option_value

        if origin is cls and args:
            # We have ReflectapiOption[SomeType]
            inner_type = args[0]
            inner_schema = handler(inner_type)

            return core_schema.no_info_plain_validator_function(
                validate_option,
                serialization=core_schema.plain_serializer_function_ser_schema(
                    serialize_option,
                    return_schema=core_schema.union_schema([
                        inner_schema,
                        core_schema.none_schema(),
                    ]),
                    when_used='json',
                )
            )
        else:
            # Fallback for untyped ReflectapiOption
            return core_schema.no_info_plain_validator_function(validate_option)


    @property
    def is_undefined(self) -> bool:
        """Check if the option is undefined (field not provided)."""
        return self._value is Undefined

    @property
    def is_none(self) -> bool:
        """Check if the option is explicitly None/null."""
        return self._value is None

    @property
    def is_some(self) -> bool:
        """Check if the option has a value (not undefined and not None)."""
        return self._value is not Undefined and self._value is not None

    @property
    def value(self) -> T | None:
        """Get the wrapped value.

        Returns:
            The wrapped value, or None if undefined or null.

        Raises:
            ValueError: If trying to access value when undefined.
        """
        if self.is_undefined:
            raise ValueError("Cannot access value of undefined option")
        return self._value

    def unwrap(self) -> T:
        """Unwrap the option, returning the value or raising an error.

        Returns:
            The wrapped value.

        Raises:
            ValueError: If the option is undefined or None.
        """
        if self.is_undefined:
            raise ValueError("Cannot unwrap undefined option")
        if self.is_none:
            raise ValueError("Cannot unwrap None option")
        return self._value

    def unwrap_or(self, default: T) -> T:
        """Unwrap the option or return a default value.

        Args:
            default: Default value to return if undefined or None.

        Returns:
            The wrapped value or the default.
        """
        if self.is_some:
            return self._value
        return default

    def map(self, func: callable) -> ReflectapiOption:
        """Apply a function to the wrapped value if it exists.

        Args:
            func: Function to apply to the value.

        Returns:
            New ReflectapiOption with the result, or unchanged if undefined/None.
        """
        if self.is_some:
            return ReflectapiOption(func(self._value))
        return ReflectapiOption(self._value)

    def filter(self, predicate: callable) -> ReflectapiOption:
        """Filter the option based on a predicate.

        Args:
            predicate: Function that returns True to keep the value.

        Returns:
            The option if predicate returns True, otherwise undefined.
        """
        if self.is_some and predicate(self._value):
            return self
        return ReflectapiOption(Undefined)

    def __eq__(self, other: Any) -> bool:
        if isinstance(other, ReflectapiOption):
            return self._value == other._value
        return self._value == other

    def __hash__(self) -> int:
        return hash(self._value)

    def __repr__(self) -> str:
        if self.is_undefined:
            return "ReflectapiOption(Undefined)"
        elif self.is_none:
            return "ReflectapiOption(None)"
        else:
            return f"ReflectapiOption({self._value!r})"

    def __str__(self) -> str:
        if self.is_undefined:
            return "Undefined"
        elif self.is_none:
            return "None"
        else:
            return str(self._value)

    def __bool__(self) -> bool:
        """Return True if the option has a truthy value."""
        return self.is_some and bool(self._value)


# Type alias for more concise usage
Option = ReflectapiOption


def some(value: T) -> ReflectapiOption[T]:
    """Create an Option with a value."""
    return ReflectapiOption(value)


def none() -> ReflectapiOption[None]:
    """Create an Option with None."""
    return ReflectapiOption(None)


def undefined() -> ReflectapiOption:
    """Create an undefined Option."""
    return ReflectapiOption(Undefined)


# Utility functions for serialization
def serialize_option_dict(data: dict) -> dict:
    """Serialize a dictionary, excluding undefined option fields.

    This is used in client serialization to properly handle Option types
    by excluding undefined fields from the JSON payload.

    Args:
        data: Dictionary that may contain ReflectapiOption values.

    Returns:
        Dictionary with undefined options excluded and others unwrapped.
    """
    result = {}

    for key, value in data.items():
        if isinstance(value, ReflectapiOption):
            if not value.is_undefined:
                # Include None values but not undefined ones
                result[key] = value._value
        elif isinstance(value, dict):
            # Recursively handle nested dictionaries
            result[key] = serialize_option_dict(value)
        elif isinstance(value, list):
            # Handle lists that might contain options or nested structures
            processed_items = []
            for item in value:
                if isinstance(item, ReflectapiOption):
                    if not item.is_undefined:
                        processed_items.append(item._value)
                elif isinstance(item, dict):
                    # Recursively handle dictionaries within lists
                    processed_items.append(serialize_option_dict(item))
                else:
                    processed_items.append(item)
            result[key] = processed_items
        else:
            result[key] = value

    return result


def is_undefined(value: Any) -> bool:
    """Check if a value is undefined."""
    if isinstance(value, ReflectapiOption):
        return value.is_undefined
    return value is Undefined
