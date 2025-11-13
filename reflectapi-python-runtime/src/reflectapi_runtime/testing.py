"""Testing utilities for ReflectAPI Python clients."""

from __future__ import annotations

import json
from pathlib import Path
from typing import TYPE_CHECKING, Any, TypeVar
from unittest.mock import AsyncMock, MagicMock

import httpx

if TYPE_CHECKING:
    from pydantic import BaseModel

from .middleware import AsyncMiddleware, SyncMiddleware
from .response import ApiResponse, TransportMetadata

T = TypeVar("T")


class CassetteMiddleware(SyncMiddleware):
    """Middleware for recording and replaying HTTP requests (sync version).

    This middleware integrates with CassetteClient to provide VCR-like
    functionality directly through the middleware chain, making recording
    and playback seamless and transparent.
    """

    def __init__(self, cassette_client: CassetteClient):
        self.cassette_client = cassette_client

    def handle(
        self, request: httpx.Request, next_call: SyncNextHandler
    ) -> httpx.Response:
        """Handle request through cassette recording/playback."""
        # For now, just pass through to next handler
        # Full implementation would integrate with cassette_client
        return next_call(request)

    def process_request(
        self, request: httpx.Request, client: httpx.Client
    ) -> httpx.Response:
        """Process request through cassette recording/playback."""
        if self.cassette_client.is_recording:
            # Make the real request and record it
            response = client.send(request)
            self.cassette_client.record_interaction(request, response)
            return response
        else:
            # Try to find a matching recorded response
            recorded_response = self.cassette_client.find_matching_response(request)
            if recorded_response:
                return recorded_response
            else:
                # No matching response found - either make real request or raise error
                if self.cassette_client.allow_new_requests:
                    response = client.send(request)
                    self.cassette_client.record_interaction(request, response)
                    return response
                else:
                    raise ValueError(
                        f"No recorded response found for {request.method} {request.url} "
                        "and new requests are not allowed"
                    )


class AsyncCassetteMiddleware(AsyncMiddleware):
    """Middleware for recording and replaying HTTP requests (async version)."""

    def __init__(self, cassette_client: CassetteClient):
        self.cassette_client = cassette_client

    async def handle(
        self, request: httpx.Request, next_call: AsyncNextHandler
    ) -> httpx.Response:
        """Handle request through cassette recording/playback."""
        # For now, just pass through to next handler
        # Full implementation would integrate with cassette_client
        return await next_call(request)

    async def process_request(
        self, request: httpx.Request, client: httpx.AsyncClient
    ) -> httpx.Response:
        """Process request through cassette recording/playback."""
        if self.cassette_client.is_recording:
            # Make the real request and record it
            response = await client.send(request)
            self.cassette_client.record_interaction(request, response)
            return response
        else:
            # Try to find a matching recorded response
            recorded_response = self.cassette_client.find_matching_response(request)
            if recorded_response:
                return recorded_response
            else:
                # No matching response found
                if self.cassette_client.allow_new_requests:
                    response = await client.send(request)
                    self.cassette_client.record_interaction(request, response)
                    return response
                else:
                    raise ValueError(
                        f"No recorded response found for {request.method} {request.url} "
                        "and new requests are not allowed"
                    )


class MockClient:
    """Mock client for testing that mimics the interface of generated clients."""

    def __init__(self) -> None:
        self._mock_responses: dict[str, Any] = {}
        self._call_history: list[dict[str, Any]] = []

    def __getattr__(self, name: str) -> Any:
        """Return a mock for any method call."""
        if name not in self._mock_responses:
            self._mock_responses[name] = MagicMock()
        return self._mock_responses[name]

    def set_response(self, method_name: str, response: Any) -> None:
        """Set a mock response for a specific method."""
        mock = MagicMock(return_value=response)
        self._mock_responses[method_name] = mock
        setattr(self, method_name, mock)

    def set_async_response(self, method_name: str, response: Any) -> None:
        """Set a mock async response for a specific method."""
        mock = AsyncMock(return_value=response)
        self._mock_responses[method_name] = mock
        setattr(self, method_name, mock)

    def get_call_history(self) -> list[dict[str, Any]]:
        """Get the history of all method calls."""
        return self._call_history.copy()


def create_api_response(
    value: T,
    *,
    status_code: int = 200,
    headers: dict[str, str] | None = None,
    timing: float = 0.1,
) -> ApiResponse[T]:
    """Create a mock ApiResponse for testing."""
    import httpx

    # Create a mock response
    mock_response = MagicMock(spec=httpx.Response)
    mock_response.status_code = status_code
    mock_response.headers = httpx.Headers(headers or {})
    mock_response.reason_phrase = "OK" if status_code < 400 else "Error"

    metadata = TransportMetadata(
        status_code=status_code,
        headers=mock_response.headers,
        timing=timing,
        raw_response=mock_response,
    )

    return ApiResponse(value, metadata)


class CassetteClient:
    """Client that records and replays HTTP requests for testing."""

    def __init__(
        self, cassette_path: str | Path, allow_new_requests: bool = True
    ) -> None:
        self.cassette_path = Path(cassette_path)
        self._recorded_interactions: list[dict[str, Any]] = []
        self._playback_interactions: list[dict[str, Any]] = []
        self._current_interaction = 0
        self._mode = "record"  # or "playback"
        self.allow_new_requests = allow_new_requests

    @classmethod
    def record(cls, cassette_path: str | Path) -> CassetteClient:
        """Create a client in record mode."""
        client = cls(cassette_path)
        client._mode = "record"
        return client

    @classmethod
    def playback(cls, cassette_path: str | Path) -> CassetteClient:
        """Create a client in playback mode."""
        client = cls(cassette_path)
        client._mode = "playback"
        client._load_cassette()
        return client

    def _load_cassette(self) -> None:
        """Load recorded interactions from the cassette file."""
        if self.cassette_path.exists():
            with open(self.cassette_path) as f:
                data = json.load(f)
                self._playback_interactions = data.get("interactions", [])

    def save_cassette(self) -> None:
        """Save recorded interactions to the cassette file."""
        self.cassette_path.parent.mkdir(parents=True, exist_ok=True)

        with open(self.cassette_path, "w") as f:
            json.dump(
                {
                    "interactions": self._recorded_interactions,
                    "version": "1.0",
                },
                f,
                indent=2,
            )

    def get_next_response(self, request: dict[str, Any]) -> Any:  # noqa: ARG002
        """Get the next recorded response for playback."""
        if self._mode != "playback":
            return None

        if self._current_interaction >= len(self._playback_interactions):
            raise RuntimeError("No more recorded interactions available")

        interaction = self._playback_interactions[self._current_interaction]
        self._current_interaction += 1

        # TODO: Match request against recorded request for validation
        return interaction["response"]

    @property
    def is_recording(self) -> bool:
        """Check if the client is in recording mode."""
        return self._mode == "record"

    @property
    def is_playback(self) -> bool:
        """Check if the client is in playback mode."""
        return self._mode == "playback"

    def record_interaction(
        self, request: httpx.Request, response: httpx.Response
    ) -> None:
        """Record an HTTP request/response interaction."""
        if not self.is_recording:
            return

        # Serialize request
        request_data = {
            "method": request.method,
            "url": str(request.url),
            "headers": dict(request.headers),
            "content": request.content.decode("utf-8", errors="replace")
            if request.content
            else None,
        }

        # Serialize response
        response_data = {
            "status_code": response.status_code,
            "headers": dict(response.headers),
            "content": response.content.decode("utf-8", errors="replace"),
            "reason_phrase": getattr(response, "reason_phrase", ""),
        }

        self._recorded_interactions.append(
            {
                "request": request_data,
                "response": response_data,
            }
        )

    def find_matching_response(self, request: httpx.Request) -> httpx.Response | None:
        """Find a recorded response that matches the given request."""
        if not self.is_playback:
            return None

        request_method = request.method
        request_url = str(request.url)

        # Simple matching by method and URL
        # In a more sophisticated implementation, you might want to match headers, body, etc.
        for interaction in self._playback_interactions:
            recorded_request = interaction["request"]
            if (
                recorded_request["method"] == request_method
                and recorded_request["url"] == request_url
            ):
                # Create a mock response
                response_data = interaction["response"]

                # Create a mock httpx.Response
                mock_response = MagicMock(spec=httpx.Response)
                mock_response.status_code = response_data["status_code"]
                mock_response.headers = httpx.Headers(response_data["headers"])
                mock_response.content = response_data["content"].encode("utf-8")
                mock_response.text = response_data["content"]
                mock_response.reason_phrase = response_data.get("reason_phrase", "")
                mock_response.json.return_value = (
                    json.loads(response_data["content"])
                    if response_data["content"]
                    else {}
                )

                return mock_response

        return None


class TestClientMixin:
    """Mixin that adds testing capabilities to generated clients."""

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        # Extract testing-specific kwargs
        self._dev_mode = kwargs.pop("dev_mode", False)
        self._cassette_client: CassetteClient | None = kwargs.pop(
            "cassette_client", None
        )

        # Only call super().__init__ if there are args/kwargs to pass
        # or if the class has a custom __init__ that's not object.__init__
        try:
            super().__init__(*args, **kwargs)
        except TypeError:
            # If TypeError occurs, it's likely object.__init__ with extra args
            # Only call it if there are no args/kwargs
            if not args and not kwargs:
                super().__init__()
            # Otherwise, just skip the super() call

    def save_requests_to_cassette(self, cassette_path: str | Path) -> None:  # noqa: ARG002
        """Save recorded requests to a cassette file."""
        if self._cassette_client:
            self._cassette_client.save_cassette()

    @classmethod
    def playback_from_cassette(
        cls,
        cassette_path: str | Path,
        **kwargs: Any,
    ) -> Any:
        """Create a client that replays requests from a cassette."""
        cassette_client = CassetteClient.playback(cassette_path)
        kwargs["cassette_client"] = cassette_client
        return cls(base_url="http://test.local", **kwargs)

    @classmethod
    def record_to_cassette(
        cls,
        cassette_path: str | Path,
        base_url: str,
        **kwargs: Any,
    ) -> Any:
        """Create a client that records requests to a cassette."""
        cassette_client = CassetteClient.record(cassette_path)

        # Add cassette middleware to the middleware list
        middleware = kwargs.get("middleware", [])

        # Determine if this is an async client
        if hasattr(cls, "__bases__") and any(
            "Async" in base.__name__ for base in cls.__bases__
        ):
            cassette_middleware = AsyncCassetteMiddleware(cassette_client)
        else:
            cassette_middleware = CassetteMiddleware(cassette_client)

        middleware.insert(0, cassette_middleware)  # Add at the beginning of the chain
        kwargs["middleware"] = middleware
        kwargs["cassette_client"] = cassette_client

        return cls(base_url=base_url, **kwargs)


# Hypothesis strategies for property-based testing
try:
    from hypothesis import strategies as st  # type: ignore[import-not-found]
    from hypothesis.strategies import (  # type: ignore[import-not-found]
        SearchStrategy,  # noqa: TC002
    )

    def create_model_strategy(
        model_class: type[BaseModel],
    ) -> SearchStrategy[BaseModel]:
        """Create a Hypothesis strategy for a Pydantic model."""
        # This is a simplified implementation
        # A full implementation would introspect the model fields
        # and create appropriate strategies for each field type

        def build_model(**kwargs: Any) -> BaseModel:
            return model_class.model_validate(kwargs)

        # Return a basic strategy that creates valid instances
        return st.builds(build_model)

except ImportError:
    # Hypothesis is not available
    def create_model_strategy(model_class: type[BaseModel]) -> None:  # type: ignore  # noqa: ARG001
        """Hypothesis not available - strategy creation disabled."""
        raise ImportError(
            "hypothesis is required for property-based testing strategies"
        )
