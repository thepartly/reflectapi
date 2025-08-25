"""Exception hierarchy for ReflectAPI Python clients."""

from __future__ import annotations

from typing import Any

import httpx  # noqa: TC002

from .response import TransportMetadata  # noqa: TC001


class ApiError(Exception):
    """Base class for all API-related errors.

    Always contains transport metadata when available, providing access to
    status codes, headers, timing, and the raw response.
    """

    def __init__(
        self,
        message: str,
        *,
        metadata: TransportMetadata | None = None,
        cause: Exception | None = None,
    ) -> None:
        super().__init__(message)
        self.metadata = metadata
        self.cause = cause

    @property
    def status_code(self) -> int | None:
        """HTTP status code if available."""
        return self.metadata.status_code if self.metadata else None

    def __repr__(self) -> str:
        parts = [f"message={self.args[0]!r}"]
        if self.metadata:
            parts.append(f"status_code={self.metadata.status_code}")
        if self.cause:
            parts.append(f"cause={self.cause!r}")
        return f"{self.__class__.__name__}({', '.join(parts)})"


class NetworkError(ApiError):
    """Network-level errors (connection failures, DNS resolution, etc.)."""

    @classmethod
    def from_httpx_error(cls, error: httpx.RequestError) -> NetworkError:
        """Create a NetworkError from an httpx RequestError."""
        return cls(
            f"Network error: {error}",
            cause=error,
        )


class TimeoutError(NetworkError):
    """Request timeout errors."""

    @classmethod
    def from_httpx_timeout(cls, error: httpx.TimeoutException) -> TimeoutError:
        """Create a TimeoutError from an httpx TimeoutException."""
        return cls(
            f"Request timed out: {error}",
            cause=error,
        )


class ApplicationError(ApiError):
    """Application-level errors (4xx and 5xx HTTP responses).

    These represent errors returned by the API server, as opposed to
    network or client-side validation errors.
    """

    def __init__(
        self,
        message: str,
        *,
        metadata: TransportMetadata,
        error_data: Any | None = None,
    ) -> None:
        super().__init__(message, metadata=metadata)
        self.error_data = error_data

    @property
    def status_code(self) -> int:
        """Get the HTTP status code from metadata."""
        return self.metadata.status_code if self.metadata else 0

    @classmethod
    def from_response(
        cls,
        response: httpx.Response,
        metadata: TransportMetadata,
        error_data: Any | None = None,
    ) -> ApplicationError:
        """Create an ApplicationError from an HTTP response."""
        message = f"API error {response.status_code}: {response.reason_phrase}"
        if error_data:
            message += f" - {error_data}"

        return cls(
            message,
            metadata=metadata,
            error_data=error_data,
        )


class ValidationError(ApiError):
    """Client-side validation errors (malformed requests, invalid data, etc.)."""

    def __init__(
        self,
        message: str,
        *,
        validation_errors: list[Any] | None = None,
        cause: Exception | None = None,
    ) -> None:
        super().__init__(message, cause=cause)
        self.validation_errors = validation_errors or []
