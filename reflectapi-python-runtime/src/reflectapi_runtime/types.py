"""Common type definitions for ReflectAPI Python runtime."""

from __future__ import annotations

from typing import TYPE_CHECKING, TypeVar, Union

from pydantic import BaseModel, ConfigDict

if TYPE_CHECKING:
    from .exceptions import ApiError
    from .response import ApiResponse

T = TypeVar("T")

# Type alias for batch operation results - avoids circular imports
BatchResult = Union["ApiResponse[T]", "ApiError"]


class ReflectapiEmpty(BaseModel):
    """Struct object with no fields.

    This represents empty struct types from ReflectAPI schemas.
    """

    model_config = ConfigDict(extra="ignore")


class ReflectapiInfallible(BaseModel):
    """Error object which is expected to be never returned.

    This represents infallible error types that should never occur.
    """

    model_config = ConfigDict(extra="ignore")
