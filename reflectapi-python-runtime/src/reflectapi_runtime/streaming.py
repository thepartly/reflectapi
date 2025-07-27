"""Streaming response handling for ReflectAPI Python clients."""

from __future__ import annotations

import time
from contextlib import asynccontextmanager
from typing import TYPE_CHECKING, Any

import httpx

from .auth import AuthHandler
from .exceptions import ApplicationError, NetworkError, TimeoutError
from .middleware import AsyncMiddleware, AsyncMiddlewareChain
from .option import serialize_option_dict
from .response import TransportMetadata

if TYPE_CHECKING:
    from collections.abc import AsyncIterator

    from pydantic import BaseModel


class StreamingResponse:
    """Wrapper for streaming HTTP responses."""

    def __init__(self, response: httpx.Response, metadata: TransportMetadata):
        self.response = response
        self.metadata = metadata
        self._closed = False

    @property
    def status_code(self) -> int:
        """HTTP status code."""
        return self.response.status_code

    @property
    def headers(self) -> httpx.Headers:
        """Response headers."""
        return self.response.headers

    @property
    def content_type(self) -> str | None:
        """Content-Type header value."""
        return self.response.headers.get("content-type")

    @property
    def content_length(self) -> int | None:
        """Content-Length header value if present."""
        try:
            length = self.response.headers.get("content-length")
            return int(length) if length else None
        except (ValueError, TypeError):
            return None

    @property
    def is_closed(self) -> bool:
        """Check if the response stream is closed."""
        return self._closed or self.response.is_closed

    async def aiter_bytes(self, chunk_size: int = 8192) -> AsyncIterator[bytes]:
        """Iterate over response content as bytes chunks."""
        if self.is_closed:
            raise RuntimeError("Cannot iterate over closed response")

        try:
            async for chunk in self.response.aiter_bytes(chunk_size):
                yield chunk
        finally:
            await self.aclose()

    async def aiter_text(self, chunk_size: int = 8192, encoding: str | None = None) -> AsyncIterator[str]:
        """Iterate over response content as text chunks."""
        if self.is_closed:
            raise RuntimeError("Cannot iterate over closed response")

        try:
            async for chunk in self.response.aiter_text(chunk_size, encoding):
                yield chunk
        finally:
            await self.aclose()

    async def aiter_lines(self, chunk_size: int = 8192, encoding: str | None = None) -> AsyncIterator[str]:
        """Iterate over response content line by line."""
        if self.is_closed:
            raise RuntimeError("Cannot iterate over closed response")

        try:
            async for line in self.response.aiter_lines(chunk_size):
                if encoding:
                    yield line.decode(encoding) if isinstance(line, bytes) else line
                else:
                    yield line
        finally:
            await self.aclose()

    async def save_to_file(self, file_path: str, chunk_size: int = 8192) -> int:
        """Save streaming content directly to a file.

        Args:
            file_path: Path where to save the file
            chunk_size: Size of chunks to read/write

        Returns:
            Number of bytes written
        """
        if self.is_closed:
            raise RuntimeError("Cannot save closed response")

        bytes_written = 0
        try:
            with open(file_path, 'wb') as f:
                async for chunk in self.aiter_bytes(chunk_size):
                    f.write(chunk)
                    bytes_written += len(chunk)
        except Exception:
            # Clean up partial file on error
            import os
            try:
                os.unlink(file_path)
            except OSError:
                pass
            raise

        return bytes_written

    async def read_all(self) -> bytes:
        """Read all content into memory. Use with caution for large responses."""
        if self.is_closed:
            raise RuntimeError("Cannot read closed response")

        try:
            return await self.response.aread()
        finally:
            await self.aclose()

    async def aclose(self) -> None:
        """Close the response stream."""
        if not self._closed:
            await self.response.aclose()
            self._closed = True


class AsyncStreamingClient:
    """Async client specialized for streaming responses."""

    def __init__(
        self,
        base_url: str,
        *,
        timeout: float | None = 120.0,  # Longer default timeout for streaming
        headers: dict[str, str] | None = None,
        middleware: list[AsyncMiddleware] | None = None,
        auth: AuthHandler | httpx.Auth | None = None,
        client: httpx.AsyncClient | None = None,
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
                # AuthHandler now inherits from httpx.Auth directly
                auth_param = auth
            elif auth is not None:
                # Use httpx built-in auth directly
                auth_param = auth

            self._client = httpx.AsyncClient(
                base_url=self.base_url,
                timeout=timeout,
                headers=headers or {},
                auth=auth_param,
                # Enable response streaming
                follow_redirects=True,
            )
            self._owns_client = True

    async def __aenter__(self) -> AsyncStreamingClient:
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        await self.aclose()

    async def aclose(self) -> None:
        """Close the underlying HTTP client if we own it."""
        if self._owns_client:
            await self._client.aclose()

    @asynccontextmanager
    async def stream_request(
        self,
        method: str,
        path: str,
        *,
        params: dict[str, Any] | None = None,
        json_data: dict[str, Any] | None = None,
        json_model: BaseModel | None = None,
        headers: dict[str, str] | None = None,
    ) -> AsyncIterator[StreamingResponse]:
        """Make a streaming HTTP request within an async context manager.

        Args:
            method: HTTP method (GET, POST, etc.)
            path: API endpoint path
            params: Query parameters
            json_data: JSON data to send (mutually exclusive with json_model)
            json_model: Pydantic model to serialize and send
            headers: Additional headers

        Yields:
            StreamingResponse: Streaming response object

        Example:
            ```python
            async with client.stream_request("GET", "/large-file") as response:
                async for chunk in response.aiter_bytes():
                    process(chunk)
            ```
        """
        url = f"{self.base_url}/{path.lstrip('/')}"
        start_time = time.time()

        # Handle request body serialization
        if json_model is not None and json_data is not None:
            raise ValueError("Cannot specify both json_data and json_model")

        request_headers = headers.copy() if headers else {}

        # Serialize Pydantic model
        if json_model is not None:
            # Use Pydantic with improved ReflectapiOption serialization
            model_dict = json_model.model_dump(exclude_none=False)  # Keep explicit None
            
            # Filter out Undefined values (ReflectapiOption serializer returns Undefined sentinel)
            from .option import Undefined
            final_dict = {
                k: v for k, v in model_dict.items() 
                if not (hasattr(v, '__class__') and v is Undefined)
            }
            
            import json
            content = json.dumps(final_dict, separators=(',', ':')).encode('utf-8')
            request_headers["Content-Type"] = "application/json"

            # Build request with raw content
            request = self._client.build_request(
                method=method,
                url=url,
                params=params,
                content=content,
                headers=request_headers,
            )
        else:
            # Build request with JSON data - assume V2 processed
            processed_json_data = json_data

            request = self._client.build_request(
                method=method,
                url=url,
                params=params,
                json=processed_json_data,
                headers=request_headers,
            )

        response = None
        try:
            # Execute through middleware chain
            if self.middleware_chain.middleware:
                response = await self.middleware_chain.execute(request, self._client)
            else:
                response = await self._client.send(request, stream=True)

            metadata = TransportMetadata.from_response(response, start_time)

            # Handle error responses
            if response.status_code >= 400:
                # For streaming responses, we need to read a limited amount to get error details
                error_data = None
                try:
                    # Read up to 1MB for error details
                    error_content = await response.aread()
                    if len(error_content) <= 1024 * 1024:  # 1MB limit
                        error_data = response.json() if error_content else None
                except Exception:
                    pass

                raise ApplicationError.from_response(response, metadata, error_data)

            # Yield the streaming response
            streaming_response = StreamingResponse(response, metadata)
            try:
                yield streaming_response
            finally:
                # Ensure cleanup even if user doesn't call aclose()
                await streaming_response.aclose()

        except httpx.TimeoutException as e:
            if response:
                await response.aclose()
            raise TimeoutError.from_httpx_timeout(e)
        except httpx.RequestError as e:
            if response:
                await response.aclose()
            raise NetworkError.from_httpx_error(e)
        except Exception:
            # Cleanup on any other exception
            if response:
                await response.aclose()
            raise

    @asynccontextmanager
    async def download_file(
        self,
        path: str,
        file_path: str,
        *,
        params: dict[str, Any] | None = None,
        chunk_size: int = 8192,
        headers: dict[str, str] | None = None,
    ) -> AsyncIterator[dict[str, Any]]:
        """Download a file directly to disk with progress tracking.

        Args:
            path: API endpoint path
            file_path: Local file path to save to
            params: Query parameters
            chunk_size: Size of chunks to read/write
            headers: Additional headers

        Yields:
            Dict with download progress information:
            - bytes_written: Number of bytes written so far
            - total_bytes: Total file size if known (from Content-Length)
            - progress: Progress percentage (0.0-1.0) if total size known
            - response: StreamingResponse object

        Example:
            ```python
            async with client.download_file("/files/large.zip", "local.zip") as progress:
                print(f"Downloaded {progress['bytes_written']} bytes")
            ```
        """
        async with self.stream_request("GET", path, params=params, headers=headers) as response:
            total_bytes = response.content_length
            bytes_written = 0

            progress_info = {
                "bytes_written": 0,
                "total_bytes": total_bytes,
                "progress": 0.0 if total_bytes else None,
                "response": response,
            }

            try:
                with open(file_path, 'wb') as f:
                    async for chunk in response.aiter_bytes(chunk_size):
                        f.write(chunk)
                        bytes_written += len(chunk)

                        # Update progress
                        progress_info["bytes_written"] = bytes_written
                        if total_bytes:
                            progress_info["progress"] = min(bytes_written / total_bytes, 1.0)

                yield progress_info

            except Exception:
                # Clean up partial file on error
                import os
                try:
                    os.unlink(file_path)
                except OSError:
                    pass
                raise

    # Convenience class methods for authentication
    @classmethod
    def from_bearer_token(
        cls,
        base_url: str,
        token: str,
        **kwargs: Any,
    ) -> AsyncStreamingClient:
        """Create a streaming client with Bearer token authentication."""
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
    ) -> AsyncStreamingClient:
        """Create a streaming client with API key authentication."""
        from .auth import APIKeyAuth
        return cls(base_url, auth=APIKeyAuth(api_key, header_name, param_name), **kwargs)

    @classmethod
    def from_basic_auth(
        cls,
        base_url: str,
        username: str,
        password: str,
        **kwargs: Any,
    ) -> AsyncStreamingClient:
        """Create a streaming client with HTTP Basic authentication."""
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
    ) -> AsyncStreamingClient:
        """Create a streaming client with OAuth2 client credentials authentication."""
        from .auth import OAuth2ClientCredentialsAuth
        return cls(
            base_url,
            auth=OAuth2ClientCredentialsAuth(token_url, client_id, client_secret, scope),
            **kwargs
        )
