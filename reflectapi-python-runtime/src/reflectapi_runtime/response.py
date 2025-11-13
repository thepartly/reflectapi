"""Response and metadata classes for ReflectAPI Python clients."""

from __future__ import annotations

import time
from dataclasses import dataclass
from typing import Any, Generic, TypeVar

import httpx  # noqa: TC002

T = TypeVar("T")


@dataclass(frozen=True)
class TransportMetadata:
    """Immutable metadata about an HTTP response.

    Contains timing information, HTTP status, headers, and the raw response object.
    """

    status_code: int
    headers: httpx.Headers
    timing: float  # Request duration in seconds
    raw_response: httpx.Response

    @classmethod
    def from_response(
        cls, response: httpx.Response, start_time: float
    ) -> TransportMetadata:
        """Create TransportMetadata from an httpx Response."""
        return cls(
            status_code=response.status_code,
            headers=response.headers,
            timing=time.time() - start_time,
            raw_response=response,
        )


class ApiResponse(Generic[T]):
    """Wrapper for successful API responses.

    Provides ergonomic access to both the deserialized value and transport metadata.
    The deserialized value can be accessed directly via attribute access.
    """

    def __init__(self, value: T, metadata: TransportMetadata) -> None:
        self._value = value
        self._metadata = metadata

    @property
    def value(self) -> T:
        """The deserialized response value."""
        return self._value

    @property
    def metadata(self) -> TransportMetadata:
        """Transport metadata including timing, headers, and status."""
        return self._metadata

    @property
    def data(self) -> T:
        """Alias for value for ergonomic access (useful when the payload is a dict)."""
        return self._value

    def __dir__(self) -> list[str]:
        """Provide comprehensive attribute listing for better introspection.

        This enhances IDE auto-completion and debugging by merging attributes
        from both the ApiResponse wrapper and the wrapped value.

        Returns:
            List of available attributes from both wrapper and value.
        """
        # Get ApiResponse's own attributes
        wrapper_attrs = ["value", "metadata", "data"]

        # Get attributes from the wrapped value
        value_attrs = []
        if hasattr(self._value, "__dict__"):
            value_attrs.extend(self._value.__dict__.keys())

        # For Pydantic models, also include field names
        if hasattr(self._value, "model_fields"):
            # Access from class to avoid deprecation warning
            value_attrs.extend(self._value.__class__.model_fields.keys())

        # For dict-like objects
        if isinstance(self._value, dict):
            value_attrs.extend(self._value.keys())

        # Get methods and properties from the wrapped value's class
        if hasattr(self._value, "__class__"):
            value_attrs.extend(
                [
                    attr
                    for attr in dir(self._value.__class__)
                    if not attr.startswith("__")
                    or attr in ["__len__", "__getitem__", "__contains__"]
                ]
            )

        # Combine and deduplicate
        all_attrs = list(set(wrapper_attrs + value_attrs))

        # Sort for consistent ordering
        return sorted(all_attrs)

    def __contains__(self, item: Any) -> bool:
        """Delegate containment checks to the wrapped value."""
        if hasattr(self._value, "__contains__"):
            return item in self._value
        return False

    def __len__(self) -> int:
        """Delegate length checks to the wrapped value."""
        if hasattr(self._value, "__len__"):
            return len(self._value)
        raise TypeError(f"object of type '{type(self._value).__name__}' has no len()")

    def __getitem__(self, key: Any) -> Any:
        """Delegate item access to the wrapped value."""
        if hasattr(self._value, "__getitem__"):
            return self._value[key]
        raise TypeError(f"'{type(self._value).__name__}' object is not subscriptable")

    def __repr__(self) -> str:
        return (
            f"ApiResponse(value={self._value!r}, status={self._metadata.status_code})"
        )
