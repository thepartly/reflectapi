"""Tests for ApiResponse and TransportMetadata classes."""

import time
from unittest.mock import Mock

import httpx
import pytest

from reflectapi_runtime import ApiResponse, TransportMetadata


class TestTransportMetadata:
    """Test TransportMetadata class."""

    def test_creation(self):
        """Test TransportMetadata creation."""
        headers = httpx.Headers({"content-type": "application/json"})
        mock_response = Mock(spec=httpx.Response)

        metadata = TransportMetadata(
            status_code=200, headers=headers, timing=0.5, raw_response=mock_response
        )

        assert metadata.status_code == 200
        assert metadata.headers == headers
        assert metadata.timing == 0.5
        assert metadata.raw_response is mock_response

    def test_from_response(self):
        """Test creating TransportMetadata from httpx Response."""
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 201
        mock_response.headers = httpx.Headers({"x-custom": "value"})

        start_time = time.time()
        # Simulate some processing time
        time.sleep(0.01)

        metadata = TransportMetadata.from_response(mock_response, start_time)

        assert metadata.status_code == 201
        assert metadata.headers["x-custom"] == "value"
        assert metadata.timing > 0.01  # Should be at least the sleep time
        assert metadata.raw_response is mock_response

    def test_immutability(self):
        """Test that TransportMetadata is immutable."""
        headers = httpx.Headers({})
        mock_response = Mock(spec=httpx.Response)

        metadata = TransportMetadata(
            status_code=200, headers=headers, timing=0.1, raw_response=mock_response
        )

        # Should not be able to modify fields (frozen dataclass)
        with pytest.raises((AttributeError, TypeError)):
            metadata.status_code = 404  # type: ignore[misc]


class TestApiResponse:
    """Test ApiResponse class."""

    def test_creation(self):
        """Test ApiResponse creation."""
        value = {"test": "data"}
        metadata = TransportMetadata(
            status_code=200,
            headers=httpx.Headers({}),
            timing=0.1,
            raw_response=Mock(spec=httpx.Response),
        )

        response = ApiResponse(value, metadata)

        assert response.value == value
        assert response.metadata is metadata



    def test_repr(self):
        """Test string representation."""
        value = {"test": "data"}
        metadata = TransportMetadata(
            status_code=201,
            headers=httpx.Headers({}),
            timing=0.1,
            raw_response=Mock(spec=httpx.Response),
        )

        response = ApiResponse(value, metadata)
        repr_str = repr(response)

        assert "ApiResponse" in repr_str
        assert "201" in repr_str
        assert "test" in repr_str

    def test_generic_typing(self):
        """Test that ApiResponse works with generic typing."""

        value = [1, 2, 3]
        metadata = TransportMetadata(
            status_code=200,
            headers=httpx.Headers({}),
            timing=0.1,
            raw_response=Mock(spec=httpx.Response),
        )

        response: ApiResponse[list[int]] = ApiResponse(value, metadata)

        assert response.value == [1, 2, 3]
        assert len(response) == 3  # Should delegate to list.__len__
        assert response[0] == 1  # Should delegate to list.__getitem__

    def test_delegation_methods_with_unsupported_types(self):
        """Test delegation methods with types that don't support them."""

        # Use a simple object that doesn't support len, contains, or getitem
        class SimpleObject:
            def __init__(self, value):
                self.value = value

        value = SimpleObject("test")
        metadata = TransportMetadata(
            status_code=200,
            headers=httpx.Headers({}),
            timing=0.1,
            raw_response=Mock(spec=httpx.Response),
        )

        response = ApiResponse(value, metadata)

        # Test __contains__ returns False for unsupported types
        assert ("test" in response) is False

        # Test __len__ raises TypeError for unsupported types
        with pytest.raises(TypeError, match="has no len"):
            len(response)

        # Test __getitem__ raises TypeError for unsupported types
        with pytest.raises(TypeError, match="not subscriptable"):
            response[0]

    def test_explicit_value_access_preferred(self):
        """Test the preferred explicit .value access pattern."""
        
        class MockObject:
            def __init__(self, name: str, count: int):
                self.name = name
                self.count = count

            def get_info(self) -> str:
                return f"{self.name}:{self.count}"

        value = MockObject("test", 42)
        metadata = TransportMetadata(
            status_code=200,
            headers=httpx.Headers({}),
            timing=0.1,
            raw_response=Mock(spec=httpx.Response),
        )

        response = ApiResponse(value, metadata)

        # Preferred: Explicit access via .value (no deprecation warning)
        assert response.value.name == "test"
        assert response.value.count == 42
        assert response.value.get_info() == "test:42"

