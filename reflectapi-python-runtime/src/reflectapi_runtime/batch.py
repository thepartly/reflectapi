"""Batch operations support for ReflectAPI Python clients."""

from __future__ import annotations

import asyncio
from typing import TYPE_CHECKING, Any, Generic


# Sentinel object for batch operations where no HTTP response exists
class _BatchNoResponse:
    """Sentinel object representing the absence of an HTTP response in batch operations."""

    def __repr__(self) -> str:
        return "<BatchNoResponse>"


# Singleton instance
BATCH_NO_RESPONSE = _BatchNoResponse()

import httpx

if TYPE_CHECKING:
    from collections.abc import Awaitable, Callable

from .exceptions import ApiError
from .response import ApiResponse, TransportMetadata
from .types import BatchResult, T


class BatchClient(Generic[T]):
    """Client for executing batch operations with concurrency control."""

    def __init__(self, max_concurrent: int = 10) -> None:
        self.max_concurrent = max_concurrent
        self._tasks: list[Callable[[], Awaitable[Any]]] = []
        self._semaphore: asyncio.Semaphore = asyncio.Semaphore(max_concurrent)

    def add_task(self, coro_func: Callable[[], Awaitable[T]]) -> None:
        """Add a coroutine function to be executed in the batch."""
        self._tasks.append(coro_func)

    async def gather(self) -> list[BatchResult[Any]]:
        """Execute all tasks concurrently and return results.

        Returns a list where each item is either an ApiResponse (success)
        or an ApiError (failure). This allows handling partial failures
        in batch operations.
        """
        if not self._tasks:
            return []

        async def execute_with_semaphore(
            task: Callable[[], Awaitable[T]],
        ) -> BatchResult[T]:
            """Execute a single task with semaphore control."""
            async with self._semaphore:
                try:
                    result = await task()
                    # If result is already an ApiResponse or ApiError, return as-is
                    if isinstance(result, ApiResponse | ApiError):
                        return result
                    # Otherwise wrap in ApiResponse with dummy metadata
                    # No HTTP request occurred - using sentinel for raw_response
                    metadata = TransportMetadata(
                        status_code=200,
                        headers=httpx.Headers({}),
                        timing=0.0,
                        raw_response=BATCH_NO_RESPONSE,  # Sentinel for batch operations
                    )
                    return ApiResponse(result, metadata)
                except Exception as e:
                    if isinstance(e, ApiError):
                        return e
                    # Wrap other exceptions as ApiErrors
                    return ApiError(
                        f"Unexpected error in batch operation: {e}", cause=e
                    )

        # Execute all tasks concurrently
        results = await asyncio.gather(
            *[execute_with_semaphore(task) for task in self._tasks],
            return_exceptions=False,  # We handle exceptions manually
        )

        return results

    async def gather_successful(self) -> list[ApiResponse[Any]]:
        """Execute all tasks and return only successful results.

        Failed operations are silently ignored. Use gather() if you need
        to handle failures explicitly.
        """
        results = await self.gather()
        return [result for result in results if isinstance(result, ApiResponse)]

    async def gather_with_errors(self) -> tuple[list[ApiResponse[Any]], list[ApiError]]:
        """Execute all tasks and return successes and failures separately."""
        results = await self.gather()

        successes: list[ApiResponse[Any]] = []
        failures: list[ApiError] = []

        for result in results:
            if isinstance(result, ApiResponse):
                successes.append(result)
            else:
                failures.append(result)

        return successes, failures

    def __len__(self) -> int:
        """Return the number of tasks in the batch."""
        return len(self._tasks)

    def clear(self) -> None:
        """Clear all tasks from the batch."""
        self._tasks.clear()


class BatchContextManager:
    """Context manager for batch operations that integrates with clients."""

    def __init__(self, client: Any, max_concurrent: int = 10) -> None:
        self.client = client
        self.batch_client = BatchClient[Any](max_concurrent)
        self._original_make_request: Callable[..., Any] | None = None

    async def __aenter__(self) -> BatchClient[Any]:
        """Enter the batch context and intercept client requests."""
        # Store the original _make_request method
        self._original_make_request = self.client._make_request

        # Replace with a batching version
        def batch_request(*args: Any, **kwargs: Any) -> Callable[[], Awaitable[Any]]:
            """Create a task that will be executed later."""

            async def task() -> Any:
                # Restore original method temporarily for execution
                if self._original_make_request:
                    return await self._original_make_request(*args, **kwargs)
                raise RuntimeError("Original request method not available")

            self.batch_client.add_task(task)
            return task  # Return the task for potential individual awaiting

        self.client._make_request = batch_request
        return self.batch_client

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Exit the batch context and restore original client behavior."""
        # Restore the original _make_request method
        if self._original_make_request:
            self.client._make_request = self._original_make_request


# Mixin for clients to add batch support
class BatchMixin:
    """Mixin that adds batch operation support to clients."""

    def batch(self, max_concurrent: int = 10) -> BatchContextManager:
        """Create a batch context for concurrent operations.

        Usage:
            async with client.batch(max_concurrent=10) as batch:
                tasks = [client.create_pet(Pet(name=f"Pet_{i}")) for i in range(100)]
                results = await batch.gather()
        """
        return BatchContextManager(self, max_concurrent)
