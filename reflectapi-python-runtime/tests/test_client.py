"""Tests for the client base classes."""

from unittest.mock import AsyncMock, Mock

import httpx
import pytest
from pydantic import BaseModel

from reflectapi_runtime import (
    ApiResponse,
    ApplicationError,
    AsyncClient,
    AsyncClientBase,
    Client,
    ClientBase,
    Request,
    Response,
    NetworkError,
    TimeoutError,
    TransportMetadata,
    ValidationError,
)


class SampleModel(BaseModel):
    name: str
    age: int


class ShapeClient:
    def __init__(self) -> None:
        self.requests: list[Request] = []

    def request(self, request: Request) -> Response:
        self.requests.append(request)
        return Response(
            status=200,
            headers=httpx.Headers({"content-type": "application/json"}),
            body=b'{"name":"test","age":25}',
        )


class AsyncShapeClient:
    def __init__(self) -> None:
        self.requests: list[Request] = []

    async def request(self, request: Request) -> Response:
        self.requests.append(request)
        return Response(
            status=200,
            headers=httpx.Headers({"content-type": "application/json"}),
            body=b'{"name":"test","age":25}',
        )


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

        result = client._make_request("/test", response_model=SampleModel)

        assert isinstance(result, ApiResponse)
        assert isinstance(result.value, SampleModel)
        assert result.value.name == "test"
        assert result.value.age == 25
        assert isinstance(result.metadata, TransportMetadata)
        assert result.metadata.status_code == 200

    def test_make_request_without_body_omits_httpx_content(self, mock_httpx_client):
        mock_client, _ = mock_httpx_client
        client = ClientBase("http://example.com", client=mock_client)

        client._make_request("/test", response_model=SampleModel)

        _, kwargs = mock_client.build_request.call_args
        assert "content" not in kwargs
        assert kwargs["headers"] is None

    def test_make_request_uses_client_request_shape(self):
        shape_client = ShapeClient()
        client = ClientBase("http://example.com", client=shape_client)

        result = client._make_request("/test",
            json_model=SampleModel(name="input", age=12),
            response_model=SampleModel,
        )

        assert isinstance(shape_client, Client)
        assert isinstance(result.value, SampleModel)
        assert shape_client.requests == [
            Request(
                path="/test",
                headers={"Content-Type": "application/json"},
                body=b'{"name":"input","age":12}',
            )
        ]

    def test_make_request_without_body_uses_empty_client_request_body(self):
        shape_client = ShapeClient()
        client = ClientBase("http://example.com", client=shape_client)

        result = client._make_request("/test", response_model=SampleModel)

        assert isinstance(result.value, SampleModel)
        assert shape_client.requests == [
            Request(path="/test", headers={}, body=b"")
        ]

    def test_make_request_error_response(self, mock_httpx_client):
        mock_client, mock_response = mock_httpx_client
        mock_response.status_code = 400
        mock_response.reason_phrase = "Bad Request"
        mock_response.json.return_value = {"error": "Invalid data"}
        mock_response.content = b'{"error": "Invalid data"}'

        client = ClientBase("http://example.com", client=mock_client)

        with pytest.raises(ApplicationError) as exc_info:
            client._make_request("/test")

        from typing import cast

        error = cast("ApplicationError", exc_info.value)
        assert error.status_code == 400
        assert "Bad Request" in str(exc_info.value)

    def test_make_request_json_parse_error(self, mock_httpx_client):
        mock_client, mock_response = mock_httpx_client
        mock_response.content = b"not valid json"
        mock_response.json.side_effect = ValueError("Invalid JSON")

        client = ClientBase("http://example.com", client=mock_client)

        with pytest.raises(ValidationError) as exc_info:
            client._make_request("/test")

        assert "Failed to parse JSON response" in str(exc_info.value)

    def test_make_request_timeout_error(self, mock_httpx_client):
        mock_client, _ = mock_httpx_client
        mock_client.send.side_effect = httpx.TimeoutException("Request timed out")

        client = ClientBase("http://example.com", client=mock_client)

        with pytest.raises(TimeoutError):
            client._make_request("/test")

    def test_make_request_network_error(self, mock_httpx_client):
        mock_client, _ = mock_httpx_client
        mock_client.send.side_effect = httpx.ConnectError("Connection failed")

        client = ClientBase("http://example.com", client=mock_client)

        with pytest.raises(NetworkError):
            client._make_request("/test")

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

        result = client._make_request("/test", response_model=SampleModel)

        assert isinstance(result, ApiResponse)
        mock_client.send.assert_called_once()
        mock_client.build_request.assert_called_once()

    def test_make_request_error_response_malformed_json(self, mock_httpx_client):
        mock_client, mock_response = mock_httpx_client
        mock_response.status_code = 500
        mock_response.reason_phrase = "Internal Server Error"
        mock_response.content = b"not valid json"
        # The runtime now hands the actual httpx.Response through to
        # error handling (so `metadata.raw_response.request` is real); a
        # spec'd Mock therefore needs the .json() failure mocked too.
        mock_response.json.side_effect = ValueError("Invalid JSON")

        client = ClientBase("http://example.com", client=mock_client)

        with pytest.raises(ApplicationError) as exc_info:
            client._make_request("/test")

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

        result = client._make_request("/test", response_model=Any)
        assert isinstance(result, ApiResponse)
        assert result.value == {"arbitrary": "data"}

    def test_make_request_string_any_response_model(self, mock_httpx_client):
        mock_client, mock_response = mock_httpx_client
        mock_response.json.return_value = {"arbitrary": "data"}
        mock_response.content = b'{"arbitrary": "data"}'

        client = ClientBase("http://example.com", client=mock_client)

        result = client._make_request("/test", response_model="Any")
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
            client._make_request("/test", response_model=SampleModel)

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

        result = await client._make_request("/test", response_model=SampleModel)

        assert isinstance(result, ApiResponse)
        assert isinstance(result.value, SampleModel)
        assert result.value.name == "test"
        assert result.value.age == 25

    @pytest.mark.asyncio
    async def test_make_request_without_body_omits_httpx_content(
        self, mock_async_httpx_client
    ):
        mock_client, _ = mock_async_httpx_client
        client = AsyncClientBase("http://example.com", client=mock_client)

        await client._make_request("/test", response_model=SampleModel)

        _, kwargs = mock_client.build_request.call_args
        assert "content" not in kwargs
        assert kwargs["headers"] is None

    @pytest.mark.asyncio
    async def test_make_request_uses_client_request_shape(self):
        shape_client = AsyncShapeClient()
        client = AsyncClientBase("http://example.com", client=shape_client)

        result = await client._make_request("/test",
            json_model=SampleModel(name="input", age=12),
            response_model=SampleModel,
        )

        assert isinstance(shape_client, AsyncClient)
        assert isinstance(result.value, SampleModel)
        assert shape_client.requests == [
            Request(
                path="/test",
                headers={"Content-Type": "application/json"},
                body=b'{"name":"input","age":12}',
            )
        ]

    @pytest.mark.asyncio
    async def test_make_request_without_body_uses_empty_client_request_body(self):
        shape_client = AsyncShapeClient()
        client = AsyncClientBase("http://example.com", client=shape_client)

        result = await client._make_request("/test", response_model=SampleModel)

        assert isinstance(result.value, SampleModel)
        assert shape_client.requests == [
            Request(path="/test", headers={}, body=b"")
        ]

    @pytest.mark.asyncio
    async def test_make_request_error_response(self, mock_async_httpx_client):
        mock_client, mock_response = mock_async_httpx_client
        mock_response.status_code = 500
        mock_response.reason_phrase = "Internal Server Error"

        client = AsyncClientBase("http://example.com", client=mock_client)

        with pytest.raises(ApplicationError) as exc_info:
            await client._make_request("/test")

        from typing import cast

        error = cast("ApplicationError", exc_info.value)
        assert error.status_code == 500

    @pytest.mark.asyncio
    async def test_make_request_timeout_error(self, mock_async_httpx_client):
        mock_client, _ = mock_async_httpx_client
        mock_client.send.side_effect = httpx.TimeoutException("Request timed out")

        client = AsyncClientBase("http://example.com", client=mock_client)

        with pytest.raises(TimeoutError):
            await client._make_request("/test")

    @pytest.mark.asyncio
    async def test_make_request_network_error(self, mock_async_httpx_client):
        mock_client, _ = mock_async_httpx_client
        mock_client.send.side_effect = httpx.ConnectError("Connection failed")

        client = AsyncClientBase("http://example.com", client=mock_client)

        with pytest.raises(NetworkError):
            await client._make_request("/test")

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

        result = await client._make_request("/test", response_model=SampleModel)

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
        mock_response.content = b"not valid json"
        mock_response.json.side_effect = ValueError("Invalid JSON")

        client = AsyncClientBase("http://example.com", client=mock_client)

        with pytest.raises(ApplicationError) as exc_info:
            await client._make_request("/test")

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

        result = await client._make_request("/test", response_model=Any)
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

        result = await client._make_request("/test", response_model="Any")
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
            await client._make_request("/test", response_model=SampleModel)

        assert "Response validation failed" in str(exc_info.value)
        assert hasattr(exc_info.value, "validation_errors")


# ---------------------------------------------------------------------------
# Regression tests for the transport refactor invariants.
# ---------------------------------------------------------------------------


class _CountingShapeClient:
    """Sync custom Client that counts Requests it sees."""

    def __init__(self) -> None:
        self.requests: list[Request] = []

    def request(self, request: Request) -> Response:
        self.requests.append(request)
        return Response(
            status=200,
            headers=httpx.Headers({"content-type": "application/json"}),
            body=b'{"name":"test","age":25}',
        )


class _AsyncCountingShapeClient:
    def __init__(self) -> None:
        self.requests: list[Request] = []

    async def request(self, request: Request) -> Response:
        self.requests.append(request)
        return Response(
            status=200,
            headers=httpx.Headers({"content-type": "application/json"}),
            body=b'{"name":"test","age":25}',
        )


class TestSingleClientRequestBuild:
    """``_build_client_request`` should run exactly once per ``_make_request``,
    on either transport branch — protects against the double-build the
    earlier shape (path="" placeholder + later rebuild) caused.
    """

    def test_sync_custom_client_path_builds_once(self):
        shape_client = _CountingShapeClient()
        client = ClientBase("http://example.com", client=shape_client)

        client._make_request("/users", json_data={"a": 1})

        assert len(shape_client.requests) == 1
        assert shape_client.requests[0].path == "/users"

    def test_sync_httpx_path_builds_client_request_once(self, mock_httpx_client):
        from unittest.mock import patch

        mock_client, _ = mock_httpx_client
        client = ClientBase("http://example.com", client=mock_client)

        with patch.object(
            client,
            "_build_client_request",
            wraps=client._build_client_request,
        ) as spy:
            client._make_request(
                "/users", json_data={"a": 1}, response_model=SampleModel
            )
            assert spy.call_count == 1

    @pytest.mark.asyncio
    async def test_async_custom_client_path_builds_once(self):
        shape_client = _AsyncCountingShapeClient()
        client = AsyncClientBase("http://example.com", client=shape_client)

        await client._make_request("/users", json_data={"a": 1})

        assert len(shape_client.requests) == 1
        assert shape_client.requests[0].path == "/users"

    @pytest.mark.asyncio
    async def test_async_httpx_path_builds_client_request_once(
        self, mock_async_httpx_client
    ):
        from unittest.mock import patch

        mock_client, _ = mock_async_httpx_client
        client = AsyncClientBase("http://example.com", client=mock_client)

        with patch.object(
            client,
            "_build_client_request",
            wraps=client._build_client_request,
        ) as spy:
            await client._make_request(
                "/users", json_data={"a": 1}, response_model=SampleModel
            )
            assert spy.call_count == 1


class TestMiddlewareRunsOnBothTransports:
    """The headline contract of this refactor: middleware sees the same
    ``Request`` whether the underlying transport is httpx or a custom
    Client. Without this, the refactor's value-add isn't observable.
    """

    def test_sync_middleware_runs_on_custom_client(self):
        from reflectapi_runtime.middleware import SyncMiddleware

        seen: list[Request] = []

        class _Recorder(SyncMiddleware):
            def handle(self, request, next_call):
                seen.append(request)
                return next_call(request)

        shape = _CountingShapeClient()
        client = ClientBase(
            "http://example.com", client=shape, middleware=[_Recorder()]
        )
        client._make_request("/users", json_data={"a": 1})

        assert len(seen) == 1
        assert seen[0].path == "/users"
        assert len(shape.requests) == 1  # terminal also ran

    def test_sync_middleware_runs_on_httpx_transport(self, mock_httpx_client):
        from reflectapi_runtime.middleware import SyncMiddleware

        seen: list[Request] = []

        class _Recorder(SyncMiddleware):
            def handle(self, request, next_call):
                seen.append(request)
                return next_call(request)

        mock_client, _ = mock_httpx_client
        client = ClientBase(
            "http://example.com", client=mock_client, middleware=[_Recorder()]
        )
        client._make_request("/users", json_data={"a": 1})

        assert len(seen) == 1
        assert seen[0].path == "/users"

    @pytest.mark.asyncio
    async def test_async_middleware_runs_on_custom_client(self):
        from reflectapi_runtime.middleware import AsyncMiddleware

        seen: list[Request] = []

        class _Recorder(AsyncMiddleware):
            async def handle(self, request, next_call):
                seen.append(request)
                return await next_call(request)

        shape = _AsyncCountingShapeClient()
        client = AsyncClientBase(
            "http://example.com", client=shape, middleware=[_Recorder()]
        )
        await client._make_request("/users", json_data={"a": 1})

        assert len(seen) == 1
        assert seen[0].path == "/users"
        assert len(shape.requests) == 1


class TestDispatchUsesIsinstanceHttpxClient:
    """A custom Client that *happens* to expose ``build_request`` and
    ``send`` should still take the Protocol path — proving the dispatch
    is ``isinstance(httpx.Client)``, not duck-typed names.
    """

    def test_duck_typed_client_with_httpx_method_names_uses_protocol_path(self):
        class _DucktypedClient:
            def __init__(self) -> None:
                self.requests: list[Request] = []

            # Names that match httpx but are unrelated; would crash if
            # the dispatch wrongly forwarded into httpx APIs.
            def build_request(self, *args, **kwargs):  # noqa: ARG002
                raise AssertionError("must not be called: not httpx.Client")

            def send(self, *args, **kwargs):  # noqa: ARG002
                raise AssertionError("must not be called: not httpx.Client")

            def request(self, request: Request) -> Response:
                self.requests.append(request)
                return Response(
                    status=200,
                    headers=httpx.Headers({"content-type": "application/json"}),
                    body=b'{"name":"x","age":1}',
                )

        ducked = _DucktypedClient()
        client = ClientBase("http://example.com", client=ducked)
        client._make_request("/test", response_model=SampleModel)
        assert len(ducked.requests) == 1


class TestRawResponsePreserved:
    """``metadata.raw_response`` should be the real ``httpx.Response`` from
    the wire (with ``.request`` / ``.extensions`` / ``.history`` populated),
    not a synthetic ``httpx.Response`` rebuilt from the transport DTO.
    """

    def test_sync_metadata_raw_response_carries_request(self):
        # Use a real httpx.Client backed by a MockTransport so the
        # response is a wire-shaped httpx.Response with .request set.
        def handler(request: httpx.Request) -> httpx.Response:
            return httpx.Response(200, content=b'{"name":"x","age":1}')

        transport = httpx.MockTransport(handler)
        with httpx.Client(transport=transport) as raw:
            client = ClientBase("http://example.com", client=raw)
            result = client._make_request("/test", response_model=SampleModel)

        assert result.metadata.raw_response.request is not None
        assert str(result.metadata.raw_response.request.url) == (
            "http://example.com/test"
        )

    @pytest.mark.asyncio
    async def test_async_metadata_raw_response_carries_request(self):
        def handler(request: httpx.Request) -> httpx.Response:
            return httpx.Response(200, content=b'{"name":"x","age":1}')

        transport = httpx.MockTransport(handler)
        async with httpx.AsyncClient(transport=transport) as raw:
            client = AsyncClientBase("http://example.com", client=raw)
            result = await client._make_request(
                "/test", response_model=SampleModel
            )

        assert result.metadata.raw_response.request is not None
        assert str(result.metadata.raw_response.request.url) == (
            "http://example.com/test"
        )


class TestCustomTransportReturningHttpxResponse:
    """For backward compatibility, a custom transport that returns an
    ``httpx.Response`` (instead of the structural ``Response`` DTO) is
    adapted at the boundary rather than failing on attribute access.
    """

    def test_sync_transport_returning_httpx_response(self):
        class _HttpxReturningClient:
            def request(self, request: Request):
                return httpx.Response(
                    200,
                    headers={"content-type": "application/json"},
                    content=b'{"name":"x","age":1}',
                )

        client = ClientBase("http://example.com", client=_HttpxReturningClient())
        result = client._make_request("/test", response_model=SampleModel)
        assert result.value.name == "x"

    @pytest.mark.asyncio
    async def test_async_transport_returning_httpx_response(self):
        class _AsyncHttpxReturningClient:
            async def request(self, request: Request):
                return httpx.Response(
                    200,
                    headers={"content-type": "application/json"},
                    content=b'{"name":"x","age":1}',
                )

        client = AsyncClientBase(
            "http://example.com", client=_AsyncHttpxReturningClient()
        )
        result = await client._make_request("/test", response_model=SampleModel)
        assert result.value.name == "x"


class TestMiddlewareTransformsAreParsed:
    """Middleware that transforms the response body must be honoured by
    the parsing path. The earlier shape leaked ``Response.raw`` through to
    parsing — if middleware replaced ``body`` via ``dataclasses.replace``,
    ``raw`` was carried forward and the original wire bytes were parsed
    instead. ``raw`` is now strictly a metadata sidecar.
    """

    def test_sync_body_rewrite_is_visible_to_caller(self):
        from dataclasses import replace

        from reflectapi_runtime.middleware import SyncMiddleware

        class _Rewrite(SyncMiddleware):
            def handle(self, request, next_call):
                resp = next_call(request)
                # `replace` would naturally carry `raw` along — the runtime
                # must not be tempted by it for body parsing.
                return replace(resp, body=b'{"name":"middleware","age":2}')

        def handler(request: httpx.Request) -> httpx.Response:
            return httpx.Response(200, content=b'{"name":"wire","age":1}')

        with httpx.Client(transport=httpx.MockTransport(handler)) as raw:
            client = ClientBase(
                "http://example.com", client=raw, middleware=[_Rewrite()]
            )
            result = client._make_request("/test", response_model=SampleModel)

        assert result.value.name == "middleware"
        assert result.value.age == 2

    @pytest.mark.asyncio
    async def test_async_body_rewrite_is_visible_to_caller(self):
        from dataclasses import replace

        from reflectapi_runtime.middleware import AsyncMiddleware

        class _Rewrite(AsyncMiddleware):
            async def handle(self, request, next_call):
                resp = await next_call(request)
                return replace(resp, body=b'{"name":"middleware","age":2}')

        def handler(request: httpx.Request) -> httpx.Response:
            return httpx.Response(200, content=b'{"name":"wire","age":1}')

        async with httpx.AsyncClient(transport=httpx.MockTransport(handler)) as raw:
            client = AsyncClientBase(
                "http://example.com", client=raw, middleware=[_Rewrite()]
            )
            result = await client._make_request(
                "/test", response_model=SampleModel
            )

        assert result.value.name == "middleware"
        assert result.value.age == 2

    def test_sync_status_rewrite_is_visible(self):
        """Middleware that flips a 5xx into a 200 should suppress the
        ApplicationError path — proves status comes from the structural
        response, not from `raw`.
        """
        from dataclasses import replace

        from reflectapi_runtime.middleware import SyncMiddleware

        class _Heal(SyncMiddleware):
            def handle(self, request, next_call):
                resp = next_call(request)
                if resp.status >= 500:
                    return replace(
                        resp, status=200, body=b'{"name":"healed","age":0}'
                    )
                return resp

        def handler(request: httpx.Request) -> httpx.Response:
            return httpx.Response(503, content=b"server is sad")

        with httpx.Client(transport=httpx.MockTransport(handler)) as raw:
            client = ClientBase(
                "http://example.com", client=raw, middleware=[_Heal()]
            )
            result = client._make_request("/test", response_model=SampleModel)

        assert result.value.name == "healed"
        assert result.metadata.status_code == 200
