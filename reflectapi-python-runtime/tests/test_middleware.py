"""Tests for the transport-agnostic middleware system."""

from __future__ import annotations

import logging
from unittest.mock import patch

import httpx
import pytest

from reflectapi_runtime import Request, Response
from reflectapi_runtime.middleware import (
    AsyncLoggingMiddleware,
    AsyncMiddleware,
    AsyncMiddlewareChain,
    RetryMiddleware,
    SyncLoggingMiddleware,
    SyncMiddleware,
    SyncMiddlewareChain,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_request(path: str = "/test", body: bytes = b"") -> Request:
    return Request(
        path=path, headers={"Authorization": "Bearer token"}, body=body
    )


def make_response(status: int = 200, body: bytes = b'{"ok": true}') -> Response:
    return Response(
        status=status,
        headers={"Content-Type": "application/json"},
        body=body,
    )


# ---------------------------------------------------------------------------
# Abstract base classes
# ---------------------------------------------------------------------------


class TestAsyncMiddleware:
    def test_is_abstract(self):
        with pytest.raises(TypeError):
            AsyncMiddleware()  # type: ignore[abstract]


class TestSyncMiddleware:
    def test_is_abstract(self):
        with pytest.raises(TypeError):
            SyncMiddleware()  # type: ignore[abstract]


# ---------------------------------------------------------------------------
# Logging middleware
# ---------------------------------------------------------------------------


class TestAsyncLoggingMiddleware:
    def test_initialization(self):
        middleware = AsyncLoggingMiddleware()
        assert middleware.logger.name == "reflectapi.client"

        middleware = AsyncLoggingMiddleware("custom.logger")
        assert middleware.logger.name == "custom.logger"

    @pytest.mark.asyncio
    async def test_handle_logs_request_and_response(self, caplog):
        middleware = AsyncLoggingMiddleware()
        request = make_request()
        response = make_response()

        async def next_handler(req: Request) -> Response:
            return response

        with caplog.at_level(logging.DEBUG):
            result = await middleware.handle(request, next_handler)

        assert result is response
        messages = [record.message for record in caplog.records]
        assert any("Making request" in m for m in messages)
        assert any("Received response" in m for m in messages)


class TestSyncLoggingMiddleware:
    def test_initialization(self):
        middleware = SyncLoggingMiddleware()
        assert middleware.logger.name == "reflectapi.client"

    def test_handle_logs_request_and_response(self, caplog):
        middleware = SyncLoggingMiddleware()
        request = make_request()
        response = make_response()

        def next_handler(req: Request) -> Response:
            return response

        with caplog.at_level(logging.DEBUG):
            result = middleware.handle(request, next_handler)

        assert result is response
        messages = [record.message for record in caplog.records]
        assert any("Making request" in m for m in messages)
        assert any("Received response" in m for m in messages)


# ---------------------------------------------------------------------------
# Retry middleware
# ---------------------------------------------------------------------------


class TestRetryMiddleware:
    def test_initialization_defaults(self):
        middleware = RetryMiddleware()
        assert middleware.max_retries == 3
        assert middleware.retry_status_codes == {429, 502, 503, 504}
        assert middleware.backoff_factor == 0.5
        assert middleware.retry_on == (httpx.RequestError,)
        assert middleware.retry_non_idempotent is False

    def test_initialization_custom(self):
        middleware = RetryMiddleware(
            max_retries=5,
            retry_status_codes={400, 500},
            backoff_factor=2.0,
            retry_on=(IOError,),
            retry_non_idempotent=True,
        )
        assert middleware.max_retries == 5
        assert middleware.retry_status_codes == {400, 500}
        assert middleware.backoff_factor == 2.0
        assert middleware.retry_on == (IOError,)
        assert middleware.retry_non_idempotent is True

    @pytest.mark.asyncio
    async def test_handle_successful_response(self):
        middleware = RetryMiddleware()
        request = make_request()
        response = make_response(status=200)

        calls = 0

        async def next_handler(req):
            nonlocal calls
            calls += 1
            return response

        result = await middleware.handle(request, next_handler)
        assert result is response
        assert calls == 1

    @pytest.mark.asyncio
    async def test_handle_non_retryable_status_returned_immediately(self):
        middleware = RetryMiddleware()
        request = make_request()
        response = make_response(status=400)

        calls = 0

        async def next_handler(req):
            nonlocal calls
            calls += 1
            return response

        result = await middleware.handle(request, next_handler)
        assert result is response
        assert calls == 1

    @pytest.mark.asyncio
    async def test_handle_retryable_status_eventually_succeeds(self):
        middleware = RetryMiddleware(max_retries=3, backoff_factor=0)
        request = make_request()
        responses = [
            make_response(status=503),
            make_response(status=503),
            make_response(status=200),
        ]
        calls = iter(responses)

        async def next_handler(req):
            return next(calls)

        with patch("asyncio.sleep") as sleep:
            result = await middleware.handle(request, next_handler)

        assert result.status == 200
        assert sleep.call_count == 2

    @pytest.mark.asyncio
    async def test_handle_retryable_status_exhausts_retries(self):
        middleware = RetryMiddleware(max_retries=2, backoff_factor=0)
        request = make_request()
        response = make_response(status=503)

        calls = 0

        async def next_handler(req):
            nonlocal calls
            calls += 1
            return response

        with patch("asyncio.sleep"):
            result = await middleware.handle(request, next_handler)

        assert result.status == 503
        assert calls == 3  # initial + 2 retries

    @pytest.mark.asyncio
    async def test_default_post_does_not_retry_network_error(self):
        # reflectapi is always POST; default behaviour does not auto-retry
        # network errors because POST is not idempotent in general.
        middleware = RetryMiddleware(max_retries=3, backoff_factor=0)
        request = make_request()

        calls = 0

        async def next_handler(req):
            nonlocal calls
            calls += 1
            raise httpx.ConnectError("boom")

        with pytest.raises(httpx.ConnectError):
            await middleware.handle(request, next_handler)
        assert calls == 1

    @pytest.mark.asyncio
    async def test_retry_non_idempotent_opts_in(self):
        middleware = RetryMiddleware(
            max_retries=2, backoff_factor=0, retry_non_idempotent=True
        )
        request = make_request()
        success = make_response(status=200)

        calls = 0

        async def next_handler(req):
            nonlocal calls
            calls += 1
            if calls < 2:
                raise httpx.ConnectError("boom")
            return success

        with patch("asyncio.sleep"):
            result = await middleware.handle(request, next_handler)

        assert result is success
        assert calls == 2

    @pytest.mark.asyncio
    async def test_custom_retry_on_exception(self):
        """Users can configure non-httpx exception types via retry_on."""

        class _CustomTransportError(Exception):
            pass

        middleware = RetryMiddleware(
            max_retries=2,
            backoff_factor=0,
            retry_on=(_CustomTransportError,),
            retry_non_idempotent=True,
        )
        request = make_request()
        success = make_response(status=200)

        calls = 0

        async def next_handler(req):
            nonlocal calls
            calls += 1
            if calls < 2:
                raise _CustomTransportError("boom")
            return success

        with patch("asyncio.sleep"):
            result = await middleware.handle(request, next_handler)

        assert result is success
        assert calls == 2

    @pytest.mark.asyncio
    async def test_429_retried_by_default(self):
        middleware = RetryMiddleware(max_retries=2, backoff_factor=0)
        request = make_request()
        responses = [make_response(status=429), make_response(status=200)]
        calls = iter(responses)

        async def next_handler(req):
            return next(calls)

        with patch("asyncio.sleep"):
            result = await middleware.handle(request, next_handler)
        assert result.status == 200

    @pytest.mark.asyncio
    async def test_custom_retry_status_codes(self):
        middleware = RetryMiddleware(
            max_retries=2, retry_status_codes={418}, backoff_factor=0
        )
        request = make_request()
        response = make_response(status=503)

        calls = 0

        async def next_handler(req):
            nonlocal calls
            calls += 1
            return response

        result = await middleware.handle(request, next_handler)
        assert result.status == 503
        assert calls == 1

    @pytest.mark.asyncio
    async def test_exponential_backoff_is_capped(self):
        middleware = RetryMiddleware(max_retries=4, backoff_factor=100.0)
        request = make_request()
        response = make_response(status=503)

        async def next_handler(req):
            return response

        sleeps: list[float] = []

        async def fake_sleep(seconds):
            sleeps.append(seconds)

        with patch("asyncio.sleep", side_effect=fake_sleep):
            await middleware.handle(request, next_handler)

        assert sleeps, "expected at least one backoff sleep"
        assert all(s <= 30.0 for s in sleeps)


# ---------------------------------------------------------------------------
# Middleware chains — terminal handler is now an arbitrary callable.
# ---------------------------------------------------------------------------


class _RecordingAsyncMiddleware(AsyncMiddleware):
    def __init__(self, name: str, log: list[str]) -> None:
        self.name = name
        self.log = log

    async def handle(self, request, next_call):
        self.log.append(f"{self.name}:before")
        response = await next_call(request)
        self.log.append(f"{self.name}:after")
        return response


class _RecordingSyncMiddleware(SyncMiddleware):
    def __init__(self, name: str, log: list[str]) -> None:
        self.name = name
        self.log = log

    def handle(self, request, next_call):
        self.log.append(f"{self.name}:before")
        response = next_call(request)
        self.log.append(f"{self.name}:after")
        return response


class TestAsyncMiddlewareChain:
    def test_initialization(self):
        chain = AsyncMiddlewareChain([])
        assert chain.middleware == []

    @pytest.mark.asyncio
    async def test_execute_empty_chain_calls_terminal(self):
        chain = AsyncMiddlewareChain([])
        request = make_request()
        response = make_response()
        terminal_calls = 0

        async def terminal(req: Request) -> Response:
            nonlocal terminal_calls
            terminal_calls += 1
            assert req is request
            return response

        result = await chain.execute(request, terminal)
        assert result is response
        assert terminal_calls == 1

    @pytest.mark.asyncio
    async def test_execute_runs_middleware_around_terminal(self):
        log: list[str] = []
        chain = AsyncMiddlewareChain([_RecordingAsyncMiddleware("m1", log)])
        request = make_request()
        response = make_response()

        async def terminal(req):
            log.append("terminal")
            return response

        result = await chain.execute(request, terminal)
        assert result is response
        assert log == ["m1:before", "terminal", "m1:after"]

    @pytest.mark.asyncio
    async def test_execute_preserves_middleware_ordering(self):
        log: list[str] = []
        chain = AsyncMiddlewareChain(
            [
                _RecordingAsyncMiddleware("outer", log),
                _RecordingAsyncMiddleware("inner", log),
            ]
        )
        request = make_request()
        response = make_response()

        async def terminal(req):
            log.append("terminal")
            return response

        await chain.execute(request, terminal)
        assert log == [
            "outer:before",
            "inner:before",
            "terminal",
            "inner:after",
            "outer:after",
        ]


class TestSyncMiddlewareChain:
    def test_initialization(self):
        chain = SyncMiddlewareChain([])
        assert chain.middleware == []

    def test_execute_empty_chain_calls_terminal(self):
        chain = SyncMiddlewareChain([])
        request = make_request()
        response = make_response()
        terminal_calls = 0

        def terminal(req):
            nonlocal terminal_calls
            terminal_calls += 1
            return response

        result = chain.execute(request, terminal)
        assert result is response
        assert terminal_calls == 1

    def test_execute_runs_middleware_around_terminal(self):
        log: list[str] = []
        chain = SyncMiddlewareChain([_RecordingSyncMiddleware("m1", log)])
        request = make_request()
        response = make_response()

        def terminal(req):
            log.append("terminal")
            return response

        result = chain.execute(request, terminal)
        assert result is response
        assert log == ["m1:before", "terminal", "m1:after"]

    def test_execute_preserves_middleware_ordering(self):
        log: list[str] = []
        chain = SyncMiddlewareChain(
            [
                _RecordingSyncMiddleware("outer", log),
                _RecordingSyncMiddleware("inner", log),
            ]
        )
        request = make_request()
        response = make_response()

        def terminal(req):
            log.append("terminal")
            return response

        chain.execute(request, terminal)
        assert log == [
            "outer:before",
            "inner:before",
            "terminal",
            "inner:after",
            "outer:after",
        ]
