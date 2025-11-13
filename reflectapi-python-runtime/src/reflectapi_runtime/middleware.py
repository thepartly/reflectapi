"""Middleware system for ReflectAPI Python clients."""

from __future__ import annotations

import asyncio
import logging
import random
from abc import ABC, abstractmethod
from collections.abc import Awaitable, Callable
from typing import Union

import httpx

logger = logging.getLogger(__name__)

# Type aliases for handlers in the middleware chain
AsyncNextHandler = Callable[[httpx.Request], Awaitable[httpx.Response]]
SyncNextHandler = Callable[[httpx.Request], httpx.Response]
NextHandler = Union[AsyncNextHandler, SyncNextHandler]


class AsyncMiddleware(ABC):
    """Base class for asynchronous client middleware.

    Middleware can intercept and transform requests and responses,
    enabling cross-cutting concerns like caching, logging, retry logic, etc.
    """

    @abstractmethod
    async def handle(
        self, request: httpx.Request, next_call: AsyncNextHandler
    ) -> httpx.Response:
        """Handle a request and return a response.

        Args:
            request: The HTTP request to process
            next_call: Async callable to continue the middleware chain

        Returns:
            The HTTP response (possibly modified)
        """
        pass


class SyncMiddleware(ABC):
    """Base class for synchronous client middleware.

    Middleware can intercept and transform requests and responses,
    enabling cross-cutting concerns like caching, logging, retry logic, etc.
    """

    @abstractmethod
    def handle(
        self, request: httpx.Request, next_call: SyncNextHandler
    ) -> httpx.Response:
        """Handle a request and return a response.

        Args:
            request: The HTTP request to process
            next_call: Callable to continue the middleware chain

        Returns:
            The HTTP response (possibly modified)
        """
        pass


class AsyncLoggingMiddleware(AsyncMiddleware):
    """Async middleware that logs request and response information."""

    def __init__(self, logger_name: str = "reflectapi.client") -> None:
        self.logger = logging.getLogger(logger_name)

    async def handle(
        self, request: httpx.Request, next_call: AsyncNextHandler
    ) -> httpx.Response:
        """Log request and response details."""
        self.logger.debug(
            "Making request",
            extra={
                "method": request.method,
                "url": str(request.url),
                "headers": dict(request.headers),
            },
        )

        response = await next_call(request)

        self.logger.debug(
            "Received response",
            extra={
                "status_code": response.status_code,
                "headers": dict(response.headers),
                "url": str(request.url),
            },
        )

        return response


class SyncLoggingMiddleware(SyncMiddleware):
    """Sync middleware that logs request and response information."""

    def __init__(self, logger_name: str = "reflectapi.client") -> None:
        self.logger = logging.getLogger(logger_name)

    def handle(
        self, request: httpx.Request, next_call: SyncNextHandler
    ) -> httpx.Response:
        """Log request and response details."""
        self.logger.debug(
            "Making request",
            extra={
                "method": request.method,
                "url": str(request.url),
                "headers": dict(request.headers),
            },
        )

        response = next_call(request)

        self.logger.debug(
            "Received response",
            extra={
                "status_code": response.status_code,
                "headers": dict(response.headers),
                "url": str(request.url),
            },
        )

        return response


class RetryMiddleware(AsyncMiddleware):
    """Middleware that retries requests on transient failures with exponential backoff and jitter."""

    # Methods that are safe to retry automatically on network failures
    IDEMPOTENT_METHODS = frozenset({"GET", "HEAD", "OPTIONS", "PUT", "DELETE", "TRACE"})

    def __init__(
        self,
        max_retries: int = 3,
        retry_status_codes: set[int] | None = None,
        backoff_factor: float = 0.5,  # Start with a shorter backoff
    ) -> None:
        self.max_retries = max_retries
        self.retry_status_codes = retry_status_codes or {429, 502, 503, 504}
        self.backoff_factor = backoff_factor

    async def handle(
        self, request: httpx.Request, next_call: AsyncNextHandler
    ) -> httpx.Response:
        """Retry requests on transient failures."""
        last_exception = None

        for attempt in range(self.max_retries + 1):
            try:
                response = await next_call(request)

                if (
                    attempt == self.max_retries
                    or response.status_code not in self.retry_status_codes
                ):
                    return response

                # If we are going to retry, store the response as the last exception
                last_exception = response

            except httpx.RequestError as e:
                last_exception = e
                # Do not retry non-idempotent methods on network errors
                if (
                    request.method not in self.IDEMPOTENT_METHODS
                    or attempt == self.max_retries
                ):
                    raise

            # Backoff with Jitter (AWS-recommended approach)
            # Cap the backoff at 30 seconds and add jitter for better distribution
            temp = min(self.backoff_factor * (2**attempt), 30.0)  # Cap backoff
            sleep_duration = temp / 2 + random.uniform(0, temp / 2)

            logger.debug(
                f"Retrying request to {request.url} after {sleep_duration:.2f}s (attempt {attempt + 1}/{self.max_retries})"
            )
            await asyncio.sleep(sleep_duration)

        # This part should be unreachable, but linters might complain.
        # It's better to re-raise the last known exception.
        if isinstance(last_exception, httpx.Response):
            return last_exception
        raise last_exception from None


class AsyncMiddlewareChain:
    """Manages a chain of async middleware for processing requests."""

    def __init__(self, middleware: list[AsyncMiddleware]) -> None:
        self.middleware = middleware

    async def execute(
        self,
        request: httpx.Request,
        transport: httpx.AsyncClient,
    ) -> httpx.Response:
        """Execute the middleware chain with the given request."""

        async def create_handler(
            middleware_list: list[AsyncMiddleware], index: int
        ) -> AsyncNextHandler:
            """Create a handler for the middleware at the given index."""

            async def handler(req: httpx.Request) -> httpx.Response:
                if index >= len(middleware_list):
                    # End of chain - make the actual HTTP request
                    return await transport.send(req)

                # Process through next middleware
                next_handler = await create_handler(middleware_list, index + 1)
                return await middleware_list[index].handle(req, next_handler)

            return handler

        handler = await create_handler(self.middleware, 0)
        return await handler(request)


class SyncMiddlewareChain:
    """Manages a chain of sync middleware for processing requests."""

    def __init__(self, middleware: list[SyncMiddleware]) -> None:
        self.middleware = middleware

    def execute(
        self,
        request: httpx.Request,
        transport: httpx.Client,
    ) -> httpx.Response:
        """Execute the middleware chain with the given request."""

        def create_handler(
            middleware_list: list[SyncMiddleware], index: int
        ) -> SyncNextHandler:
            """Create a handler for the middleware at the given index."""

            def handler(req: httpx.Request) -> httpx.Response:
                if index >= len(middleware_list):
                    # End of chain - make the actual HTTP request
                    return transport.send(req)

                # Process through next middleware
                next_handler = create_handler(middleware_list, index + 1)
                return middleware_list[index].handle(req, next_handler)

            return handler

        handler = create_handler(self.middleware, 0)
        return handler(request)
