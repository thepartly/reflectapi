"""Tests for batch operations."""

import asyncio
from unittest.mock import AsyncMock, Mock

import pytest

from reflectapi_runtime.batch import BatchClient, BatchContextManager
from reflectapi_runtime.client import AsyncClientBase
from reflectapi_runtime.exceptions import ApiError
from reflectapi_runtime.response import ApiResponse, TransportMetadata


class TestBatchClient:
    def test_initialization(self):
        client = BatchClient[str]()
        assert client.max_concurrent == 10
        assert client._tasks == []
        assert client._semaphore._value == 10

    def test_initialization_custom_concurrency(self):
        client = BatchClient[int](max_concurrent=5)
        assert client.max_concurrent == 5
        assert client._semaphore._value == 5

    def test_add_task(self):
        client = BatchClient[str]()

        async def task1():
            return "result1"

        async def task2():
            return "result2"

        client.add_task(task1)
        client.add_task(task2)

        assert len(client._tasks) == 2

    @pytest.mark.asyncio
    async def test_gather_empty_tasks(self):
        client = BatchClient[str]()
        results = await client.gather()
        assert results == []

    @pytest.mark.asyncio
    async def test_gather_successful_tasks(self):
        from unittest.mock import Mock

        import httpx

        client = BatchClient[str]()

        async def task1():
            metadata = TransportMetadata(
                status_code=200,
                headers=httpx.Headers({}),
                timing=0.1,
                raw_response=Mock(spec=httpx.Response),
            )
            return ApiResponse("result1", metadata)

        async def task2():
            return "result2"

        client.add_task(task1)
        client.add_task(task2)

        results = await client.gather()

        assert len(results) == 2
        assert isinstance(results[0], ApiResponse)
        assert results[0].value == "result1"
        assert isinstance(results[1], ApiResponse)
        assert results[1].value == "result2"

    @pytest.mark.asyncio
    async def test_gather_with_api_errors(self):
        client = BatchClient[str]()

        async def successful_task():
            return "success"

        async def failing_task():
            raise ApiError("API failed")

        client.add_task(successful_task)
        client.add_task(failing_task)

        results = await client.gather()

        assert len(results) == 2
        assert isinstance(results[0], ApiResponse)
        assert results[0].value == "success"
        assert isinstance(results[1], ApiError)
        assert "API failed" in str(results[1])

    @pytest.mark.asyncio
    async def test_gather_with_general_exceptions(self):
        client = BatchClient[str]()

        async def failing_task():
            raise ValueError("Something went wrong")

        client.add_task(failing_task)

        results = await client.gather()

        assert len(results) == 1
        assert isinstance(results[0], ApiError)
        assert "Unexpected error in batch operation" in str(results[0])
        assert isinstance(results[0].cause, ValueError)

    @pytest.mark.asyncio
    async def test_gather_concurrency_control(self):
        """Test concurrency is controlled by semaphore."""
        client = BatchClient[int](max_concurrent=2)

        active_tasks = 0
        max_concurrent_seen = 0

        async def task(delay: float):
            nonlocal active_tasks, max_concurrent_seen
            active_tasks += 1
            max_concurrent_seen = max(max_concurrent_seen, active_tasks)
            await asyncio.sleep(delay)
            active_tasks -= 1
            return active_tasks

        for i in range(5):
            client.add_task(lambda d=i * 0.01: task(d))

        results = await client.gather()

        assert max_concurrent_seen <= 2
        assert len(results) == 5

    def test_clear(self):
        client = BatchClient[str]()

        async def task():
            return "test"

        client.add_task(task)
        assert len(client._tasks) == 1

        client.clear()
        assert len(client._tasks) == 0

    def test_len(self):
        client = BatchClient[str]()
        assert len(client) == 0

        async def task():
            return "test"

        client.add_task(task)
        assert len(client) == 1

        client.add_task(task)
        assert len(client) == 2

    @pytest.mark.asyncio
    async def test_gather_successful(self):
        client = BatchClient[str]()

        async def successful_task():
            return "success"

        async def failing_task():
            raise ApiError("API failed")

        async def another_successful_task():
            from unittest.mock import Mock

            import httpx

            metadata = TransportMetadata(
                status_code=200,
                headers=httpx.Headers({}),
                timing=0.1,
                raw_response=Mock(spec=httpx.Response),
            )
            return ApiResponse("direct_response", metadata)

        client.add_task(successful_task)
        client.add_task(failing_task)
        client.add_task(another_successful_task)

        results = await client.gather_successful()

        assert len(results) == 2
        assert all(isinstance(result, ApiResponse) for result in results)
        assert results[0].value == "success"
        assert results[1].value == "direct_response"

    @pytest.mark.asyncio
    async def test_gather_with_errors(self):
        client = BatchClient[str]()

        async def successful_task1():
            return "success1"

        async def successful_task2():
            return "success2"

        async def failing_task1():
            raise ApiError("API failed 1")

        async def failing_task2():
            raise ValueError("General error")

        client.add_task(successful_task1)
        client.add_task(failing_task1)
        client.add_task(successful_task2)
        client.add_task(failing_task2)

        successes, failures = await client.gather_with_errors()

        assert len(successes) == 2
        assert all(isinstance(result, ApiResponse) for result in successes)
        assert successes[0].value == "success1"
        assert successes[1].value == "success2"

        assert len(failures) == 2
        assert all(isinstance(error, ApiError) for error in failures)
        assert "API failed 1" in str(failures[0])
        assert "Unexpected error in batch operation" in str(failures[1])


class TestBatchContextManager:
    def test_initialization(self):
        client = Mock(spec=AsyncClientBase)
        manager = BatchContextManager(client, max_concurrent=5)

        assert manager.client is client
        assert manager.batch_client.max_concurrent == 5
        assert isinstance(manager.batch_client, BatchClient)

    @pytest.mark.asyncio
    async def test_context_manager_protocol(self):
        client = Mock(spec=AsyncClientBase)
        manager = BatchContextManager(client)

        async with manager as batch:
            assert isinstance(batch, BatchClient)

    @pytest.mark.asyncio
    async def test_context_manager_request_batching(self):
        client = Mock(spec=AsyncClientBase)
        client._make_request = AsyncMock(return_value="mocked_result")

        manager = BatchContextManager(client)

        async with manager as batch:
            batch.add_task(lambda: client._make_request("GET", "/test"))
            results = await batch.gather()

        assert len(results) == 1

    @pytest.mark.asyncio
    async def test_full_workflow(self):
        """Test complete batch workflow with mixed success/failure scenarios."""
        import httpx

        client = Mock(spec=AsyncClientBase)
        client._make_request = AsyncMock()

        manager = BatchContextManager(client, max_concurrent=3)

        async with manager as batch:

            async def get_user(user_id: int):
                metadata = TransportMetadata(
                    status_code=200,
                    headers=httpx.Headers({}),
                    timing=0.1,
                    raw_response=Mock(spec=httpx.Response),
                )
                return ApiResponse({"id": user_id, "name": f"User {user_id}"}, metadata)

            async def get_post_fail():
                from reflectapi_runtime.exceptions import ApiError

                raise ApiError("Post not found")

            batch.add_task(lambda: get_user(1))
            batch.add_task(lambda: get_user(2))
            batch.add_task(get_post_fail)

            results = await batch.gather()

        assert len(results) == 3

        assert isinstance(results[0], ApiResponse)
        assert results[0].value["id"] == 1

        assert isinstance(results[1], ApiResponse)
        assert results[1].value["id"] == 2

        from reflectapi_runtime.exceptions import ApiError

        assert isinstance(results[2], ApiError)
        assert "Post not found" in str(results[2])
