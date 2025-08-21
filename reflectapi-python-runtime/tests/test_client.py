"""Tests for the client base classes."""

from unittest.mock import AsyncMock, Mock

import httpx
import pytest
from pydantic import BaseModel

from reflectapi_runtime import (
    ApiResponse,
    ApplicationError,
    AsyncClientBase,
    ClientBase,
    NetworkError,
    TimeoutError,
    TransportMetadata,
    ValidationError,
)


class SampleModel(BaseModel):
    name: str
    age: int


@pytest.fixture
def mock_httpx_client():
    mock_client = Mock(spec=httpx.Client)
    mock_response = Mock(spec=httpx.Response)
    mock_response.status_code = 200
    mock_response.headers = httpx.Headers({})
    mock_response.reason_phrase = "OK"
    mock_response.json.return_value = {"name": "test", "age": 25}
    mock_response.content = b'{"name": "test", "age": 25}'  # Raw JSON bytes
    mock_client.request.return_value = mock_response
    mock_client.send.return_value = mock_response
    mock_client.build_request.return_value = Mock(spec=httpx.Request)
    return mock_client, mock_response


@pytest.fixture
def mock_async_httpx_client():
    mock_client = AsyncMock(spec=httpx.AsyncClient)
    mock_response = Mock(spec=httpx.Response)
    mock_response.status_code = 200
    mock_response.headers = httpx.Headers({})
    mock_response.reason_phrase = "OK"
    mock_response.json.return_value = {"name": "test", "age": 25}
    mock_response.content = b'{"name": "test", "age": 25}'  # Raw JSON bytes
    mock_client.send = AsyncMock(return_value=mock_response)
    mock_client.build_request.return_value = Mock(spec=httpx.Request)
    return mock_client, mock_response


class TestClientBase:
    def test_initialization(self):
        client = ClientBase("http://example.com")
        assert client.base_url == "http://example.com"
        assert client._owns_client is True

    def test_initialization_with_custom_client(self, mock_httpx_client):
        mock_client, _ = mock_httpx_client
        client = ClientBase("http://example.com", client=mock_client)
        assert client._client is mock_client
        assert client._owns_client is False

    def test_from_bearer_token(self):
        client = ClientBase.from_bearer_token("http://example.com", "test-token")
        assert client.base_url == "http://example.com"

    def test_from_bearer_token_with_headers(self):
        existing_headers = {"User-Agent": "test-agent"}
        client = ClientBase.from_bearer_token(
            "http://example.com", "test-token", headers=existing_headers
        )
        assert client.base_url == "http://example.com"

    def test_close_owned_client(self):
        client = ClientBase("http://example.com")
        assert client._owns_client is True

        mock_close = Mock()
        client._client.close = mock_close
        client.close()

        mock_close.assert_called_once()

    def test_close_unowned_client(self):
        mock_client = Mock()
        mock_client.close = Mock()

        client = ClientBase("http://example.com", client=mock_client)
        assert client._owns_client is False

        client.close()

        mock_client.close.assert_not_called()

    def test_context_manager(self, mock_httpx_client):
        mock_client, _ = mock_httpx_client

        with ClientBase("http://example.com", client=mock_client) as client:
            assert isinstance(client, ClientBase)

    def test_make_request_success(self, mock_httpx_client):
        mock_client, mock_response = mock_httpx_client

        client = ClientBase("http://example.com", client=mock_client)

        result = client._make_request("GET", "/test", response_model=SampleModel)

        assert isinstance(result, ApiResponse)
        assert isinstance(result.value, SampleModel)
        assert result.value.name == "test"
        assert result.value.age == 25
        assert isinstance(result.metadata, TransportMetadata)
        assert result.metadata.status_code == 200

    def test_make_request_error_response(self, mock_httpx_client):
        mock_client, mock_response = mock_httpx_client
        mock_response.status_code = 400
        mock_response.reason_phrase = "Bad Request"
        mock_response.json.return_value = {"error": "Invalid data"}
        mock_response.content = b'{"error": "Invalid data"}'

        client = ClientBase("http://example.com", client=mock_client)

        with pytest.raises(ApplicationError) as exc_info:
            client._make_request("GET", "/test")

        from typing import cast

        error = cast("ApplicationError", exc_info.value)
        assert error.status_code == 400
        assert "Bad Request" in str(exc_info.value)

    def test_make_request_json_parse_error(self, mock_httpx_client):
        mock_client, mock_response = mock_httpx_client
        mock_response.json.side_effect = ValueError("Invalid JSON")

        client = ClientBase("http://example.com", client=mock_client)

        with pytest.raises(ValidationError) as exc_info:
            client._make_request("GET", "/test")

        assert "Failed to parse JSON response" in str(exc_info.value)

    def test_make_request_timeout_error(self, mock_httpx_client):
        mock_client, _ = mock_httpx_client
        mock_client.send.side_effect = httpx.TimeoutException("Request timed out")

        client = ClientBase("http://example.com", client=mock_client)

        with pytest.raises(TimeoutError):
            client._make_request("GET", "/test")

    def test_make_request_network_error(self, mock_httpx_client):
        mock_client, _ = mock_httpx_client
        mock_client.send.side_effect = httpx.ConnectError("Connection failed")

        client = ClientBase("http://example.com", client=mock_client)

        with pytest.raises(NetworkError):
            client._make_request("GET", "/test")

    def test_make_request_with_middleware(self, mock_httpx_client):
        from reflectapi_runtime.middleware import SyncLoggingMiddleware

        mock_client, mock_response = mock_httpx_client
        middleware = [SyncLoggingMiddleware()]

        mock_request = Mock(spec=httpx.Request)
        mock_request.method = "GET"
        mock_request.url = "http://example.com/test"
        mock_request.headers = {}
        mock_client.build_request.return_value = mock_request

        client = ClientBase(
            "http://example.com", client=mock_client, middleware=middleware
        )

        result = client._make_request("GET", "/test", response_model=SampleModel)

        assert isinstance(result, ApiResponse)
        mock_client.send.assert_called_once()
        mock_client.build_request.assert_called_once()

    def test_make_request_error_response_malformed_json(self, mock_httpx_client):
        mock_client, mock_response = mock_httpx_client
        mock_response.status_code = 500
        mock_response.reason_phrase = "Internal Server Error"
        mock_response.json.side_effect = ValueError("Invalid JSON")

        client = ClientBase("http://example.com", client=mock_client)

        with pytest.raises(ApplicationError) as exc_info:
            client._make_request("GET", "/test")

        from typing import cast

        error = cast("ApplicationError", exc_info.value)
        assert error.status_code == 500
        assert error.error_data is None

    def test_make_request_any_type_response_model(self, mock_httpx_client):
        from typing import Any

        mock_client, mock_response = mock_httpx_client
        mock_response.json.return_value = {"arbitrary": "data"}
        mock_response.content = b'{"arbitrary": "data"}'

        client = ClientBase("http://example.com", client=mock_client)

        result = client._make_request("GET", "/test", response_model=Any)
        assert isinstance(result, ApiResponse)
        assert result.value == {"arbitrary": "data"}

    def test_make_request_string_any_response_model(self, mock_httpx_client):
        mock_client, mock_response = mock_httpx_client
        mock_response.json.return_value = {"arbitrary": "data"}
        mock_response.content = b'{"arbitrary": "data"}'

        client = ClientBase("http://example.com", client=mock_client)

        result = client._make_request("GET", "/test", response_model="Any")
        assert isinstance(result, ApiResponse)
        assert result.value == {"arbitrary": "data"}

    def test_make_request_pydantic_validation_error(self, mock_httpx_client):
        mock_client, mock_response = mock_httpx_client
        mock_response.json.return_value = {
            "invalid": "data",
            "missing_required_fields": True,
        }
        mock_response.content = b'{"invalid": "data", "missing_required_fields": true}'

        client = ClientBase("http://example.com", client=mock_client)

        with pytest.raises(ValidationError) as exc_info:
            client._make_request("GET", "/test", response_model=SampleModel)

        assert "Response validation failed" in str(exc_info.value)
        assert hasattr(exc_info.value, "validation_errors")


class TestAsyncClientBase:
    def test_initialization(self):
        client = AsyncClientBase("http://example.com")
        assert client.base_url == "http://example.com"
        assert client._owns_client is True

    def test_from_bearer_token(self):
        client = AsyncClientBase.from_bearer_token("http://example.com", "test-token")
        assert client.base_url == "http://example.com"

    def test_from_bearer_token_with_existing_headers(self):
        existing_headers = {"User-Agent": "test-agent"}
        client = AsyncClientBase.from_bearer_token(
            "http://example.com", "test-token", headers=existing_headers
        )
        assert client.base_url == "http://example.com"

    @pytest.mark.asyncio
    async def test_close_owned_client(self):
        client = AsyncClientBase("http://example.com")
        assert client._owns_client is True

        mock_aclose = AsyncMock()
        client._client.aclose = mock_aclose
        await client.aclose()

        mock_aclose.assert_called_once()

    @pytest.mark.asyncio
    async def test_close_unowned_client(self):
        mock_client = AsyncMock()
        mock_client.aclose = AsyncMock()

        client = AsyncClientBase("http://example.com", client=mock_client)
        assert client._owns_client is False

        await client.aclose()

        mock_client.aclose.assert_not_called()

    @pytest.mark.asyncio
    async def test_context_manager(self, mock_async_httpx_client):
        mock_client, _ = mock_async_httpx_client

        async with AsyncClientBase("http://example.com", client=mock_client) as client:
            assert isinstance(client, AsyncClientBase)

    @pytest.mark.asyncio
    async def test_make_request_success(self, mock_async_httpx_client):
        mock_client, mock_response = mock_async_httpx_client

        client = AsyncClientBase("http://example.com", client=mock_client)

        result = await client._make_request("GET", "/test", response_model=SampleModel)

        assert isinstance(result, ApiResponse)
        assert isinstance(result.value, SampleModel)
        assert result.value.name == "test"
        assert result.value.age == 25

    @pytest.mark.asyncio
    async def test_make_request_error_response(self, mock_async_httpx_client):
        mock_client, mock_response = mock_async_httpx_client
        mock_response.status_code = 500
        mock_response.reason_phrase = "Internal Server Error"

        client = AsyncClientBase("http://example.com", client=mock_client)

        with pytest.raises(ApplicationError) as exc_info:
            await client._make_request("GET", "/test")

        from typing import cast

        error = cast("ApplicationError", exc_info.value)
        assert error.status_code == 500

    @pytest.mark.asyncio
    async def test_make_request_timeout_error(self, mock_async_httpx_client):
        mock_client, _ = mock_async_httpx_client
        mock_client.send.side_effect = httpx.TimeoutException("Request timed out")

        client = AsyncClientBase("http://example.com", client=mock_client)

        with pytest.raises(TimeoutError):
            await client._make_request("GET", "/test")

    @pytest.mark.asyncio
    async def test_make_request_network_error(self, mock_async_httpx_client):
        mock_client, _ = mock_async_httpx_client
        mock_client.send.side_effect = httpx.ConnectError("Connection failed")

        client = AsyncClientBase("http://example.com", client=mock_client)

        with pytest.raises(NetworkError):
            await client._make_request("GET", "/test")

    @pytest.mark.asyncio
    async def test_make_request_with_middleware(self, mock_async_httpx_client):
        from reflectapi_runtime.middleware import AsyncLoggingMiddleware

        mock_client, mock_response = mock_async_httpx_client
        middleware = [AsyncLoggingMiddleware()]

        mock_request = Mock(spec=httpx.Request)
        mock_request.method = "GET"
        mock_request.url = "http://example.com/test"
        mock_request.headers = {}
        mock_client.build_request.return_value = mock_request

        client = AsyncClientBase(
            "http://example.com", client=mock_client, middleware=middleware
        )

        result = await client._make_request("GET", "/test", response_model=SampleModel)

        assert isinstance(result, ApiResponse)
        mock_client.send.assert_called_once()
        mock_client.build_request.assert_called_once()

    @pytest.mark.asyncio
    async def test_make_request_error_response_malformed_json(
        self, mock_async_httpx_client
    ):
        mock_client, mock_response = mock_async_httpx_client
        mock_response.status_code = 500
        mock_response.reason_phrase = "Internal Server Error"
        mock_response.json.side_effect = ValueError("Invalid JSON")

        client = AsyncClientBase("http://example.com", client=mock_client)

        with pytest.raises(ApplicationError) as exc_info:
            await client._make_request("GET", "/test")

        from typing import cast

        error = cast("ApplicationError", exc_info.value)
        assert error.status_code == 500
        assert error.error_data is None

    @pytest.mark.asyncio
    async def test_make_request_any_type_response_model(self, mock_async_httpx_client):
        from typing import Any

        mock_client, mock_response = mock_async_httpx_client
        mock_response.json.return_value = {"arbitrary": "data"}
        mock_response.content = b'{"arbitrary": "data"}'

        client = AsyncClientBase("http://example.com", client=mock_client)

        result = await client._make_request("GET", "/test", response_model=Any)
        assert isinstance(result, ApiResponse)
        assert result.value == {"arbitrary": "data"}

    @pytest.mark.asyncio
    async def test_make_request_string_any_response_model(
        self, mock_async_httpx_client
    ):
        mock_client, mock_response = mock_async_httpx_client
        mock_response.json.return_value = {"arbitrary": "data"}
        mock_response.content = b'{"arbitrary": "data"}'

        client = AsyncClientBase("http://example.com", client=mock_client)

        result = await client._make_request("GET", "/test", response_model="Any")
        assert isinstance(result, ApiResponse)
        assert result.value == {"arbitrary": "data"}

    @pytest.mark.asyncio
    async def test_make_request_pydantic_validation_error(
        self, mock_async_httpx_client
    ):
        mock_client, mock_response = mock_async_httpx_client
        mock_response.json.return_value = {
            "invalid": "data",
            "missing_required_fields": True,
        }
        mock_response.content = b'{"invalid": "data", "missing_required_fields": true}'

        client = AsyncClientBase("http://example.com", client=mock_client)

        with pytest.raises(ValidationError) as exc_info:
            await client._make_request("GET", "/test", response_model=SampleModel)

        assert "Response validation failed" in str(exc_info.value)
        assert hasattr(exc_info.value, "validation_errors")
