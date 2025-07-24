"""Tests for exception classes."""

from unittest.mock import Mock

import httpx

from reflectapi_runtime import (
    ApiError,
    ApplicationError,
    NetworkError,
    TimeoutError,
    TransportMetadata,
    ValidationError,
)


class TestApiError:
    """Test base ApiError class."""

    def test_creation_basic(self):
        """Test basic ApiError creation."""
        error = ApiError("Something went wrong")

        assert str(error) == "Something went wrong"
        assert error.metadata is None
        assert error.cause is None
        assert error.status_code is None

    def test_creation_with_metadata(self):
        """Test ApiError creation with metadata."""
        metadata = TransportMetadata(
            status_code=400,
            headers=httpx.Headers({}),
            timing=0.1,
            raw_response=Mock(spec=httpx.Response),
        )

        error = ApiError("Bad request", metadata=metadata)

        assert error.metadata is metadata
        assert error.status_code == 400

    def test_creation_with_cause(self):
        """Test ApiError creation with cause."""
        cause = ValueError("Original error")
        error = ApiError("Wrapped error", cause=cause)

        assert error.cause is cause

    def test_repr(self):
        """Test string representation."""
        metadata = TransportMetadata(
            status_code=500,
            headers=httpx.Headers({}),
            timing=0.1,
            raw_response=Mock(spec=httpx.Response),
        )
        cause = ValueError("Original")

        error = ApiError("Test error", metadata=metadata, cause=cause)
        repr_str = repr(error)

        assert "ApiError" in repr_str
        assert "Test error" in repr_str
        assert "500" in repr_str
        assert "ValueError" in repr_str


class TestNetworkError:
    """Test NetworkError class."""

    def test_from_httpx_error(self):
        """Test creating NetworkError from httpx RequestError."""
        original_error = httpx.ConnectError("Connection failed")

        error = NetworkError.from_httpx_error(original_error)

        assert "Network error" in str(error)
        assert "Connection failed" in str(error)
        assert error.cause is original_error

    def test_inheritance(self):
        """Test that NetworkError inherits from ApiError."""
        error = NetworkError("Test network error")
        assert isinstance(error, ApiError)


class TestTimeoutError:
    """Test TimeoutError class."""

    def test_from_httpx_timeout(self):
        """Test creating TimeoutError from httpx TimeoutException."""
        original_error = httpx.TimeoutException("Request timed out")

        error = TimeoutError.from_httpx_timeout(original_error)

        assert "Request timed out" in str(error)
        assert error.cause is original_error

    def test_inheritance(self):
        """Test that TimeoutError inherits from NetworkError."""
        error = TimeoutError("Test timeout")
        assert isinstance(error, NetworkError)
        assert isinstance(error, ApiError)


class TestApplicationError:
    """Test ApplicationError class."""

    def test_creation(self):
        """Test ApplicationError creation."""
        metadata = TransportMetadata(
            status_code=404,
            headers=httpx.Headers({}),
            timing=0.1,
            raw_response=Mock(spec=httpx.Response),
        )
        error_data = {"code": "NOT_FOUND", "message": "Resource not found"}

        error = ApplicationError("Not found", metadata=metadata, error_data=error_data)

        assert str(error) == "Not found"
        assert error.metadata is metadata
        assert error.error_data == error_data
        assert error.status_code == 404

    def test_from_response(self):
        """Test creating ApplicationError from httpx Response."""
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 422
        mock_response.reason_phrase = "Unprocessable Entity"

        metadata = TransportMetadata(
            status_code=422,
            headers=httpx.Headers({}),
            timing=0.1,
            raw_response=mock_response,
        )

        error_data = {"validation_errors": ["field is required"]}

        error = ApplicationError.from_response(mock_response, metadata, error_data)

        assert "API error 422: Unprocessable Entity" in str(error)
        assert "validation_errors" in str(error)
        assert error.metadata is metadata
        assert error.error_data == error_data

    def test_inheritance(self):
        """Test that ApplicationError inherits from ApiError."""
        metadata = TransportMetadata(
            status_code=400,
            headers=httpx.Headers({}),
            timing=0.1,
            raw_response=Mock(spec=httpx.Response),
        )

        error = ApplicationError("Test error", metadata=metadata)
        assert isinstance(error, ApiError)


class TestValidationError:
    """Test ValidationError class."""

    def test_creation_basic(self):
        """Test basic ValidationError creation."""
        error = ValidationError("Invalid data")

        assert str(error) == "Invalid data"
        assert error.validation_errors == []

    def test_creation_with_validation_errors(self):
        """Test ValidationError creation with validation errors."""
        validation_errors = [
            {"field": "name", "message": "required"},
            {"field": "age", "message": "must be positive"},
        ]

        error = ValidationError(
            "Validation failed", validation_errors=validation_errors
        )

        assert error.validation_errors == validation_errors

    def test_creation_with_cause(self):
        """Test ValidationError creation with cause."""
        cause = ValueError("Pydantic validation error")
        error = ValidationError("Validation failed", cause=cause)

        assert error.cause is cause

    def test_inheritance(self):
        """Test that ValidationError inherits from ApiError."""
        error = ValidationError("Test validation error")
        assert isinstance(error, ApiError)
