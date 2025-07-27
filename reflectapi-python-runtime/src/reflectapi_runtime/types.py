"""Common type definitions for ReflectAPI Python runtime."""

from __future__ import annotations

from typing import TYPE_CHECKING, TypeVar, Union

if TYPE_CHECKING:
    from .exceptions import ApiError
    from .response import ApiResponse

T = TypeVar("T")

# Type alias for batch operation results - avoids circular imports
BatchResult = Union["ApiResponse[T]", "ApiError"]
