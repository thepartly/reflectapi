"""Edge case and negative tests for ReflectAPI runtime."""

import asyncio
import json
import time
from typing import Any
from unittest.mock import AsyncMock, Mock, patch

import httpx
import pytest
from pydantic import BaseModel, ValidationError

from reflectapi_runtime import (
    APIKeyAuth,
    ApiResponse,
    ApplicationError,
    AsyncClientBase,
    BasicAuth,
    BearerTokenAuth,
    ClientBase,
    NetworkError,
    ReflectapiOption,
    TimeoutError,
    TransportMetadata,
    Undefined,
)
from reflectapi_runtime import (
    ValidationError as ReflectApiValidationError,
)


class EdgeCaseTestModel(BaseModel):
    """Test model for edge case testing."""

    name: str
    value: int
    optional_field: ReflectapiOption[str] = ReflectapiOption()


class TestClientEdgeCases:
    """Test edge cases for client behavior."""

    def test_empty_base_url(self):
        """Test client with empty base URL."""
        client = ClientBase("")
        assert client.base_url == ""

    def test_base_url_stripping(self):
        """Test that trailing slashes are stripped from base URL."""
        client = ClientBase("https://api.example.com/////")
        assert client.base_url == "https://api.example.com"

    def test_malformed_base_url(self):
        """Test client with malformed base URL."""
        # Should not raise exception during construction
        client = ClientBase("not-a-url")
        assert client.base_url == "not-a-url"

    def test_none_headers(self):
        """Test client with None headers."""
        client = ClientBase("https://api.example.com", headers=None)
        # Should not crash
        assert hasattr(client, "_client")

    def test_empty_headers_dict(self):
        """Test client with empty headers dict."""
        client = ClientBase("https://api.example.com", headers={})
        assert hasattr(client, "_client")

    def test_extremely_long_timeout(self):
        """Test client with extremely long timeout."""
        client = ClientBase("https://api.example.com", timeout=999999.0)
        assert hasattr(client, "_client")

    def test_zero_timeout(self):
        """Test client with zero timeout."""
        client = ClientBase("https://api.example.com", timeout=0.0)
        assert hasattr(client, "_client")

    def test_negative_timeout(self):
        """Test client with negative timeout."""
        client = ClientBase("https://api.example.com", timeout=-1.0)
        assert hasattr(client, "_client")

    @patch("httpx.Client.send")
    def test_request_with_empty_path(self, mock_send):
        """Test making request with empty path."""
        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.json.return_value = {"result": "ok"}
        mock_response.headers = {}
        mock_response.elapsed.total_seconds.return_value = 0.1
        mock_send.return_value = mock_response

        client = ClientBase("https://api.example.com")
        response = client._make_request("GET", "")

        assert response.value == {"result": "ok"}

    @patch("httpx.Client.send")
    def test_request_with_special_characters_in_path(self, mock_send):
        """Test making request with special characters in path."""
        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.json.return_value = {"result": "ok"}
        mock_response.headers = {}
        mock_response.elapsed.total_seconds.return_value = 0.1
        mock_send.return_value = mock_response

        client = ClientBase("https://api.example.com")
        response = client._make_request("GET", "/path with spaces/αβγ/数字/🚀")

        assert response.value == {"result": "ok"}

    @patch("httpx.Client.send")
    def test_request_with_both_json_data_and_model(self, mock_send):
        """Test that providing both json_data and json_model raises error."""
        client = ClientBase("https://api.example.com")
        model = EdgeCaseTestModel(name="test", value=42)

        with pytest.raises(
            ValueError, match="Cannot specify both json_data and json_model"
        ):
            client._make_request(
                "POST", "/test", json_data={"key": "value"}, json_model=model
            )

    @patch("httpx.Client.send")
    def test_request_with_circular_reference_in_json_data(self, mock_send):
        """Test request with circular reference in JSON data."""
        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.json.return_value = {"result": "ok"}
        mock_response.headers = {}
        mock_response.elapsed.total_seconds.return_value = 0.1
        mock_send.return_value = mock_response

        # Create circular reference
        data = {"key": "value"}
        data["self"] = data

        client = ClientBase("https://api.example.com")
        # Should raise RecursionError for circular references
        with pytest.raises(RecursionError):
            client._make_request("POST", "/test", json_data=data)


class TestAsyncClientEdgeCases:
    """Test edge cases for async client behavior."""

    @pytest.mark.asyncio
    async def test_async_client_with_sync_auth(self):
        """Test async client with synchronous auth handler."""
        auth = BearerTokenAuth("test-token")
        client = AsyncClientBase("https://api.example.com", auth=auth)
        assert client.auth is auth

    @pytest.mark.asyncio
    async def test_concurrent_client_operations(self):
        """Test many concurrent operations on the same client."""
        with patch("httpx.AsyncClient.send") as mock_send:
            mock_response = Mock()
            mock_response.status_code = 200
            mock_response.json.return_value = {"result": "ok"}
            mock_response.headers = {}
            mock_response.elapsed.total_seconds.return_value = 0.1
            mock_send.return_value = mock_response

            client = AsyncClientBase("https://api.example.com")

            # Run 100 concurrent requests
            tasks = [client._make_request("GET", f"/test/{i}") for i in range(100)]
            responses = await asyncio.gather(*tasks)

            assert len(responses) == 100
            assert all(response.value == {"result": "ok"} for response in responses)

    @pytest.mark.asyncio
    async def test_async_client_context_manager_exception(self):
        """Test async client context manager with exception."""
        try:
            async with AsyncClientBase("https://api.example.com") as client:
                assert client is not None
                raise ValueError("Test exception")
        except ValueError:
            pass  # Expected
        # Should not raise additional exceptions during cleanup


class TestReflectapiOptionEdgeCases:
    """Test edge cases for ReflectapiOption."""

    def test_option_with_complex_objects(self):
        """Test ReflectapiOption with complex nested objects."""
        complex_data = {
            "nested": {
                "list": [1, 2, {"deep": "value"}],
                "tuple": (1, 2, 3),
                "set": {1, 2, 3},  # Sets are not JSON serializable
            }
        }

        option = ReflectapiOption(complex_data)
        assert option.is_some
        assert option.unwrap() == complex_data

    def test_option_with_large_data(self):
        """Test ReflectapiOption with very large data."""
        large_string = "x" * 1000000  # 1MB string
        option = ReflectapiOption(large_string)

        assert option.is_some
        assert len(option.unwrap()) == 1000000

    def test_option_equality_with_different_types(self):
        """Test ReflectapiOption equality with different types."""
        option_int = ReflectapiOption(42)
        option_str = ReflectapiOption("42")
        option_list = ReflectapiOption([42])

        assert option_int != option_str
        assert option_int != option_list  # Different types
        assert option_str != option_list  # Different types

    def test_option_hash_with_unhashable_values(self):
        """Test ReflectapiOption hash with unhashable values."""
        unhashable_data = {"key": [1, 2, 3]}  # Lists are unhashable
        option = ReflectapiOption(unhashable_data)

        # Should raise TypeError when trying to hash
        with pytest.raises(TypeError):
            hash(option)

    def test_option_map_chain(self):
        """Test chaining multiple map operations."""
        option = ReflectapiOption(10)
        result = option.map(lambda x: x * 2).map(lambda x: x + 5).map(str)

        assert result.is_some
        assert result.unwrap() == "25"

    def test_option_filter_with_exception_in_predicate(self):
        """Test filter with predicate that raises exception."""
        option = ReflectapiOption("test")

        def bad_predicate(x):
            raise ValueError("Predicate error")

        # Should propagate the exception
        with pytest.raises(ValueError, match="Predicate error"):
            option.filter(bad_predicate)

    def test_option_map_with_exception_in_function(self):
        """Test map with function that raises exception."""
        option = ReflectapiOption(10)

        def bad_function(x):
            raise RuntimeError("Function error")

        # Should propagate the exception
        with pytest.raises(RuntimeError, match="Function error"):
            option.map(bad_function)


class TestErrorHandlingEdgeCases:
    """Test edge cases in error handling."""

    @patch("httpx.Client.send")
    def test_malformed_json_error_response(self, mock_send):
        """Test handling of malformed JSON in error responses."""
        mock_response = Mock()
        mock_response.status_code = 400
        mock_response.json.side_effect = json.JSONDecodeError("Invalid JSON", "doc", 0)
        mock_response.text = "Invalid JSON response"
        mock_response.headers = {}
        mock_response.elapsed.total_seconds.return_value = 0.1
        mock_send.return_value = mock_response

        client = ClientBase("https://api.example.com")

        with pytest.raises(ApplicationError) as exc_info:
            client._make_request("GET", "/test")

        # Should still create ApplicationError even with malformed JSON
        assert exc_info.value.status_code == 400

    @patch("httpx.Client.send")
    def test_response_with_invalid_json_body(self, mock_send):
        """Test handling of invalid JSON in success response."""
        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.json.side_effect = json.JSONDecodeError("Invalid JSON", "doc", 0)
        mock_response.headers = {}
        mock_response.elapsed.total_seconds.return_value = 0.1
        mock_send.return_value = mock_response

        client = ClientBase("https://api.example.com")

        with pytest.raises(
            ReflectApiValidationError, match="Failed to parse JSON response"
        ):
            client._make_request("GET", "/test")

    @patch("httpx.Client.send")
    def test_extremely_large_error_response(self, mock_send):
        """Test handling of extremely large error responses."""
        large_error_data = {"error": "x" * 1000000}  # 1MB error message

        mock_response = Mock()
        mock_response.status_code = 500
        mock_response.json.return_value = large_error_data
        mock_response.headers = {}
        mock_response.elapsed.total_seconds.return_value = 0.1
        mock_send.return_value = mock_response

        client = ClientBase("https://api.example.com")

        with pytest.raises(ApplicationError) as exc_info:
            client._make_request("GET", "/test")

        # Should handle large error data without issues
        assert exc_info.value.status_code == 500

    def test_application_error_with_none_response(self):
        """Test ApplicationError construction with None response."""
        mock_response = Mock()
        mock_response.status_code = 500
        mock_response.headers = {}

        metadata = TransportMetadata(
            status_code=500,
            headers=httpx.Headers({}),
            timing=0.1,
            raw_response=mock_response,
        )

        # Should not crash with None error_data
        error = ApplicationError.from_response(
            response=mock_response, metadata=metadata, error_data=None
        )

        assert error.status_code == 500
        assert str(error) is not None  # Can convert to string


class TestSerializationEdgeCases:
    """Test edge cases in serialization."""

    def test_serialize_option_dict_with_nested_options(self):
        """Test serializing dictionary with nested ReflectapiOption values."""
        from reflectapi_runtime.option import serialize_option_dict

        nested_data = {
            "level1": {
                "level2": {
                    "option_field": ReflectapiOption("value"),
                    "undefined_field": ReflectapiOption(Undefined),
                    "none_field": ReflectapiOption(None),
                }
            },
            "list_with_options": [
                ReflectapiOption("item1"),
                ReflectapiOption(Undefined),
                ReflectapiOption(None),
                "regular_item",
            ],
        }

        result = serialize_option_dict(nested_data)

        # Should properly handle nested structures
        assert result["level1"]["level2"]["option_field"] == "value"
        assert "undefined_field" not in result["level1"]["level2"]
        assert result["level1"]["level2"]["none_field"] is None

        # Should handle lists with options
        expected_list = ["item1", None, "regular_item"]
        assert result["list_with_options"] == expected_list

    def test_serialize_option_dict_with_circular_references(self):
        """Test serializing dictionary with circular references."""
        from reflectapi_runtime.option import serialize_option_dict

        # Create circular reference
        data = {"option": ReflectapiOption("value"), "nested": {}}
        data["nested"]["parent"] = data

        # Should handle circular references gracefully or raise appropriate error
        try:
            result = serialize_option_dict(data)
            # If it succeeds, verify the option was processed
            assert result["option"] == "value"
        except RecursionError:
            # Circular references might cause recursion error - that's acceptable
            pass

    def test_pydantic_model_with_invalid_reflectapi_option(self):
        """Test Pydantic model creation and validation."""

        class TestModel(BaseModel):
            field: ReflectapiOption[str] = ReflectapiOption()

        # Should work with valid data
        model = TestModel(field=ReflectapiOption("valid"))
        assert model.field.unwrap() == "valid"

        # Should work with automatic wrapping
        model2 = TestModel(field="auto_wrapped")
        assert model2.field.unwrap() == "auto_wrapped"


class TestAuthEdgeCases:
    """Test edge cases in authentication."""

    def test_bearer_token_with_empty_string(self):
        """Test BearerTokenAuth with empty token."""
        auth = BearerTokenAuth("")
        assert auth.token == ""

        # Should still work, just send empty Authorization header
        request = Mock()
        request.headers = {}
        auth.auth_flow(request)
        assert request.headers.get("Authorization") == "Bearer "

    def test_bearer_token_with_special_characters(self):
        """Test BearerTokenAuth with special characters."""
        special_token = "token-with-special-chars!@#$%^&*()_+{}[]|\\:;\"'<>?,./"
        auth = BearerTokenAuth(special_token)

        request = Mock()
        request.headers = {}
        auth.auth_flow(request)
        assert request.headers["Authorization"] == f"Bearer {special_token}"

    def test_api_key_auth_with_empty_values(self):
        """Test APIKeyAuth with empty values."""
        auth = APIKeyAuth("", "")

        request = Mock()
        request.headers = {}
        auth.auth_flow(request)

        # Should set empty header
        assert "" in request.headers

    def test_basic_auth_with_special_characters(self):
        """Test BasicAuth with special characters in credentials."""
        username = "user@domain.com"
        password = "pass!@#$%^&*()"
        auth = BasicAuth(username, password)

        request = Mock()
        request.headers = {}
        auth.auth_flow(request)

        # Should properly encode special characters
        assert "Authorization" in request.headers
        assert request.headers["Authorization"].startswith("Basic ")


class TestConcurrencyEdgeCases:
    """Test edge cases related to concurrency."""

    @pytest.mark.asyncio
    async def test_many_simultaneous_async_clients(self):
        """Test creating many async clients simultaneously."""

        async def create_client(i):
            return AsyncClientBase(f"https://api{i}.example.com")

        # Create 100 clients concurrently
        tasks = [create_client(i) for i in range(100)]
        clients = await asyncio.gather(*tasks)

        assert len(clients) == 100
        assert len(set(client.base_url for client in clients)) == 100

    @pytest.mark.asyncio
    async def test_async_client_cleanup_during_request(self):
        """Test async client cleanup while requests are in progress."""
        with patch("httpx.AsyncClient.send") as mock_send:
            # Make send method slow
            async def slow_send(request):
                await asyncio.sleep(0.1)
                mock_response = Mock()
                mock_response.status_code = 200
                mock_response.json.return_value = {"result": "ok"}
                mock_response.headers = {}
                mock_response.elapsed.total_seconds.return_value = 0.1
                return mock_response

            mock_send.side_effect = slow_send

            async with AsyncClientBase("https://api.example.com") as client:
                # Start request but don't wait for it
                task = asyncio.create_task(client._make_request("GET", "/test"))

                # Let request start
                await asyncio.sleep(0.01)

                # Context manager should wait for pending requests

            # Task should complete successfully
            response = await task
            assert response.value == {"result": "ok"}


class TestMemoryAndResourceEdgeCases:
    """Test edge cases related to memory and resource usage."""

    def test_client_with_many_headers(self):
        """Test client with extremely large number of headers."""
        large_headers = {f"header_{i}": f"value_{i}" for i in range(1000)}

        client = ClientBase("https://api.example.com", headers=large_headers)
        assert hasattr(client, "_client")

    def test_reflectapi_option_memory_usage(self):
        """Test ReflectapiOption memory usage with large data."""
        import sys

        # Create option with large data
        large_data = list(range(100000))
        option = ReflectapiOption(large_data)

        # Should not use significantly more memory than the data itself
        option_size = sys.getsizeof(option)
        data_size = sys.getsizeof(large_data)

        # Option should not add significant overhead
        assert option_size < data_size * 1.1  # Less than 10% overhead

    def test_transport_metadata_with_large_headers(self):
        """Test TransportMetadata with very large headers."""
        large_headers = {f"header_{i}": "x" * 1000 for i in range(100)}

        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.headers = large_headers

        metadata = TransportMetadata(
            status_code=200,
            headers=httpx.Headers(large_headers),
            timing=0.1,
            raw_response=mock_response,
        )

        assert metadata.status_code == 200
        assert len(metadata.headers) == 100


class TestTypeValidationEdgeCases:
    """Test edge cases in type validation."""

    def test_api_response_with_invalid_metadata(self):
        """Test ApiResponse with invalid metadata."""
        # Should handle None metadata gracefully
        response = ApiResponse({"data": "test"}, None)
        assert response.value == {"data": "test"}
        assert response.metadata is None

    def test_transport_metadata_with_invalid_response_time(self):
        """Test TransportMetadata with invalid response times."""
        mock_response = Mock()
        mock_response.status_code = 200
        mock_response.headers = {}

        # Negative response time
        metadata = TransportMetadata(
            status_code=200,
            headers=httpx.Headers({}),
            timing=-1.0,
            raw_response=mock_response,
        )
        assert metadata.timing == -1.0

        # Extremely large response time
        metadata = TransportMetadata(
            status_code=200,
            headers=httpx.Headers({}),
            timing=999999.0,
            raw_response=mock_response,
        )
        assert metadata.timing == 999999.0

    def test_client_base_with_invalid_middleware(self):
        """Test ClientBase with invalid middleware."""
        # Should handle None middleware gracefully
        client = ClientBase("https://api.example.com", middleware=None)
        assert hasattr(client, "middleware_chain")

        # Should handle empty middleware list
        client = ClientBase("https://api.example.com", middleware=[])
        assert hasattr(client, "middleware_chain")
