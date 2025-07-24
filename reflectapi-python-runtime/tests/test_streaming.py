"""Tests for streaming response functionality."""

import asyncio
import os
import sys
import tempfile
from unittest.mock import AsyncMock, Mock, patch

import httpx
import pytest
from pydantic import BaseModel

sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'src'))

from reflectapi_runtime.auth import BearerTokenAuth
from reflectapi_runtime.exceptions import ApplicationError, NetworkError, TimeoutError
from reflectapi_runtime.streaming import AsyncStreamingClient, StreamingResponse


class MockModel(BaseModel):
    """Test model for request serialization."""
    name: str
    value: int


@pytest.fixture
def mock_response():
    """Create a mock httpx.Response for streaming."""
    response = Mock(spec=httpx.Response)
    response.status_code = 200
    response.headers = httpx.Headers({"content-type": "application/octet-stream"})
    response.is_closed = False
    response.aclose = AsyncMock()
    response.aread = AsyncMock(return_value=b"test content")
    return response


@pytest.fixture
def mock_streaming_response(mock_response):
    """Create a mock StreamingResponse."""
    from reflectapi_runtime.response import TransportMetadata
    metadata = TransportMetadata(
        status_code=200,
        headers=httpx.Headers({"content-type": "application/octet-stream"}),
        timing=0.1,
        raw_response=mock_response
    )
    return StreamingResponse(mock_response, metadata)


class TestStreamingResponse:
    """Test StreamingResponse functionality."""

    def test_initialization(self, mock_response):
        """Test StreamingResponse initialization."""
        from reflectapi_runtime.response import TransportMetadata
        metadata = TransportMetadata(
            status_code=200,
            headers=httpx.Headers({"content-type": "application/json"}),
            timing=0.15,
            raw_response=mock_response
        )

        streaming_response = StreamingResponse(mock_response, metadata)

        assert streaming_response.response is mock_response
        assert streaming_response.metadata is metadata
        assert not streaming_response._closed

    def test_properties(self, mock_streaming_response):
        """Test StreamingResponse properties."""
        assert mock_streaming_response.status_code == 200
        assert mock_streaming_response.headers["content-type"] == "application/octet-stream"
        assert mock_streaming_response.content_type == "application/octet-stream"
        assert not mock_streaming_response.is_closed

    def test_content_length_present(self, mock_response):
        """Test content-length parsing when present."""
        mock_response.headers = httpx.Headers({"content-length": "1024"})
        from reflectapi_runtime.response import TransportMetadata
        metadata = TransportMetadata(
            status_code=200,
            headers=httpx.Headers({"content-length": "1024"}),
            timing=0.1,
            raw_response=mock_response
        )

        streaming_response = StreamingResponse(mock_response, metadata)
        assert streaming_response.content_length == 1024

    def test_content_length_missing(self, mock_streaming_response):
        """Test content-length when not present."""
        assert mock_streaming_response.content_length is None

    def test_content_length_invalid(self, mock_response):
        """Test content-length with invalid value."""
        mock_response.headers = httpx.Headers({"content-length": "invalid"})
        from reflectapi_runtime.response import TransportMetadata
        metadata = TransportMetadata(
            status_code=200,
            headers=httpx.Headers({"content-length": "invalid"}),
            timing=0.1,
            raw_response=mock_response
        )

        streaming_response = StreamingResponse(mock_response, metadata)
        assert streaming_response.content_length is None

    @pytest.mark.asyncio
    async def test_aiter_bytes(self, mock_streaming_response):
        """Test iterating over bytes chunks."""
        # Mock the async iterator
        async def mock_aiter_bytes(chunk_size):
            yield b"chunk1"
            yield b"chunk2"
            yield b"chunk3"

        mock_streaming_response.response.aiter_bytes = mock_aiter_bytes

        chunks = []
        async for chunk in mock_streaming_response.aiter_bytes():
            chunks.append(chunk)

        assert chunks == [b"chunk1", b"chunk2", b"chunk3"]
        # Should auto-close after iteration
        mock_streaming_response.response.aclose.assert_called_once()

    @pytest.mark.asyncio
    async def test_aiter_bytes_closed_response(self, mock_streaming_response):
        """Test iterating over closed response raises error."""
        mock_streaming_response._closed = True

        with pytest.raises(RuntimeError, match="Cannot iterate over closed response"):
            async for _chunk in mock_streaming_response.aiter_bytes():
                pass

    @pytest.mark.asyncio
    async def test_aiter_text(self, mock_streaming_response):
        """Test iterating over text chunks."""
        # Mock the async iterator
        async def mock_aiter_text(chunk_size, encoding):
            yield "text1"
            yield "text2"

        mock_streaming_response.response.aiter_text = mock_aiter_text

        chunks = []
        async for chunk in mock_streaming_response.aiter_text():
            chunks.append(chunk)

        assert chunks == ["text1", "text2"]
        mock_streaming_response.response.aclose.assert_called_once()

    @pytest.mark.asyncio
    async def test_aiter_lines(self, mock_streaming_response):
        """Test iterating over lines."""
        # Mock the async iterator
        async def mock_aiter_lines(chunk_size):
            yield "line1"
            yield "line2"

        mock_streaming_response.response.aiter_lines = mock_aiter_lines

        lines = []
        async for line in mock_streaming_response.aiter_lines():
            lines.append(line)

        assert lines == ["line1", "line2"]
        mock_streaming_response.response.aclose.assert_called_once()

    @pytest.mark.asyncio
    async def test_aiter_lines_with_encoding(self, mock_streaming_response):
        """Test iterating over lines with encoding."""
        # Mock the async iterator returning bytes
        async def mock_aiter_lines(chunk_size):
            yield b"line1"
            yield b"line2"

        mock_streaming_response.response.aiter_lines = mock_aiter_lines

        lines = []
        async for line in mock_streaming_response.aiter_lines(encoding="utf-8"):
            lines.append(line)

        assert lines == ["line1", "line2"]

    @pytest.mark.asyncio
    async def test_save_to_file(self, mock_streaming_response):
        """Test saving stream to file."""
        # Mock the async iterator
        async def mock_aiter_bytes(chunk_size):
            yield b"chunk1"
            yield b"chunk2"

        mock_streaming_response.response.aiter_bytes = mock_aiter_bytes

        with tempfile.NamedTemporaryFile(delete=False) as tmp_file:
            tmp_path = tmp_file.name

        try:
            bytes_written = await mock_streaming_response.save_to_file(tmp_path)

            assert bytes_written == 12  # len("chunk1chunk2")

            # Verify file contents
            with open(tmp_path, 'rb') as f:
                content = f.read()
            assert content == b"chunk1chunk2"

        finally:
            os.unlink(tmp_path)

    @pytest.mark.asyncio
    async def test_save_to_file_with_error(self, mock_streaming_response):
        """Test saving stream to file with error cleanup."""
        # Mock the async iterator that raises an error
        async def mock_aiter_bytes(chunk_size):
            yield b"chunk1"
            raise RuntimeError("Stream error")

        mock_streaming_response.response.aiter_bytes = mock_aiter_bytes

        with tempfile.NamedTemporaryFile(delete=False) as tmp_file:
            tmp_path = tmp_file.name

        # Remove the temp file so we can test cleanup
        os.unlink(tmp_path)

        with pytest.raises(RuntimeError, match="Stream error"):
            await mock_streaming_response.save_to_file(tmp_path)

        # File should not exist after error cleanup
        assert not os.path.exists(tmp_path)

    @pytest.mark.asyncio
    async def test_read_all(self, mock_streaming_response):
        """Test reading all content."""
        mock_streaming_response.response.aread.return_value = b"full content"

        content = await mock_streaming_response.read_all()

        assert content == b"full content"
        mock_streaming_response.response.aread.assert_called_once()
        mock_streaming_response.response.aclose.assert_called_once()

    @pytest.mark.asyncio
    async def test_aclose(self, mock_streaming_response):
        """Test closing the response."""
        await mock_streaming_response.aclose()

        assert mock_streaming_response._closed
        mock_streaming_response.response.aclose.assert_called_once()

    @pytest.mark.asyncio
    async def test_aclose_idempotent(self, mock_streaming_response):
        """Test that closing multiple times is safe."""
        await mock_streaming_response.aclose()
        await mock_streaming_response.aclose()

        assert mock_streaming_response._closed
        # Should only call aclose once
        mock_streaming_response.response.aclose.assert_called_once()


class TestAsyncStreamingClient:
    """Test AsyncStreamingClient functionality."""

    def test_initialization(self):
        """Test client initialization."""
        client = AsyncStreamingClient("https://api.example.com")

        assert client.base_url == "https://api.example.com"
        assert client._owns_client

    def test_initialization_with_client(self):
        """Test initialization with provided client."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        client = AsyncStreamingClient("https://api.example.com", client=mock_client)

        assert client._client is mock_client
        assert not client._owns_client

    def test_initialization_with_auth(self):
        """Test initialization with authentication."""
        auth = BearerTokenAuth("test_token")
        client = AsyncStreamingClient("https://api.example.com", auth=auth)

        assert client.auth is auth

    @pytest.mark.asyncio
    async def test_context_manager(self):
        """Test client as async context manager."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)

        async with AsyncStreamingClient("https://api.example.com", client=mock_client) as client:
            assert isinstance(client, AsyncStreamingClient)

        # Client should not be closed since we don't own it
        mock_client.aclose.assert_not_called()

    @pytest.mark.asyncio
    async def test_aclose_owned_client(self):
        """Test closing owned client."""
        with patch('httpx.AsyncClient') as mock_client_class:
            mock_client = AsyncMock()
            mock_client_class.return_value = mock_client

            client = AsyncStreamingClient("https://api.example.com")
            await client.aclose()

            mock_client.aclose.assert_called_once()

    @pytest.mark.asyncio
    async def test_aclose_external_client(self):
        """Test not closing external client."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        client = AsyncStreamingClient("https://api.example.com", client=mock_client)

        await client.aclose()

        mock_client.aclose.assert_not_called()

    @pytest.mark.asyncio
    async def test_stream_request_basic(self):
        """Test basic streaming request."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200
        mock_response.headers = httpx.Headers({"content-type": "application/json"})
        mock_response.aclose = AsyncMock()

        mock_request = Mock(spec=httpx.Request)
        mock_client.build_request.return_value = mock_request
        mock_client.send.return_value = mock_response

        client = AsyncStreamingClient("https://api.example.com", client=mock_client)

        async with client.stream_request("GET", "/test") as response:
            assert isinstance(response, StreamingResponse)
            assert response.status_code == 200

        mock_client.build_request.assert_called_once()
        mock_client.send.assert_called_once_with(mock_request, stream=True)
        mock_response.aclose.assert_called_once()

    @pytest.mark.asyncio
    async def test_stream_request_with_json_data(self):
        """Test streaming request with JSON data."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200
        mock_response.headers = httpx.Headers({})
        mock_response.aclose = AsyncMock()

        mock_request = Mock(spec=httpx.Request)
        mock_client.build_request.return_value = mock_request
        mock_client.send.return_value = mock_response

        client = AsyncStreamingClient("https://api.example.com", client=mock_client)

        async with client.stream_request("POST", "/test", json_data={"key": "value"}):
            pass

        mock_client.build_request.assert_called_once_with(
            method="POST",
            url="https://api.example.com/test",
            params=None,
            json={"key": "value"},
            headers={},
        )

    @pytest.mark.asyncio
    async def test_stream_request_with_pydantic_model(self):
        """Test streaming request with Pydantic model."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200
        mock_response.headers = httpx.Headers({})
        mock_response.aclose = AsyncMock()

        mock_request = Mock(spec=httpx.Request)
        mock_client.build_request.return_value = mock_request
        mock_client.send.return_value = mock_response

        client = AsyncStreamingClient("https://api.example.com", client=mock_client)
        model = MockModel(name="test", value=42)

        async with client.stream_request("POST", "/test", json_model=model):
            pass

        # Should be called with raw content and Content-Type header
        call_args = mock_client.build_request.call_args
        assert call_args[1]["method"] == "POST"
        assert call_args[1]["url"] == "https://api.example.com/test"
        assert call_args[1]["content"] == b'{"name":"test","value":42}'
        assert call_args[1]["headers"]["Content-Type"] == "application/json"

    @pytest.mark.asyncio
    async def test_stream_request_conflicting_params(self):
        """Test streaming request with conflicting json_data and json_model."""
        client = AsyncStreamingClient("https://api.example.com")
        model = MockModel(name="test", value=42)

        with pytest.raises(ValueError, match="Cannot specify both json_data and json_model"):
            async with client.stream_request(
                "POST", "/test",
                json_data={"key": "value"},
                json_model=model
            ):
                pass

    @pytest.mark.asyncio
    async def test_stream_request_error_response(self):
        """Test streaming request with error response."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 404
        mock_response.headers = httpx.Headers({"content-type": "application/json"})
        mock_response.aread.return_value = b'{"error": "Not found"}'
        mock_response.json.return_value = {"error": "Not found"}
        mock_response.aclose = AsyncMock()

        mock_request = Mock(spec=httpx.Request)
        mock_client.build_request.return_value = mock_request
        mock_client.send.return_value = mock_response

        client = AsyncStreamingClient("https://api.example.com", client=mock_client)

        with pytest.raises(ApplicationError) as exc_info:
            async with client.stream_request("GET", "/test"):
                pass

        assert exc_info.value.status_code == 404

    @pytest.mark.asyncio
    async def test_stream_request_timeout_error(self):
        """Test streaming request with timeout error."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_client.send.side_effect = httpx.TimeoutException("Request timeout")

        mock_request = Mock(spec=httpx.Request)
        mock_client.build_request.return_value = mock_request

        client = AsyncStreamingClient("https://api.example.com", client=mock_client)

        with pytest.raises(TimeoutError):
            async with client.stream_request("GET", "/test"):
                pass

    @pytest.mark.asyncio
    async def test_stream_request_network_error(self):
        """Test streaming request with network error."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_client.send.side_effect = httpx.RequestError("Connection failed")

        mock_request = Mock(spec=httpx.Request)
        mock_client.build_request.return_value = mock_request

        client = AsyncStreamingClient("https://api.example.com", client=mock_client)

        with pytest.raises(NetworkError):
            async with client.stream_request("GET", "/test"):
                pass

    @pytest.mark.asyncio
    async def test_download_file(self):
        """Test file download functionality."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200
        mock_response.headers = httpx.Headers({"content-length": "12"})
        mock_response.is_closed = False
        mock_response.aclose = AsyncMock()

        # Mock the async iterator
        async def mock_aiter_bytes(chunk_size):
            yield b"chunk1"
            yield b"chunk2"

        mock_response.aiter_bytes = mock_aiter_bytes

        mock_request = Mock(spec=httpx.Request)
        mock_client.build_request.return_value = mock_request
        mock_client.send.return_value = mock_response

        client = AsyncStreamingClient("https://api.example.com", client=mock_client)

        with tempfile.NamedTemporaryFile(delete=False) as tmp_file:
            tmp_path = tmp_file.name

        try:
            async with client.download_file("/test", tmp_path) as progress:
                assert progress["bytes_written"] == 12
                assert progress["total_bytes"] == 12
                assert progress["progress"] == 1.0
                assert isinstance(progress["response"], StreamingResponse)

            # Verify file contents
            with open(tmp_path, 'rb') as f:
                content = f.read()
            assert content == b"chunk1chunk2"

        finally:
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)

    @pytest.mark.asyncio
    async def test_download_file_no_content_length(self):
        """Test file download without content-length header."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200
        mock_response.headers = httpx.Headers({})  # No content-length
        mock_response.is_closed = False
        mock_response.aclose = AsyncMock()

        # Mock the async iterator
        async def mock_aiter_bytes(chunk_size):
            yield b"data"

        mock_response.aiter_bytes = mock_aiter_bytes

        mock_request = Mock(spec=httpx.Request)
        mock_client.build_request.return_value = mock_request
        mock_client.send.return_value = mock_response

        client = AsyncStreamingClient("https://api.example.com", client=mock_client)

        with tempfile.NamedTemporaryFile(delete=False) as tmp_file:
            tmp_path = tmp_file.name

        try:
            async with client.download_file("/test", tmp_path) as progress:
                assert progress["bytes_written"] == 4
                assert progress["total_bytes"] is None
                assert progress["progress"] is None

        finally:
            if os.path.exists(tmp_path):
                os.unlink(tmp_path)

    @pytest.mark.asyncio
    async def test_download_file_error_cleanup(self):
        """Test file download with error cleanup."""
        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200
        mock_response.headers = httpx.Headers({})
        mock_response.is_closed = False
        mock_response.aclose = AsyncMock()

        # Mock the async iterator that raises an error
        async def mock_aiter_bytes(chunk_size):
            yield b"data"
            raise RuntimeError("Stream error")

        mock_response.aiter_bytes = mock_aiter_bytes

        mock_request = Mock(spec=httpx.Request)
        mock_client.build_request.return_value = mock_request
        mock_client.send.return_value = mock_response

        client = AsyncStreamingClient("https://api.example.com", client=mock_client)

        with tempfile.NamedTemporaryFile(delete=False) as tmp_file:
            tmp_path = tmp_file.name

        # Remove the temp file so we can test cleanup
        os.unlink(tmp_path)

        with pytest.raises(RuntimeError, match="Stream error"):
            async with client.download_file("/test", tmp_path):
                pass

        # File should not exist after error cleanup
        assert not os.path.exists(tmp_path)


class TestAsyncStreamingClientConvenienceMethods:
    """Test convenience factory methods."""

    def test_from_bearer_token(self):
        """Test creating client with bearer token."""
        client = AsyncStreamingClient.from_bearer_token(
            "https://api.example.com",
            "test_token"
        )

        assert isinstance(client.auth, BearerTokenAuth)
        assert client.auth.token == "test_token"

    def test_from_api_key(self):
        """Test creating client with API key."""
        from reflectapi_runtime.auth import APIKeyAuth

        client = AsyncStreamingClient.from_api_key(
            "https://api.example.com",
            "test_key"
        )

        assert isinstance(client.auth, APIKeyAuth)
        assert client.auth.api_key == "test_key"

    def test_from_basic_auth(self):
        """Test creating client with basic auth."""
        from reflectapi_runtime.auth import BasicAuth

        client = AsyncStreamingClient.from_basic_auth(
            "https://api.example.com",
            "username",
            "password"
        )

        assert isinstance(client.auth, BasicAuth)

    def test_from_oauth2_client_credentials(self):
        """Test creating client with OAuth2 client credentials."""
        from reflectapi_runtime.auth import OAuth2ClientCredentialsAuth

        client = AsyncStreamingClient.from_oauth2_client_credentials(
            "https://api.example.com",
            "https://auth.example.com/token",
            "client_id",
            "client_secret",
            "read write"
        )

        assert isinstance(client.auth, OAuth2ClientCredentialsAuth)
        assert client.auth.token_url == "https://auth.example.com/token"
        assert client.auth.client_id == "client_id"
        assert client.auth.client_secret == "client_secret"
        assert client.auth.scope == "read write"
