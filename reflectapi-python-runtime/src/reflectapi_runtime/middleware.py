"""Middleware system for ReflectAPI Python clients.

Middleware operates on the transport-agnostic
:class:`~reflectapi_runtime.transport.Request` /
:class:`~reflectapi_runtime.transport.Response` shape, so the same
middleware works whether the underlying transport is an ``httpx.Client``
or a user-defined :class:`~reflectapi_runtime.transport.Client`.
"""

from __future__ import annotations

import asyncio
import logging
import random
from abc import ABC, abstractmethod
from collections.abc import Awaitable, Callable
from typing import Union

import httpx

from .transport import Request, Response

logger = logging.getLogger(__name__)

# Type aliases for the per-request handler chain.
AsyncNextHandler = Callable[[Request], Awaitable[Response]]
SyncNextHandler = Callable[[Request], Response]
NextHandler = Union[AsyncNextHandler, SyncNextHandler]


class AsyncMiddleware(ABC):
    """Base class for asynchronous client middleware."""

    @abstractmethod
    async def handle(
        self, request: Request, next_call: AsyncNextHandler
    ) -> Response:
        """Handle a request and return a response.

        Args:
            request: the transport request about to be sent.
            next_call: invoke to continue the middleware chain.

        Returns:
            The response to surface to the caller (possibly modified).
        """
        ...


class SyncMiddleware(ABC):
    """Base class for synchronous client middleware."""

    @abstractmethod
    def handle(
        self, request: Request, next_call: SyncNextHandler
    ) -> Response:
        """Handle a request and return a response.

        Args:
            request: the transport request about to be sent.
            next_call: invoke to continue the middleware chain.
        """
        ...


# ---------------------------------------------------------------------------
# Built-in middleware
# ---------------------------------------------------------------------------


def _request_log_extra(request: Request) -> dict[str, object]:
    return {
        "path": request.path,
        "headers": dict(request.headers),
    }


def _response_log_extra(
    request: Request, response: Response
) -> dict[str, object]:
    return {
        "status": response.status,
        "path": request.path,
        # Headers can be httpx.Headers or a dict — both behave like mappings.
        "headers": dict(response.headers),
    }


class AsyncLoggingMiddleware(AsyncMiddleware):
    """Async middleware that logs request and response information."""

    def __init__(self, logger_name: str = "reflectapi.client") -> None:
        self.logger = logging.getLogger(logger_name)

    async def handle(
        self, request: Request, next_call: AsyncNextHandler
    ) -> Response:
        self.logger.debug("Making request", extra=_request_log_extra(request))
        response = await next_call(request)
        self.logger.debug(
            "Received response", extra=_response_log_extra(request, response)
        )
        return response


class SyncLoggingMiddleware(SyncMiddleware):
    """Sync middleware that logs request and response information."""

    def __init__(self, logger_name: str = "reflectapi.client") -> None:
        self.logger = logging.getLogger(logger_name)

    def handle(
        self, request: Request, next_call: SyncNextHandler
    ) -> Response:
        self.logger.debug("Making request", extra=_request_log_extra(request))
        response = next_call(request)
        self.logger.debug(
            "Received response", extra=_response_log_extra(request, response)
        )
        return response


class RetryMiddleware(AsyncMiddleware):
    """Retry transient failures with exponential backoff and jitter.

    Retries on 4xx/5xx status codes from ``retry_status_codes`` and on any
    exception of type ``retry_on`` raised by the downstream handler.
    Defaults retry exceptions to ``httpx.RequestError`` so the existing
    httpx-backed flow keeps working; users with custom transports can pass
    their own exception types via ``retry_on``.
    """

    def __init__(
        self,
        max_retries: int = 3,
        retry_status_codes: set[int] | None = None,
        backoff_factor: float = 0.5,
        retry_on: tuple[type[Exception], ...] = (httpx.RequestError,),
        retry_non_idempotent: bool = False,
    ) -> None:
        self.max_retries = max_retries
        self.retry_status_codes = retry_status_codes or {429, 502, 503, 504}
        self.backoff_factor = backoff_factor
        self.retry_on = retry_on
        self.retry_non_idempotent = retry_non_idempotent

    async def handle(
        self, request: Request, next_call: AsyncNextHandler
    ) -> Response:
        last_exception: Exception | None = None
        last_response: Response | None = None

        for attempt in range(self.max_retries + 1):
            try:
                response = await next_call(request)
                if (
                    attempt == self.max_retries
                    or response.status not in self.retry_status_codes
                ):
                    return response
                last_response = response
            except self.retry_on as e:
                last_exception = e
                # reflectapi is always POST; only retry on network errors
                # if the caller has opted in.
                if (
                    not self.retry_non_idempotent
                    or attempt == self.max_retries
                ):
                    raise

            # Exponential backoff with jitter, capped at 30s.
            cap = min(self.backoff_factor * (2**attempt), 30.0)
            sleep_duration = cap / 2 + random.uniform(0, cap / 2)
            logger.debug(
                "Retrying %s after %.2fs (attempt %d/%d)",
                request.path,
                sleep_duration,
                attempt + 1,
                self.max_retries,
            )
            await asyncio.sleep(sleep_duration)

        # Loop ended without an early return: surface whatever we last saw.
        if last_response is not None:
            return last_response
        if last_exception is not None:
            raise last_exception from None
        # Unreachable: the loop body either returns, raises, or sets one of
        # the two `last_*` slots before continuing.
        raise RuntimeError("RetryMiddleware exhausted retries without an outcome")


# ---------------------------------------------------------------------------
# Middleware chains
# ---------------------------------------------------------------------------


class AsyncMiddlewareChain:
    """Drive an async middleware list around a terminal handler."""

    def __init__(self, middleware: list[AsyncMiddleware]) -> None:
        self.middleware = middleware

    async def execute(
        self,
        request: Request,
        terminal: AsyncNextHandler,
    ) -> Response:
        """Run ``request`` through every middleware, ending in ``terminal``.

        ``terminal`` is the handler that actually performs the request —
        the chain itself is transport-agnostic.
        """

        async def chain_at(index: int) -> AsyncNextHandler:
            async def handler(req: Request) -> Response:
                if index >= len(self.middleware):
                    return await terminal(req)
                next_handler = await chain_at(index + 1)
                return await self.middleware[index].handle(req, next_handler)

            return handler

        handler = await chain_at(0)
        return await handler(request)


class SyncMiddlewareChain:
    """Drive a sync middleware list around a terminal handler."""

    def __init__(self, middleware: list[SyncMiddleware]) -> None:
        self.middleware = middleware

    def execute(
        self,
        request: Request,
        terminal: SyncNextHandler,
    ) -> Response:
        """Run ``request`` through every middleware, ending in ``terminal``."""

        def chain_at(index: int) -> SyncNextHandler:
            def handler(req: Request) -> Response:
                if index >= len(self.middleware):
                    return terminal(req)
                next_handler = chain_at(index + 1)
                return self.middleware[index].handle(req, next_handler)

            return handler

        handler = chain_at(0)
        return handler(request)
