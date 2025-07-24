"""Tests for middleware system."""

import asyncio
import logging
import time
from unittest.mock import AsyncMock, Mock, patch

import httpx
import pytest

from reflectapi_runtime.middleware import (
    AsyncLoggingMiddleware,
    AsyncMiddleware,
    AsyncMiddlewareChain,
    RetryMiddleware,
    SyncLoggingMiddleware,
    SyncMiddleware,
    SyncMiddlewareChain,
)


class TestAsyncMiddleware:
    def test_is_abstract(self):
        with pytest.raises(TypeError):
            AsyncMiddleware()


class TestAsyncLoggingMiddleware:
    def test_initialization(self):
        middleware = AsyncLoggingMiddleware()
        assert middleware.logger.name == "reflectapi.client"

        middleware = AsyncLoggingMiddleware("custom.logger")
        assert middleware.logger.name == "custom.logger"

    @pytest.mark.asyncio
    async def test_handle_logs_request_and_response(self, caplog):
        middleware = AsyncLoggingMiddleware()

        mock_request = Mock(spec=httpx.Request)
        mock_request.method = "GET"
        mock_request.url = "http://example.com/test"
        mock_request.headers = {"Authorization": "Bearer token"}

        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200
        mock_response.headers = {"Content-Type": "application/json"}

        async def next_handler(request):
            return mock_response

        with caplog.at_level(logging.DEBUG):
            result = await middleware.handle(mock_request, next_handler)

        assert result is mock_response

        log_messages = [record.message for record in caplog.records]
        assert any("Making request" in msg for msg in log_messages)
        assert any("Received response" in msg for msg in log_messages)


class TestRetryMiddleware:
    def test_initialization_defaults(self):
        middleware = RetryMiddleware()

        assert middleware.max_retries == 3
        assert middleware.retry_status_codes == {429, 502, 503, 504}
        assert middleware.backoff_factor == 0.5
        assert frozenset({"GET", "HEAD", "OPTIONS", "PUT", "DELETE", "TRACE"}) == middleware.IDEMPOTENT_METHODS

    def test_initialization_custom(self):
        middleware = RetryMiddleware(
            max_retries=5, retry_status_codes={400, 500}, backoff_factor=2.0
        )

        assert middleware.max_retries == 5
        assert middleware.retry_status_codes == {400, 500}
        assert middleware.backoff_factor == 2.0

    @pytest.mark.asyncio
    async def test_handle_successful_response(self):
        middleware = RetryMiddleware()

        mock_request = Mock(spec=httpx.Request)
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200

        call_count = 0

        async def next_handler(request):
            nonlocal call_count
            call_count += 1
            return mock_response

        result = await middleware.handle(mock_request, next_handler)

        assert result is mock_response
        assert call_count == 1

    @pytest.mark.asyncio
    async def test_handle_non_retryable_error(self):
        middleware = RetryMiddleware()

        mock_request = Mock(spec=httpx.Request)
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 404

        call_count = 0

        async def next_handler(request):
            nonlocal call_count
            call_count += 1
            return mock_response

        result = await middleware.handle(mock_request, next_handler)

        assert result is mock_response
        assert call_count == 1

    @pytest.mark.asyncio
    async def test_handle_retryable_error_eventually_succeeds(self):
        middleware = RetryMiddleware(max_retries=2, backoff_factor=0.01)

        mock_request = Mock(spec=httpx.Request)
        mock_request.url = "http://example.com/test"

        call_count = 0

        async def next_handler(request):
            nonlocal call_count
            call_count += 1

            if call_count <= 2:
                mock_response = Mock(spec=httpx.Response)
                mock_response.status_code = 503  # Use default retry status code
                return mock_response
            else:
                mock_response = Mock(spec=httpx.Response)
                mock_response.status_code = 200
                return mock_response

        result = await middleware.handle(mock_request, next_handler)

        assert result.status_code == 200
        assert call_count == 3

    @pytest.mark.asyncio
    async def test_handle_retryable_error_exhausts_retries(self):
        middleware = RetryMiddleware(max_retries=2, backoff_factor=0.01)

        mock_request = Mock(spec=httpx.Request)
        mock_request.url = "http://example.com/test"
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 503  # Use default retry status code

        call_count = 0

        async def next_handler(request):
            nonlocal call_count
            call_count += 1
            return mock_response

        result = await middleware.handle(mock_request, next_handler)

        assert result.status_code == 503
        assert call_count == 3

    @pytest.mark.asyncio
    async def test_handle_network_error_retries(self):
        middleware = RetryMiddleware(max_retries=2, backoff_factor=0.01)

        mock_request = Mock(spec=httpx.Request)
        mock_request.method = "GET"  # Idempotent method
        mock_request.url = "http://example.com/test"

        call_count = 0

        async def next_handler(request):
            nonlocal call_count
            call_count += 1

            if call_count <= 2:
                raise httpx.ConnectError("Connection failed")
            else:
                mock_response = Mock(spec=httpx.Response)
                mock_response.status_code = 200
                return mock_response

        result = await middleware.handle(mock_request, next_handler)

        assert result.status_code == 200
        assert call_count == 3

    @pytest.mark.asyncio
    async def test_handle_network_error_exhausts_retries(self):
        middleware = RetryMiddleware(max_retries=1, backoff_factor=0.01)

        mock_request = Mock(spec=httpx.Request)
        mock_request.method = "GET"  # Idempotent method
        mock_request.url = "http://example.com/test"

        async def next_handler(request):
            raise httpx.ConnectError("Connection failed")

        with pytest.raises(httpx.ConnectError):
            await middleware.handle(mock_request, next_handler)

    @pytest.mark.asyncio
    async def test_idempotency_check_non_idempotent_methods(self):
        """Test that non-idempotent methods are not retried on network errors."""
        middleware = RetryMiddleware(max_retries=3, backoff_factor=0.01)

        non_idempotent_methods = ["POST", "PATCH"]

        for method in non_idempotent_methods:
            mock_request = Mock(spec=httpx.Request)
            mock_request.method = method
            mock_request.url = "http://example.com/test"

            call_count = 0

            async def next_handler(request):
                nonlocal call_count
                call_count += 1
                raise httpx.ConnectError("Connection failed")

            with pytest.raises(httpx.ConnectError):
                await middleware.handle(mock_request, next_handler)

            # Should not retry non-idempotent methods on network errors
            assert call_count == 1
            call_count = 0  # Reset for next method

    @pytest.mark.asyncio
    async def test_idempotency_check_idempotent_methods(self):
        """Test that idempotent methods are retried on network errors."""
        middleware = RetryMiddleware(max_retries=2, backoff_factor=0.01)

        idempotent_methods = ["GET", "HEAD", "OPTIONS", "PUT", "DELETE", "TRACE"]

        for method in idempotent_methods:
            mock_request = Mock(spec=httpx.Request)
            mock_request.method = method
            mock_request.url = "http://example.com/test"

            call_count = 0

            async def next_handler(request):
                nonlocal call_count
                call_count += 1
                if call_count <= 2:
                    raise httpx.ConnectError("Connection failed")
                else:
                    mock_response = Mock(spec=httpx.Response)
                    mock_response.status_code = 200
                    return mock_response

            result = await middleware.handle(mock_request, next_handler)

            # Should retry idempotent methods
            assert result.status_code == 200
            assert call_count == 3
            call_count = 0  # Reset for next method

    @pytest.mark.asyncio
    async def test_rate_limit_retry_429(self):
        """Test that 429 (rate limit) responses are retried."""
        middleware = RetryMiddleware(max_retries=2, backoff_factor=0.01)

        mock_request = Mock(spec=httpx.Request)
        mock_request.method = "POST"  # Non-idempotent, but should retry on 429
        mock_request.url = "http://example.com/test"

        call_count = 0

        async def next_handler(request):
            nonlocal call_count
            call_count += 1
            if call_count <= 2:
                mock_response = Mock(spec=httpx.Response)
                mock_response.status_code = 429
                return mock_response
            else:
                mock_response = Mock(spec=httpx.Response)
                mock_response.status_code = 200
                return mock_response

        result = await middleware.handle(mock_request, next_handler)

        assert result.status_code == 200
        assert call_count == 3

    @pytest.mark.asyncio
    async def test_exponential_backoff_with_jitter(self):
        """Test that backoff timing follows exponential pattern with jitter."""
        middleware = RetryMiddleware(max_retries=3, backoff_factor=1.0)

        mock_request = Mock(spec=httpx.Request)
        mock_request.url = "http://example.com/test"
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 503

        sleep_times = []
        original_sleep = asyncio.sleep

        async def mock_sleep(duration):
            sleep_times.append(duration)
            # Use a very short sleep for testing
            await original_sleep(0.001)

        async def next_handler(request):
            return mock_response

        with patch('asyncio.sleep', side_effect=mock_sleep):
            result = await middleware.handle(mock_request, next_handler)

        # Should have 3 sleep calls (for 3 retry attempts)
        assert len(sleep_times) == 3

        # Verify exponential backoff pattern (with jitter, so approximate)
        # With backoff_factor=1.0: base delays are 1.0, 2.0, 4.0
        # AWS-style jitter: temp/2 + random(0, temp/2) where temp = min(base, 30)
        # So ranges are: 0.5-1.0, 1.0-2.0, 2.0-4.0
        assert 0.5 <= sleep_times[0] <= 1.0
        assert 1.0 <= sleep_times[1] <= 2.0
        assert 2.0 <= sleep_times[2] <= 4.0

        assert result.status_code == 503

    @pytest.mark.asyncio
    async def test_backoff_cap_at_30_seconds(self):
        """Test that backoff is capped at 30 seconds."""
        middleware = RetryMiddleware(max_retries=10, backoff_factor=2.0)

        mock_request = Mock(spec=httpx.Request)
        mock_request.url = "http://example.com/test"
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 503

        sleep_times = []
        original_sleep = asyncio.sleep

        async def mock_sleep(duration):
            sleep_times.append(duration)
            await original_sleep(0.001)

        async def next_handler(request):
            return mock_response

        with patch('asyncio.sleep', side_effect=mock_sleep):
            await middleware.handle(mock_request, next_handler)

        # All sleep times should be <= 30 seconds (with jitter, max is 30)
        for sleep_time in sleep_times:
            assert sleep_time <= 30.0

        # Later attempts should be capped
        assert max(sleep_times[-3:]) <= 30.0

    @pytest.mark.asyncio
    async def test_retry_logging(self, caplog):
        """Test that retry attempts are logged with proper details."""
        middleware = RetryMiddleware(max_retries=2, backoff_factor=0.01)

        mock_request = Mock(spec=httpx.Request)
        mock_request.url = "http://example.com/test"
        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 503

        async def next_handler(request):
            return mock_response

        with caplog.at_level(logging.DEBUG):
            await middleware.handle(mock_request, next_handler)

        # Should log retry attempts
        log_messages = [record.message for record in caplog.records]
        retry_logs = [msg for msg in log_messages if "Retrying request" in msg]

        assert len(retry_logs) == 2  # 2 retry attempts
        assert "http://example.com/test" in retry_logs[0]
        assert "attempt 1/2" in retry_logs[0]
        assert "attempt 2/2" in retry_logs[1]

    @pytest.mark.asyncio
    async def test_custom_retry_status_codes(self):
        """Test retry behavior with custom status codes."""
        middleware = RetryMiddleware(
            max_retries=2,
            retry_status_codes={418, 420},  # Custom status codes
            backoff_factor=0.01
        )

        mock_request = Mock(spec=httpx.Request)
        mock_request.url = "http://example.com/test"

        # Test that custom status code is retried
        call_count = 0
        async def next_handler_custom(request):
            nonlocal call_count
            call_count += 1
            if call_count <= 2:
                mock_response = Mock(spec=httpx.Response)
                mock_response.status_code = 418  # Custom retry code
                return mock_response
            else:
                mock_response = Mock(spec=httpx.Response)
                mock_response.status_code = 200
                return mock_response

        result = await middleware.handle(mock_request, next_handler_custom)
        assert result.status_code == 200
        assert call_count == 3

        # Test that default retry codes are not retried
        call_count = 0
        async def next_handler_default(request):
            nonlocal call_count
            call_count += 1
            mock_response = Mock(spec=httpx.Response)
            mock_response.status_code = 503  # Default retry code, but not in custom set
            return mock_response

        result = await middleware.handle(mock_request, next_handler_default)
        assert result.status_code == 503
        assert call_count == 1  # No retries


class TestAsyncMiddlewareChain:
    def test_initialization(self):
        middleware1 = AsyncLoggingMiddleware()
        middleware2 = RetryMiddleware()

        chain = AsyncMiddlewareChain([middleware1, middleware2])

        assert len(chain.middleware) == 2
        assert chain.middleware[0] is middleware1
        assert chain.middleware[1] is middleware2

    @pytest.mark.asyncio
    async def test_execute_empty_chain(self):
        chain = AsyncMiddlewareChain([])

        mock_request = Mock(spec=httpx.Request)
        mock_response = Mock(spec=httpx.Response)

        mock_transport = AsyncMock()
        mock_transport.send.return_value = mock_response

        result = await chain.execute(mock_request, mock_transport)

        assert result is mock_response
        mock_transport.send.assert_called_once_with(mock_request)

    @pytest.mark.asyncio
    async def test_execute_single_middleware(self):
        middleware = AsyncLoggingMiddleware()
        chain = AsyncMiddlewareChain([middleware])

        mock_request = Mock(spec=httpx.Request)
        mock_request.method = "GET"
        mock_request.url = "http://example.com"
        mock_request.headers = {}

        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200
        mock_response.headers = {}

        mock_transport = AsyncMock()
        mock_transport.send.return_value = mock_response

        result = await chain.execute(mock_request, mock_transport)

        assert result is mock_response
        mock_transport.send.assert_called_once_with(mock_request)

    @pytest.mark.asyncio
    async def test_execute_multiple_middleware(self):
        """Test middleware chain execution order with response modification."""

        class TestMiddleware1(AsyncMiddleware):
            async def handle(self, request, next_call):
                response = await next_call(request)
                response.test_attr1 = "middleware1"
                return response

        class TestMiddleware2(AsyncMiddleware):
            async def handle(self, request, next_call):
                response = await next_call(request)
                response.test_attr2 = "middleware2"
                return response

        middleware1 = TestMiddleware1()
        middleware2 = TestMiddleware2()
        chain = AsyncMiddlewareChain([middleware1, middleware2])

        mock_request = Mock(spec=httpx.Request)
        mock_response = Mock(spec=httpx.Response)

        mock_transport = AsyncMock()
        mock_transport.send.return_value = mock_response

        result = await chain.execute(mock_request, mock_transport)

        assert result is mock_response
        assert hasattr(result, "test_attr1")
        assert hasattr(result, "test_attr2")
        assert result.test_attr1 == "middleware1"
        assert result.test_attr2 == "middleware2"

    @pytest.mark.asyncio
    async def test_execute_with_async_client(self):
        chain = AsyncMiddlewareChain([])

        mock_request = Mock(spec=httpx.Request)
        mock_response = Mock(spec=httpx.Response)

        mock_client = AsyncMock(spec=httpx.AsyncClient)
        mock_client.send.return_value = mock_response

        result = await chain.execute(mock_request, mock_client)

        assert result is mock_response
        mock_client.send.assert_called_once_with(mock_request)


class TestSyncMiddleware:
    def test_is_abstract(self):
        with pytest.raises(TypeError):
            SyncMiddleware()


class TestSyncLoggingMiddleware:
    def test_initialization(self):
        middleware = SyncLoggingMiddleware()
        assert middleware.logger.name == "reflectapi.client"

        middleware = SyncLoggingMiddleware("custom.logger")
        assert middleware.logger.name == "custom.logger"

    def test_handle_logs_request_and_response(self, caplog):
        middleware = SyncLoggingMiddleware()

        mock_request = Mock(spec=httpx.Request)
        mock_request.method = "GET"
        mock_request.url = "http://example.com/test"
        mock_request.headers = {"Authorization": "Bearer token"}

        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200
        mock_response.headers = {"Content-Type": "application/json"}

        def next_handler(request):
            return mock_response

        with caplog.at_level(logging.DEBUG):
            result = middleware.handle(mock_request, next_handler)

        assert result is mock_response

        log_messages = [record.message for record in caplog.records]
        assert any("Making request" in msg for msg in log_messages)
        assert any("Received response" in msg for msg in log_messages)


class TestSyncMiddlewareChain:
    def test_initialization(self):
        middleware1 = SyncLoggingMiddleware()
        middleware2 = SyncLoggingMiddleware("custom")

        chain = SyncMiddlewareChain([middleware1, middleware2])

        assert len(chain.middleware) == 2
        assert chain.middleware[0] is middleware1
        assert chain.middleware[1] is middleware2

    def test_execute_empty_chain(self):
        chain = SyncMiddlewareChain([])

        mock_request = Mock(spec=httpx.Request)
        mock_response = Mock(spec=httpx.Response)

        mock_transport = Mock(spec=httpx.Client)
        mock_transport.send.return_value = mock_response

        result = chain.execute(mock_request, mock_transport)

        assert result is mock_response
        mock_transport.send.assert_called_once_with(mock_request)

    def test_execute_single_middleware(self):
        middleware = SyncLoggingMiddleware()
        chain = SyncMiddlewareChain([middleware])

        mock_request = Mock(spec=httpx.Request)
        mock_request.method = "GET"
        mock_request.url = "http://example.com"
        mock_request.headers = {}

        mock_response = Mock(spec=httpx.Response)
        mock_response.status_code = 200
        mock_response.headers = {}

        mock_transport = Mock(spec=httpx.Client)
        mock_transport.send.return_value = mock_response

        result = chain.execute(mock_request, mock_transport)

        assert result is mock_response
        mock_transport.send.assert_called_once_with(mock_request)

    def test_execute_multiple_middleware(self):
        class TestSyncMiddleware1(SyncMiddleware):
            def handle(self, request, next_call):
                response = next_call(request)
                response.test_attr1 = "sync_middleware1"
                return response

        class TestSyncMiddleware2(SyncMiddleware):
            def handle(self, request, next_call):
                response = next_call(request)
                response.test_attr2 = "sync_middleware2"
                return response

        middleware1 = TestSyncMiddleware1()
        middleware2 = TestSyncMiddleware2()
        chain = SyncMiddlewareChain([middleware1, middleware2])

        mock_request = Mock(spec=httpx.Request)
        mock_response = Mock(spec=httpx.Response)

        mock_transport = Mock(spec=httpx.Client)
        mock_transport.send.return_value = mock_response

        result = chain.execute(mock_request, mock_transport)

        assert result is mock_response
        assert hasattr(result, "test_attr1")
        assert hasattr(result, "test_attr2")
        assert result.test_attr1 == "sync_middleware1"
        assert result.test_attr2 == "sync_middleware2"

    def test_execute_with_sync_client(self):
        chain = SyncMiddlewareChain([])

        mock_request = Mock(spec=httpx.Request)
        mock_response = Mock(spec=httpx.Response)

        mock_client = Mock(spec=httpx.Client)
        mock_client.send.return_value = mock_response

        result = chain.execute(mock_request, mock_client)

        assert result is mock_response
        mock_client.send.assert_called_once_with(mock_request)

    def test_sync_middleware_execution_order(self):
        """Test middleware execution order follows onion pattern."""
        execution_order = []

        class OrderTestMiddleware1(SyncMiddleware):
            def handle(self, request, next_call):
                execution_order.append("middleware1_before")
                response = next_call(request)
                execution_order.append("middleware1_after")
                return response

        class OrderTestMiddleware2(SyncMiddleware):
            def handle(self, request, next_call):
                execution_order.append("middleware2_before")
                response = next_call(request)
                execution_order.append("middleware2_after")
                return response

        middleware1 = OrderTestMiddleware1()
        middleware2 = OrderTestMiddleware2()
        chain = SyncMiddlewareChain([middleware1, middleware2])

        mock_request = Mock(spec=httpx.Request)
        mock_response = Mock(spec=httpx.Response)

        def mock_transport_send(request):
            execution_order.append("transport_send")
            return mock_response

        mock_transport = Mock(spec=httpx.Client)
        mock_transport.send = mock_transport_send

        result = chain.execute(mock_request, mock_transport)

        assert result is mock_response
        expected_order = [
            "middleware1_before",
            "middleware2_before",
            "transport_send",
            "middleware2_after",
            "middleware1_after",
        ]
        assert execution_order == expected_order
