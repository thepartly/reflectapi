"""Base client classes for ReflectAPI Python clients."""

from __future__ import annotations

import datetime
import json
import time
from abc import ABC
from collections.abc import AsyncIterator, Awaitable, Callable, Iterator
from typing import Any, TypeVar, overload

import httpx
from pydantic import BaseModel, TypeAdapter
from pydantic import ValidationError as PydanticValidationError

from .auth import AuthHandler
from .exceptions import ApplicationError, NetworkError, TimeoutError, ValidationError
from .middleware import (
    AsyncMiddleware,
    AsyncMiddlewareChain,
    SyncMiddleware,
    SyncMiddlewareChain,
)
from .response import ApiResponse, TransportMetadata
from .sse import aparse_sse, parse_sse
from .transport import AsyncClient, Client, Request, Response


# Sentinel object to represent "no validation needed"
class _NoValidation:
    pass


NO_VALIDATION = _NoValidation()

T = TypeVar("T", bound=BaseModel)


def _json_serializer(obj: Any) -> Any:
    """JSON serializer function for datetime and Pydantic objects."""
    if isinstance(obj, datetime.datetime):
        return obj.isoformat()
    elif isinstance(obj, datetime.date):
        return obj.isoformat()
    elif hasattr(obj, "model_dump"):
        return obj.model_dump(exclude_none=True)
    raise TypeError(f"Object of type {type(obj)} is not JSON serializable")


# Note: AsyncAuthWrapper removed - AuthHandler now inherits from httpx.Auth directly


class ClientBase(ABC):
    """Base class for synchronous ReflectAPI clients."""

    def __init__(
        self,
        base_url: str,
        *,
        timeout: float | None = 30.0,
        headers: dict[str, str] | None = None,
        middleware: list[SyncMiddleware] | None = None,
        auth: AuthHandler | httpx.Auth | None = None,
        client: Client | httpx.Client | None = None,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self.middleware_chain = SyncMiddlewareChain(middleware or [])
        self.auth = auth

        # Use provided client or create a new one
        if client is not None:
            self._client = client
            self._owns_client = False
        else:
            # Handle authentication
            auth_param = None
            if isinstance(auth, AuthHandler):
                # Use our custom auth handler as httpx auth
                auth_param = auth
            elif auth is not None:
                # Use httpx built-in auth directly
                auth_param = auth

            self._client = httpx.Client(
                base_url=self.base_url,
                timeout=timeout,
                headers=headers or {},
                auth=auth_param,
            )
            self._owns_client = True

    def __enter__(self) -> ClientBase:
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        self.close()

    def close(self) -> None:
        """Close the underlying HTTP client if we own it."""
        if self._owns_client:
            self._client.close()

    @classmethod
    def from_bearer_token(
        cls,
        base_url: str,
        token: str,
        **kwargs: Any,
    ) -> ClientBase:
        """Create a client with Bearer token authentication."""
        from .auth import BearerTokenAuth

        return cls(base_url, auth=BearerTokenAuth(token), **kwargs)

    @classmethod
    def from_api_key(
        cls,
        base_url: str,
        api_key: str,
        header_name: str = "X-API-Key",
        param_name: str | None = None,
        **kwargs: Any,
    ) -> ClientBase:
        """Create a client with API key authentication."""
        from .auth import APIKeyAuth

        return cls(
            base_url, auth=APIKeyAuth(api_key, header_name, param_name), **kwargs
        )

    @classmethod
    def from_basic_auth(
        cls,
        base_url: str,
        username: str,
        password: str,
        **kwargs: Any,
    ) -> ClientBase:
        """Create a client with HTTP Basic authentication."""
        from .auth import BasicAuth

        return cls(base_url, auth=BasicAuth(username, password), **kwargs)

    @classmethod
    def from_oauth2_client_credentials(
        cls,
        base_url: str,
        token_url: str,
        client_id: str,
        client_secret: str,
        scope: str | None = None,
        **kwargs: Any,
    ) -> ClientBase:
        """Create a client with OAuth2 client credentials authentication."""
        from .auth import OAuth2ClientCredentialsAuth

        return cls(
            base_url,
            auth=OAuth2ClientCredentialsAuth(
                token_url, client_id, client_secret, scope
            ),
            **kwargs,
        )

    @overload
    def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: None = None,
        headers_model: BaseModel | None = None,
        response_model: type[T],
    ) -> ApiResponse[T]: ...

    @overload
    def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: None = None,
        headers_model: BaseModel | None = None,
        response_model: None = None,
    ) -> ApiResponse[dict[str, Any]]: ...

    @overload
    def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: None = None,
        headers_model: BaseModel | None = None,
        response_model: type[Any],
    ) -> ApiResponse[dict[str, Any]]: ...

    @overload
    def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: None = None,
        response_model: str,
    ) -> ApiResponse[dict[str, Any]]: ...

    @overload
    def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: None = None,
        json_model: BaseModel,
        headers_model: BaseModel | None = None,
        response_model: type[T],
    ) -> ApiResponse[T]: ...

    @overload
    def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: None = None,
        json_model: BaseModel,
        headers_model: BaseModel | None = None,
        response_model: None = None,
    ) -> ApiResponse[dict[str, Any]]: ...

    def _validate_request_params(
        self,
        json_data: Any | None,
        json_model: BaseModel | None,
    ) -> None:
        """Validate request parameters for conflicts."""
        if json_model is not None and json_data is not None:
            raise ValueError("Cannot specify both json_data and json_model")

    def _serialize_request_body(
        self, json_model: BaseModel | int | float | str | bool | list | dict
    ) -> tuple[bytes, dict[str, str]]:
        """Serialize the request body.

        ``ReflectapiPartialModel`` subclasses carry their own
        ``@model_serializer`` that omits keys not in
        ``model_fields_set``, so explicit ``None`` values must reach
        the wire (they encode the protocol's "explicit null" state).

        Plain Pydantic models use ``exclude_none=True`` so an unset
        optional field renders as an absent key rather than a JSON
        ``null`` — matching what serde sees as
        ``#[serde(skip_serializing_if = "Option::is_none")]`` and
        preserving the historical wire format.
        """
        # Handle primitive types (for untagged unions)
        if not hasattr(json_model, "model_dump_json"):
            content = json.dumps(
                json_model, default=_json_serializer, separators=(",", ":")
            ).encode("utf-8")
            return content, {"Content-Type": "application/json"}

        from .partial import ReflectapiPartialModel

        if isinstance(json_model, ReflectapiPartialModel):
            content = json_model.model_dump_json(by_alias=True).encode("utf-8")
        else:
            content = json_model.model_dump_json(
                by_alias=True, exclude_none=True
            ).encode("utf-8")
        return content, {"Content-Type": "application/json"}

    def _build_headers(
        self, base_headers: dict[str, str], headers_model: BaseModel | None
    ) -> dict[str, str]:
        """Build complete headers dict including custom headers from headers_model."""
        headers = base_headers.copy()

        # Add headers from headers_model if provided
        if headers_model is not None:
            header_dict = headers_model.model_dump(by_alias=True, exclude_unset=True)
            for key, value in header_dict.items():
                if value is not None:
                    headers[key] = str(value)

        return headers

    def _build_client_request(
        self,
        path: str,
        json_data: Any | None,
        json_model: BaseModel | None,
        headers_model: BaseModel | None,
    ) -> Request:
        """Build ReflectAPI transport request object."""
        if json_model is not None:
            content, base_headers = self._serialize_request_body(json_model)
            headers = self._build_headers(base_headers, headers_model)
        elif json_data is not None:
            content = json.dumps(
                json_data,
                default=_json_serializer,
                separators=(",", ":"),
            ).encode("utf-8")
            headers = self._build_headers(
                {"Content-Type": "application/json"}, headers_model
            )
        else:
            content = b""
            headers = self._build_headers({}, headers_model)

        return Request(path=path, headers=headers, body=content)

    def _build_httpx_request(
        self,
        request: Request,
        url: str,
        params: dict[str, Any] | None,
    ) -> httpx.Request:
        """Materialise an httpx.Request from a transport Request.

        Method is always POST by design.
        """
        request_kwargs: dict[str, Any] = {
            "method": "POST",
            "url": url,
            "params": params,
            "headers": request.headers if request.headers else None,
        }
        if request.body:
            request_kwargs["content"] = request.body
        return self._client.build_request(**request_kwargs)

    def _make_terminal(
        self,
        params: dict[str, Any] | None,
    ) -> Callable[[Request], Response]:
        """Return the chain-terminal handler for the active transport.

        For an ``httpx.Client``, the terminal lifts the Request into an
        ``httpx.Request``, sends it, and surfaces the real
        ``httpx.Response`` via ``Response.raw`` so callers can read its
        ``request`` / ``extensions`` / ``history`` from
        :class:`TransportMetadata`.

        For a custom :class:`Client`, the terminal accepts either a
        :class:`Response` (the Protocol contract) or an ``httpx.Response``;
        the latter is adapted at the boundary for transports that wrap
        httpx directly. *Transitional:* the httpx-shaped path is a
        backward-compat shim for adapters written against the previous
        runtime; new code should return a :class:`Response`.
        """
        if isinstance(self._client, httpx.Client):
            base_url = self.base_url

            def terminal(req: Request) -> Response:
                url = f"{base_url}/{req.path.lstrip('/')}"
                httpx_req = self._build_httpx_request(req, url, params)
                httpx_resp = self._client.send(httpx_req)
                return Response(
                    status=httpx_resp.status_code,
                    headers=httpx_resp.headers,
                    body=httpx_resp.content,
                    raw=httpx_resp,
                )

            return terminal

        def terminal(req: Request) -> Response:
            response = self._client.request(req)
            if isinstance(response, httpx.Response):
                return Response(
                    status=response.status_code,
                    headers=response.headers,
                    body=response.content,
                    raw=response,
                )
            return response

        return terminal

    def _handle_error_response(
        self,
        response: httpx.Response,
        metadata: TransportMetadata,
        error_model: type | None = None,
    ) -> None:
        """Handle HTTP error responses (4xx, 5xx).

        If error_model is provided, attempts to deserialize the error body
        into a typed Pydantic model before raising ApplicationError.
        """
        if response.status_code >= 400:
            error_data = None
            typed_error = None
            try:
                error_data = response.json()
            except Exception:
                pass

            # Try typed error deserialization
            if error_model is not None and error_data is not None:
                try:
                    ta = TypeAdapter(error_model)
                    typed_error = ta.validate_python(error_data)
                except Exception:
                    pass  # Fall back to raw error_data

            raise ApplicationError.from_response(
                response, metadata, error_data, typed_error=typed_error
            )

    def _parse_json_response(self, response: httpx.Response) -> dict[str, Any]:
        """Parse JSON response with error handling."""
        try:
            return response.json()
        except Exception as e:
            raise ValidationError(
                f"Failed to parse JSON response: {e}",
                cause=e,
            )

    def _validate_response_model(
        self,
        response: httpx.Response,
        response_model: type[T] | type[Any] | str | _NoValidation,
        metadata: TransportMetadata,
    ) -> ApiResponse[T] | ApiResponse[dict[str, Any]]:
        """Validate response using Pydantic model via TypeAdapter.

        TypeAdapter handles all types: plain BaseModel, Generic types
        (list[Model], dict[str, Model]), Union types, and primitives.
        Uses validate_json for performance when raw bytes are available.
        """
        # Handle special cases where no validation is needed
        if response_model == "Any" or response_model is NO_VALIDATION:
            json_response = self._parse_json_response(response)
            return ApiResponse(json_response, metadata)

        if response_model is Any:
            json_response = self._parse_json_response(response)
            return ApiResponse(json_response, metadata)

        try:
            ta = TypeAdapter(response_model)
            # Prefer validate_json for Pydantic's fast Rust-based JSON parsing
            content = response.content
            if isinstance(content, (bytes, bytearray)):
                validated_data = ta.validate_json(content)
            else:
                json_response = self._parse_json_response(response)
                validated_data = ta.validate_python(json_response)
            return ApiResponse(validated_data, metadata)
        except PydanticValidationError as e:
            raise ValidationError(
                f"Response validation failed: {e}",
                validation_errors=e.errors(),
                cause=e,
            )

    def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: BaseModel | None = None,
        headers_model: BaseModel | None = None,
        response_model: type[T] | type[Any] | str | _NoValidation | None = None,
        error_model: type | None = None,
    ) -> ApiResponse[T] | ApiResponse[dict[str, Any]]:
        """Make an HTTP request and return an ApiResponse."""
        # Validate request parameters
        self._validate_request_params(json_data, json_model)

        # Build the transport request once and drive it through the
        # middleware chain. The chain's terminal handler is transport-
        # specific; middleware itself is transport-agnostic.
        request = self._build_client_request(
            path, json_data, json_model, headers_model
        )
        terminal = self._make_terminal(params)

        # Execute request with timing
        start_time = time.time()

        try:
            client_response = self.middleware_chain.execute(request, terminal)
            # Body / status / headers are read from the structural
            # response so any middleware transforms apply; `raw` is used
            # only as the metadata sidecar (preserves `.request` /
            # `.extensions` / `.history` from the real wire response when
            # available, fallback to the synthetic for custom transports).
            parsed_response = httpx.Response(
                status_code=client_response.status,
                headers=client_response.headers,
                content=client_response.body,
            )
            metadata = TransportMetadata(
                status_code=client_response.status,
                headers=client_response.headers,
                timing=time.time() - start_time,
                raw_response=client_response.raw or parsed_response,
            )

            # Handle error responses
            self._handle_error_response(
                parsed_response, metadata, error_model=error_model
            )

            # Validate and return response
            if response_model is not None:
                return self._validate_response_model(
                    parsed_response, response_model, metadata
                )
            else:
                # No response_model provided - parse JSON into dict
                json_response = self._parse_json_response(parsed_response)
                return ApiResponse(json_response, metadata)

        except httpx.TimeoutException as e:
            raise TimeoutError.from_httpx_timeout(e)
        except httpx.RequestError as e:
            raise NetworkError.from_httpx_error(e)

    def _make_sse_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: BaseModel | None = None,
        headers_model: BaseModel | None = None,
        item_model: type[T] | type[Any] | str | _NoValidation | None = None,
        error_model: type | None = None,
    ) -> Iterator[T] | Iterator[Any]:
        """Open an SSE stream and yield items validated against ``item_model``.

        Errors raised when the server returns a non-2xx response are
        ``ApplicationError`` (matching the non-streaming flow); transport
        problems raise ``NetworkError`` / ``TimeoutError``. Per-event
        validation failures raise ``ValidationError``.

        The HTTP connection is released when the generator is exhausted,
        ``close()``-d, or garbage-collected, so consumers can ``break`` out
        of the loop without leaking sockets. Middleware is intentionally
        bypassed because typical middleware buffers the response body.
        """

        self._validate_request_params(json_data, json_model)

        url = f"{self.base_url}/{path.lstrip('/')}"
        client_request = self._build_client_request(
            path, json_data, json_model, headers_model
        )
        request = self._build_httpx_request(client_request, url, params)
        request.headers["accept"] = "text/event-stream"

        start_time = time.time()
        try:
            response = self._client.send(request, stream=True)
        except httpx.TimeoutException as e:
            raise TimeoutError.from_httpx_timeout(e)
        except httpx.RequestError as e:
            raise NetworkError.from_httpx_error(e)

        try:
            metadata = TransportMetadata.from_response(response, start_time)
            if response.status_code >= 400:
                # read_full body so error parsing can see it
                response.read()
                self._handle_error_response(response, metadata, error_model=error_model)

            adapter: TypeAdapter[Any] | None = None
            if (
                item_model is not None
                and item_model is not NO_VALIDATION
                and item_model != "Any"
                and item_model is not Any
            ):
                adapter = TypeAdapter(item_model)

            try:
                for event in parse_sse(response.iter_lines()):
                    if adapter is None:
                        yield json.loads(event.data)
                    else:
                        try:
                            yield adapter.validate_json(event.data)
                        except PydanticValidationError as e:
                            raise ValidationError(
                                f"SSE event validation failed: {e}",
                                validation_errors=e.errors(),
                                cause=e,
                            )
            except httpx.TimeoutException as e:
                raise TimeoutError.from_httpx_timeout(e)
            except httpx.RequestError as e:
                raise NetworkError.from_httpx_error(e)
        finally:
            response.close()


class AsyncClientBase(ABC):
    """Base class for asynchronous ReflectAPI clients."""

    def __init__(
        self,
        base_url: str,
        *,
        timeout: float | None = 30.0,
        headers: dict[str, str] | None = None,
        middleware: list[AsyncMiddleware] | None = None,
        auth: AuthHandler | httpx.Auth | None = None,
        client: AsyncClient | httpx.AsyncClient | None = None,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self.middleware_chain = AsyncMiddlewareChain(middleware or [])
        self.auth = auth

        # Use provided client or create a new one
        if client is not None:
            self._client = client
            self._owns_client = False
        else:
            # Handle authentication for async client
            auth_param = None
            if isinstance(auth, AuthHandler):
                # Create wrapper for async auth handler
                auth_param = auth  # AuthHandler now inherits from httpx.Auth
            elif auth is not None:
                # Use httpx built-in auth directly
                auth_param = auth

            self._client = httpx.AsyncClient(
                base_url=self.base_url,
                timeout=timeout,
                headers=headers or {},
                auth=auth_param,
            )
            self._owns_client = True

    async def __aenter__(self) -> AsyncClientBase:
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        await self.aclose()

    async def aclose(self) -> None:
        """Close the underlying HTTP client if we own it."""
        if self._owns_client:
            await self._client.aclose()

    @classmethod
    def from_bearer_token(
        cls,
        base_url: str,
        token: str,
        **kwargs: Any,
    ) -> AsyncClientBase:
        """Create a client with Bearer token authentication."""
        from .auth import BearerTokenAuth

        return cls(base_url, auth=BearerTokenAuth(token), **kwargs)

    @classmethod
    def from_api_key(
        cls,
        base_url: str,
        api_key: str,
        header_name: str = "X-API-Key",
        param_name: str | None = None,
        **kwargs: Any,
    ) -> AsyncClientBase:
        """Create a client with API key authentication."""
        from .auth import APIKeyAuth

        return cls(
            base_url, auth=APIKeyAuth(api_key, header_name, param_name), **kwargs
        )

    @classmethod
    def from_basic_auth(
        cls,
        base_url: str,
        username: str,
        password: str,
        **kwargs: Any,
    ) -> AsyncClientBase:
        """Create a client with HTTP Basic authentication."""
        from .auth import BasicAuth

        return cls(base_url, auth=BasicAuth(username, password), **kwargs)

    @classmethod
    def from_oauth2_client_credentials(
        cls,
        base_url: str,
        token_url: str,
        client_id: str,
        client_secret: str,
        scope: str | None = None,
        **kwargs: Any,
    ) -> AsyncClientBase:
        """Create a client with OAuth2 client credentials authentication."""
        from .auth import OAuth2ClientCredentialsAuth

        return cls(
            base_url,
            auth=OAuth2ClientCredentialsAuth(
                token_url, client_id, client_secret, scope
            ),
            **kwargs,
        )

    @overload
    async def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: None = None,
        response_model: type[T],
    ) -> ApiResponse[T]: ...

    @overload
    async def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: None = None,
        headers_model: BaseModel | None = None,
        response_model: None = None,
    ) -> ApiResponse[dict[str, Any]]: ...

    @overload
    async def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: None = None,
        headers_model: BaseModel | None = None,
        response_model: type[Any],
    ) -> ApiResponse[dict[str, Any]]: ...

    @overload
    async def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: None = None,
        response_model: str,
    ) -> ApiResponse[dict[str, Any]]: ...

    @overload
    async def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: None = None,
        json_model: BaseModel,
        headers_model: BaseModel | None = None,
        response_model: type[T],
    ) -> ApiResponse[T]: ...

    @overload
    async def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: None = None,
        json_model: BaseModel,
        headers_model: BaseModel | None = None,
        response_model: None = None,
    ) -> ApiResponse[dict[str, Any]]: ...

    def _validate_request_params(
        self,
        json_data: Any | None,
        json_model: BaseModel | None,
    ) -> None:
        """Validate request parameters for conflicts."""
        if json_model is not None and json_data is not None:
            raise ValueError("Cannot specify both json_data and json_model")

    def _serialize_request_body(
        self, json_model: BaseModel | int | float | str | bool | list | dict
    ) -> tuple[bytes, dict[str, str]]:
        """Serialize the request body.

        ``ReflectapiPartialModel`` subclasses carry their own
        ``@model_serializer`` that omits keys not in
        ``model_fields_set``, so explicit ``None`` values must reach
        the wire (they encode the protocol's "explicit null" state).

        Plain Pydantic models use ``exclude_none=True`` so an unset
        optional field renders as an absent key rather than a JSON
        ``null`` — matching what serde sees as
        ``#[serde(skip_serializing_if = "Option::is_none")]`` and
        preserving the historical wire format.
        """
        # Handle primitive types (for untagged unions)
        if not hasattr(json_model, "model_dump_json"):
            content = json.dumps(
                json_model, default=_json_serializer, separators=(",", ":")
            ).encode("utf-8")
            return content, {"Content-Type": "application/json"}

        from .partial import ReflectapiPartialModel

        if isinstance(json_model, ReflectapiPartialModel):
            content = json_model.model_dump_json(by_alias=True).encode("utf-8")
        else:
            content = json_model.model_dump_json(
                by_alias=True, exclude_none=True
            ).encode("utf-8")
        return content, {"Content-Type": "application/json"}

    def _build_headers(
        self, base_headers: dict[str, str], headers_model: BaseModel | None
    ) -> dict[str, str]:
        """Build complete headers dict including custom headers from headers_model."""
        headers = base_headers.copy()

        # Add headers from headers_model if provided
        if headers_model is not None:
            header_dict = headers_model.model_dump(by_alias=True, exclude_unset=True)
            for key, value in header_dict.items():
                if value is not None:
                    headers[key] = str(value)

        return headers

    def _build_client_request(
        self,
        path: str,
        json_data: Any | None,
        json_model: BaseModel | None,
        headers_model: BaseModel | None,
    ) -> Request:
        """Build ReflectAPI transport request object."""
        if json_model is not None:
            content, base_headers = self._serialize_request_body(json_model)
            headers = self._build_headers(base_headers, headers_model)
        elif json_data is not None:
            content = json.dumps(
                json_data,
                default=_json_serializer,
                separators=(",", ":"),
            ).encode("utf-8")
            headers = self._build_headers(
                {"Content-Type": "application/json"}, headers_model
            )
        else:
            content = b""
            headers = self._build_headers({}, headers_model)

        return Request(path=path, headers=headers, body=content)

    def _build_httpx_request(
        self,
        request: Request,
        url: str,
        params: dict[str, Any] | None,
    ) -> httpx.Request:
        """Materialise an httpx.Request from a transport Request.

        Method is always POST by design.
        """
        request_kwargs: dict[str, Any] = {
            "method": "POST",
            "url": url,
            "params": params,
            "headers": request.headers if request.headers else None,
        }
        if request.body:
            request_kwargs["content"] = request.body
        return self._client.build_request(**request_kwargs)

    def _make_terminal(
        self,
        params: dict[str, Any] | None,
    ) -> Callable[[Request], Awaitable[Response]]:
        """Return the chain-terminal handler for the active transport.

        For an ``httpx.AsyncClient``, the terminal lifts the Request into
        an ``httpx.Request``, sends it, and surfaces the real
        ``httpx.Response`` via ``Response.raw``.

        For a custom :class:`AsyncClient`, the terminal accepts either a
        :class:`Response` or an ``httpx.Response``; the latter is a
        *transitional* backward-compat shim for adapters written against
        the previous runtime.
        """
        if isinstance(self._client, httpx.AsyncClient):
            base_url = self.base_url

            async def terminal(req: Request) -> Response:
                url = f"{base_url}/{req.path.lstrip('/')}"
                httpx_req = self._build_httpx_request(req, url, params)
                httpx_resp = await self._client.send(httpx_req)
                return Response(
                    status=httpx_resp.status_code,
                    headers=httpx_resp.headers,
                    body=httpx_resp.content,
                    raw=httpx_resp,
                )

            return terminal

        async def terminal(req: Request) -> Response:
            response = await self._client.request(req)
            if isinstance(response, httpx.Response):
                return Response(
                    status=response.status_code,
                    headers=response.headers,
                    body=response.content,
                    raw=response,
                )
            return response

        return terminal

    def _handle_error_response(
        self,
        response: httpx.Response,
        metadata: TransportMetadata,
        error_model: type | None = None,
    ) -> None:
        """Handle HTTP error responses (4xx, 5xx)."""
        if response.status_code >= 400:
            error_data = None
            typed_error = None
            try:
                error_data = response.json()
            except Exception:
                pass

            if error_model is not None and error_data is not None:
                try:
                    ta = TypeAdapter(error_model)
                    typed_error = ta.validate_python(error_data)
                except Exception:
                    pass

            raise ApplicationError.from_response(
                response, metadata, error_data, typed_error=typed_error
            )

    def _parse_json_response(self, response: httpx.Response) -> dict[str, Any]:
        """Parse JSON response with error handling."""
        try:
            return response.json()
        except Exception as e:
            raise ValidationError(
                f"Failed to parse JSON response: {e}",
                cause=e,
            )

    def _validate_response_model(
        self,
        response: httpx.Response,
        response_model: type[T] | type[Any] | str | _NoValidation,
        metadata: TransportMetadata,
    ) -> ApiResponse[T] | ApiResponse[dict[str, Any]]:
        """Validate response using Pydantic model via TypeAdapter."""
        if response_model == "Any" or response_model is NO_VALIDATION:
            json_response = self._parse_json_response(response)
            return ApiResponse(json_response, metadata)

        if response_model is Any:
            json_response = self._parse_json_response(response)
            return ApiResponse(json_response, metadata)

        try:
            ta = TypeAdapter(response_model)
            content = response.content
            if isinstance(content, (bytes, bytearray)):
                validated_data = ta.validate_json(content)
            else:
                json_response = self._parse_json_response(response)
                validated_data = ta.validate_python(json_response)
            return ApiResponse(validated_data, metadata)
        except PydanticValidationError as e:
            raise ValidationError(
                f"Response validation failed: {e}",
                validation_errors=e.errors(),
                cause=e,
            )

    async def _make_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: BaseModel | None = None,
        headers_model: BaseModel | None = None,
        response_model: type[T] | type[Any] | str | _NoValidation | None = None,
        error_model: type | None = None,
    ) -> ApiResponse[T] | ApiResponse[dict[str, Any]]:
        """Make an HTTP request and return an ApiResponse."""
        # Validate request parameters
        self._validate_request_params(json_data, json_model)

        # Build the transport request once and drive it through the
        # middleware chain. The chain's terminal handler is transport-
        # specific; middleware itself is transport-agnostic.
        request = self._build_client_request(
            path, json_data, json_model, headers_model
        )
        terminal = self._make_terminal(params)

        # Execute request with timing
        start_time = time.time()

        try:
            client_response = await self.middleware_chain.execute(request, terminal)
            # Body / status / headers are read from the structural
            # response so any middleware transforms apply; `raw` is used
            # only as the metadata sidecar (preserves `.request` /
            # `.extensions` / `.history` from the real wire response when
            # available, fallback to the synthetic for custom transports).
            parsed_response = httpx.Response(
                status_code=client_response.status,
                headers=client_response.headers,
                content=client_response.body,
            )
            metadata = TransportMetadata(
                status_code=client_response.status,
                headers=client_response.headers,
                timing=time.time() - start_time,
                raw_response=client_response.raw or parsed_response,
            )

            # Handle error responses
            self._handle_error_response(
                parsed_response, metadata, error_model=error_model
            )

            # Validate and return response
            if response_model is not None:
                return self._validate_response_model(
                    parsed_response, response_model, metadata
                )
            else:
                json_response = self._parse_json_response(parsed_response)
                return ApiResponse(json_response, metadata)

        except httpx.TimeoutException as e:
            raise TimeoutError.from_httpx_timeout(e)
        except httpx.RequestError as e:
            raise NetworkError.from_httpx_error(e)

    async def _make_sse_request(
        self,

        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: BaseModel | None = None,
        headers_model: BaseModel | None = None,
        item_model: type[T] | type[Any] | str | _NoValidation | None = None,
        error_model: type | None = None,
    ) -> AsyncIterator[T] | AsyncIterator[Any]:
        """Open an SSE stream and yield items validated against ``item_model``.

        Errors raised when the server returns a non-2xx response are
        ``ApplicationError`` (matching the non-streaming flow); transport
        problems raise ``NetworkError`` / ``TimeoutError``. Per-event
        validation failures raise ``ValidationError``.

        The HTTP connection is released when the async generator is
        exhausted or ``aclose()``-d, so consumers can ``break`` out of the
        loop without leaking sockets. Middleware is intentionally
        bypassed because typical middleware buffers the response body.
        """

        self._validate_request_params(json_data, json_model)

        url = f"{self.base_url}/{path.lstrip('/')}"
        client_request = self._build_client_request(
            path, json_data, json_model, headers_model
        )
        request = self._build_httpx_request(client_request, url, params)
        request.headers["accept"] = "text/event-stream"

        start_time = time.time()
        try:
            response = await self._client.send(request, stream=True)
        except httpx.TimeoutException as e:
            raise TimeoutError.from_httpx_timeout(e)
        except httpx.RequestError as e:
            raise NetworkError.from_httpx_error(e)

        try:
            metadata = TransportMetadata.from_response(response, start_time)
            if response.status_code >= 400:
                await response.aread()
                self._handle_error_response(response, metadata, error_model=error_model)

            adapter: TypeAdapter[Any] | None = None
            if (
                item_model is not None
                and item_model is not NO_VALIDATION
                and item_model != "Any"
                and item_model is not Any
            ):
                adapter = TypeAdapter(item_model)

            try:
                async for event in aparse_sse(response.aiter_lines()):
                    if adapter is None:
                        yield json.loads(event.data)
                    else:
                        try:
                            yield adapter.validate_json(event.data)
                        except PydanticValidationError as e:
                            raise ValidationError(
                                f"SSE event validation failed: {e}",
                                validation_errors=e.errors(),
                                cause=e,
                            )
            except httpx.TimeoutException as e:
                raise TimeoutError.from_httpx_timeout(e)
            except httpx.RequestError as e:
                raise NetworkError.from_httpx_error(e)
        finally:
            await response.aclose()
