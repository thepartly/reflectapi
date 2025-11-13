"""Hypothesis strategies for generated Pydantic models.

This module provides utilities to automatically generate Hypothesis strategies
for ReflectAPI-generated Pydantic models, enabling property-based testing.
"""

from __future__ import annotations

import datetime
import uuid
from typing import Any, Union, get_args, get_origin

try:
    import hypothesis
    from hypothesis import strategies as st

    HAS_HYPOTHESIS = True
except ImportError:
    HAS_HYPOTHESIS = False

from pydantic import BaseModel

from .option import ReflectapiOption, Undefined

if HAS_HYPOTHESIS:

    def strategy_for_option(inner_strategy: st.SearchStrategy) -> st.SearchStrategy:
        """Create a strategy for ReflectapiOption types.

        Args:
            inner_strategy: Strategy for the inner type T in Option<T>.

        Returns:
            Strategy that produces ReflectapiOption instances with undefined, None, or values.
        """
        return st.one_of(
            st.just(ReflectapiOption(Undefined)),  # Undefined case
            st.just(ReflectapiOption(None)),  # None case
            inner_strategy.map(ReflectapiOption),  # Some(value) case
        )

    def strategy_for_type(type_hint: type) -> st.SearchStrategy:
        """Generate a Hypothesis strategy for a given type hint.

        This function maps Python types to appropriate Hypothesis strategies,
        with special handling for ReflectAPI types.

        Args:
            type_hint: The type to generate a strategy for.

        Returns:
            Hypothesis strategy for the given type.
        """
        # Handle basic types
        if type_hint is str:
            return st.text()
        elif type_hint is int:
            return st.integers()
        elif type_hint is float:
            return st.floats(allow_nan=False, allow_infinity=False)
        elif type_hint is bool:
            return st.booleans()
        elif type_hint is bytes:
            return st.binary()
        elif type_hint is datetime.datetime:
            return st.datetimes()
        elif type_hint is datetime.date:
            return st.dates()
        elif type_hint is uuid.UUID:
            return st.uuids()

        # Handle ReflectapiOption specifically
        if (
            hasattr(type_hint, "__origin__")
            and type_hint.__origin__ is ReflectapiOption
        ):
            inner_type = get_args(type_hint)[0] if get_args(type_hint) else Any
            inner_strategy = strategy_for_type(inner_type)
            return strategy_for_option(inner_strategy)

        # Handle generic types
        origin = get_origin(type_hint)
        args = get_args(type_hint)

        if origin is Union:
            # Handle Optional[T] (Union[T, None]) and other unions
            if len(args) == 2 and type(None) in args:
                # Optional type
                non_none_type = args[0] if args[1] is type(None) else args[1]
                return st.one_of(st.none(), strategy_for_type(non_none_type))
            else:
                # General union
                return st.one_of(*[strategy_for_type(arg) for arg in args])

        elif origin is list:
            inner_type = args[0] if args else Any
            return st.lists(strategy_for_type(inner_type), max_size=10)

        elif origin is dict:
            key_type = args[0] if len(args) > 0 else str
            value_type = args[1] if len(args) > 1 else Any
            return st.dictionaries(
                strategy_for_type(key_type), strategy_for_type(value_type), max_size=10
            )

        elif origin is tuple:
            if args:
                return st.tuples(*[strategy_for_type(arg) for arg in args])
            else:
                return st.tuples()

        # Handle Pydantic models
        if isinstance(type_hint, type) and issubclass(type_hint, BaseModel):
            return strategy_for_pydantic_model(type_hint)

        # Fallback for unknown types
        return st.none()

    def strategy_for_pydantic_model(model_class: type[BaseModel]) -> st.SearchStrategy:
        """Generate a Hypothesis strategy for a Pydantic model.

        This creates a strategy that generates valid instances of the given
        Pydantic model class, respecting field types and constraints.

        Args:
            model_class: The Pydantic model class to generate strategies for.

        Returns:
            Strategy that produces instances of the model class.
        """
        field_strategies = {}

        # Handle Pydantic models
        if hasattr(model_class, "model_fields"):
            for field_name, field_info in model_class.model_fields.items():
                field_type = field_info.annotation
                field_strategies[field_name] = strategy_for_type(field_type)
        else:
            raise ValueError(
                f"Unsupported Pydantic model: {model_class}. Only Pydantic V2 models are supported."
            )

        # Create a strategy that builds the model
        return st.builds(model_class, **field_strategies)

    def register_custom_strategy(type_hint: type, strategy: st.SearchStrategy) -> None:
        """Register a custom strategy for a specific type.

        This allows users to override the default strategy generation
        for specific types or add support for custom types.

        Args:
            type_hint: The type to register a strategy for.
            strategy: The Hypothesis strategy to use for this type.
        """
        if not hasattr(register_custom_strategy, "_custom_strategies"):
            register_custom_strategy._custom_strategies = {}
        register_custom_strategy._custom_strategies[type_hint] = strategy

    def get_custom_strategy(type_hint: type) -> st.SearchStrategy | None:
        """Get a custom strategy for a type, if registered.

        Args:
            type_hint: The type to look up.

        Returns:
            Custom strategy if registered, None otherwise.
        """
        if hasattr(register_custom_strategy, "_custom_strategies"):
            return register_custom_strategy._custom_strategies.get(type_hint)
        return None

    def enhanced_strategy_for_type(type_hint: type) -> st.SearchStrategy:
        """Enhanced strategy generation with custom strategy support.

        This is the main entry point that checks for custom strategies
        before falling back to the default generation logic.

        Args:
            type_hint: The type to generate a strategy for.

        Returns:
            Hypothesis strategy for the given type.
        """
        # Check for custom strategies first
        custom_strategy = get_custom_strategy(type_hint)
        if custom_strategy is not None:
            return custom_strategy

        # Fall back to default logic
        return strategy_for_type(type_hint)

    # Convenience function for common patterns
    def api_model_strategy(
        model_class: type[BaseModel], **field_overrides: st.SearchStrategy
    ) -> st.SearchStrategy:
        """Create a strategy for an API model with field overrides.

        This is useful when you want to use the default strategy for most fields
        but customize specific fields for testing scenarios.

        Args:
            model_class: The Pydantic model class.
            **field_overrides: Custom strategies for specific fields.

        Returns:
            Strategy for the model with custom field strategies applied.

        Example:
            ```python
            # Override the 'id' field to use positive integers
            strategy = api_model_strategy(
                UserModel,
                id=st.integers(min_value=1),
                email=st.emails()
            )
            ```
        """
        field_strategies = {}

        # Get default strategies for all fields
        if hasattr(model_class, "model_fields"):
            # Pydantic models
            for field_name, field_info in model_class.model_fields.items():
                if field_name in field_overrides:
                    field_strategies[field_name] = field_overrides[field_name]
                else:
                    field_strategies[field_name] = enhanced_strategy_for_type(
                        field_info.annotation
                    )
        else:
            raise ValueError(
                f"Unsupported Pydantic model: {model_class}. Only Pydantic V2 models are supported."
            )

        return st.builds(model_class, **field_strategies)


else:
    # Hypothesis not available - provide stub implementations

    def strategy_for_option(inner_strategy):
        """Stub implementation when Hypothesis is not available."""
        raise ImportError("Hypothesis is required for strategy generation")

    def strategy_for_type(type_hint):
        """Stub implementation when Hypothesis is not available."""
        raise ImportError("Hypothesis is required for strategy generation")

    def strategy_for_pydantic_model(model_class):
        """Stub implementation when Hypothesis is not available."""
        raise ImportError("Hypothesis is required for strategy generation")

    def register_custom_strategy(type_hint, strategy):
        """Stub implementation when Hypothesis is not available."""
        raise ImportError("Hypothesis is required for strategy generation")

    def enhanced_strategy_for_type(type_hint):
        """Stub implementation when Hypothesis is not available."""
        raise ImportError("Hypothesis is required for strategy generation")

    def api_model_strategy(model_class, **field_overrides):
        """Stub implementation when Hypothesis is not available."""
        raise ImportError("Hypothesis is required for strategy generation")


# Export the main functions
__all__ = [
    "HAS_HYPOTHESIS",
    "strategy_for_option",
    "strategy_for_type",
    "strategy_for_pydantic_model",
    "enhanced_strategy_for_type",
    "register_custom_strategy",
    "api_model_strategy",
]
