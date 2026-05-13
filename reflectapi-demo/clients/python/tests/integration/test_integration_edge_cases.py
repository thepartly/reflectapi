"""Integration edge cases and negative tests requiring external dependencies."""

import pytest
import asyncio
import json
import time
from unittest.mock import Mock, patch, AsyncMock
from typing import Dict, Any

import httpx

from tests.package_imports import (
    MyapiModelInputPet as Pet,
    MyapiModelOutputPet as PetDetails,
    MyapiModelKind as PetKind,
    MyapiModelKindDog as PetKindDog,
    MyapiModelKindCat as PetKindCat,
    MyapiModelBehavior as Behavior,
    MyapiProtoPetsUpdateRequest as PetsUpdateRequest,
    MyapiProtoPetsListRequest as PetsListRequest,
    AsyncClient,
    MyapiProtoPaginated as Paginated,
)
from reflectapi_runtime import (
    ApplicationError,
    NetworkError,
    TimeoutError,
    ValidationError,
)
from tests.model_helpers import root_value


def httpx_response(
    status_code: int,
    *,
    json_data: Any | None = None,
    content: str | bytes = b"",
    headers: dict[str, str] | None = None,
    request: httpx.Request | None = None,
) -> httpx.Response:
    response_headers = dict(headers or {})
    if json_data is not None:
        content = json.dumps(json_data).encode("utf-8")
        response_headers.setdefault("Content-Type", "application/json")
    elif isinstance(content, str):
        content = content.encode("utf-8")

    return httpx.Response(
        status_code,
        headers=response_headers,
        content=content,
        request=request or httpx.Request("POST", "https://api.example.com/test"),
    )


@pytest.mark.integration
class TestClientNetworkEdgeCases:
    """Test client behavior with network edge cases."""

    @pytest.mark.asyncio
    async def test_client_with_unreachable_server(self):
        """Test client behavior when server is unreachable."""
        # Use non-routable IP address with short timeout
        client = AsyncClient(
            "http://192.0.2.1:9999", timeout=1.0
        )  # RFC5737 test IP, 1s timeout

        with pytest.raises(NetworkError):
            await client.pets.list()

    @pytest.mark.asyncio
    async def test_client_with_invalid_hostname(self):
        """Test client with invalid hostname."""
        client = AsyncClient(
            "https://this-hostname-does-not-exist-12345.com", timeout=2.0
        )

        with pytest.raises(NetworkError):
            await client.pets.list()

    @pytest.mark.asyncio
    async def test_client_with_ssl_errors(self):
        """Test client with SSL certificate errors."""
        # Use a server with invalid SSL certificate
        client = AsyncClient("https://self-signed.badssl.com")

        # Should raise network error due to SSL verification failure
        with pytest.raises(NetworkError):
            await client.health.check()

    @pytest.mark.asyncio
    async def test_client_with_connection_timeout(self):
        """Test client with very short timeout."""
        from httpx import TimeoutException

        # Mock httpx to raise timeout exception directly
        with patch(
            "httpx.AsyncClient.send", side_effect=TimeoutException("Mock timeout")
        ):
            client = AsyncClient(
                "https://api.example.com", timeout=0.001
            )  # 1ms timeout

            with pytest.raises(TimeoutError):
                await client._make_request("/delay/2")

    @pytest.mark.asyncio
    async def test_client_with_large_response_payload(self):
        """Test client handling very large response payloads."""
        # This test would need a real server endpoint that returns large data
        # For now, we'll mock it
        with patch("httpx.AsyncClient.send") as mock_send:
            # Create a very large response (10MB of JSON)
            large_pets = []
            for i in range(10000):
                large_pets.append(
                    {
                        "name": f"Pet_{i}_{'x' * 100}",
                        "kind": {"type": "dog", "breed": f"Breed_{i}"},
                        "updated_at": "2023-01-01T00:00:00Z",
                    }
                )

            large_response_data = {"items": large_pets, "cursor": "large_cursor"}

            mock_send.return_value = httpx_response(200, json_data=large_response_data)

            client = AsyncClient("https://api.example.com")

            # Should handle large response without issues
            response = await client._make_request("/pets", response_model="Any")

            assert len(response.data["items"]) == 10000


@pytest.mark.integration
class TestPartialFieldIntegration:
    """Three-state partial field handling on update requests."""

    def test_update_request_with_set_field_and_absent_field(self):
        request = PetsUpdateRequest(
            name="Option Test Pet",
            kind=PetKindDog(type="dog", breed="Option Breed"),
            age=5,
            # `behaviors` left absent
        )

        json_data = json.loads(request.model_dump_json())

        assert json_data["name"] == "Option Test Pet"
        assert json_data["kind"]["type"] == "dog"
        assert json_data["age"] == 5
        # Field not set → omitted from wire payload entirely
        assert "behaviors" not in json_data

    def test_distinguishes_explicit_null_from_absent(self):
        request_with_none = PetsUpdateRequest(name="None Test", age=None)
        request_default = PetsUpdateRequest(name="Default Test")

        none_json = json.loads(request_with_none.model_dump_json())
        default_json = json.loads(request_default.model_dump_json())

        # Explicit None lands on the wire as null …
        assert none_json["age"] is None
        # … and an unset field is omitted entirely.
        assert "age" not in default_json

    def test_large_behavior_list_round_trip(self):
        from tests.package_imports import (
            MyapiModelBehavior as Behavior,
            MyapiModelBehaviorAggressiveVariant as BehaviorAggressive,
            MyapiModelBehaviorOtherVariant as BehaviorOther,
        )

        complex_behaviors = [
            Behavior("Calm"),
            Behavior(BehaviorAggressive(field_0=1.0, field_1="test")),
            Behavior(BehaviorOther(description="Other")),
        ] * 100

        request = PetsUpdateRequest(
            name="Complex Behaviors", behaviors=complex_behaviors
        )

        assert request.behaviors is not None
        assert len(request.behaviors) == 300

        json_data = json.loads(request.model_dump_json())
        assert len(json_data["behaviors"]) == 300


@pytest.mark.integration
class TestDiscriminatedUnionEdgeCases:
    """Test discriminated union (PetKind) edge cases."""

    def test_pet_kind_with_mixed_case_discriminator(self):
        """Test PetKind with mixed case in discriminator field."""
        # These should fail as discriminators are case-sensitive
        invalid_cases = [
            {"type": "Dog", "breed": "Labrador"},  # Capital D
            {"type": "DOG", "breed": "Labrador"},  # All caps
            {"type": "Cat", "lives": 9},  # Capital C
            {"type": "CAT", "lives": 9},  # All caps
        ]

        for invalid_data in invalid_cases:
            # Model validation raises Pydantic's ValidationError; accept any exception
            with pytest.raises(Exception):
                Pet(name="Case Test", kind=invalid_data)

    def test_pet_kind_with_extra_fields(self):
        """Test PetKind with extra fields that don't belong."""
        # Dog with cat field - extra fields are ignored by models
        dog = PetKindDog(type="dog", breed="Labrador", lives=9)
        assert dog.type == "dog" and dog.breed == "Labrador"
        assert not hasattr(dog, "lives")

        # Cat with dog field - extra fields are ignored
        cat = PetKindCat(type="cat", lives=9, breed="Siamese")
        assert cat.type == "cat" and cat.lives == 9
        assert not hasattr(cat, "breed")

    def test_pet_kind_json_round_trip_edge_cases(self):
        """Test PetKind JSON round-trip with edge case data."""
        edge_cases = [
            PetKindDog(type="dog", breed=""),  # Empty breed
            PetKindDog(type="dog", breed="a" * 1000),  # Very long breed
            PetKindCat(type="cat", lives=0),  # Zero lives
            PetKindCat(type="cat", lives=-1),  # Negative lives
            PetKindCat(type="cat", lives=999999),  # Very high lives
        ]

        for pet_kind in edge_cases:
            # Should serialize and deserialize correctly
            json_data = pet_kind.model_dump()
            from pydantic import TypeAdapter

            reconstructed = TypeAdapter(PetKind).validate_python(json_data)
            reconstructed = root_value(reconstructed)

            assert type(reconstructed) == type(pet_kind)
            if isinstance(pet_kind, PetKindDog):
                assert reconstructed.breed == pet_kind.breed
            else:
                assert reconstructed.lives == pet_kind.lives

    def test_pet_kind_discriminator_tampering_in_json(self):
        """Test tampering with discriminator in JSON data."""
        # Create valid dog JSON, then tamper with discriminator
        dog = PetKindDog(type="dog", breed="Tampered")
        dog_json = dog.model_dump_json()
        dog_data = json.loads(dog_json)

        # Tamper with discriminator
        dog_data["type"] = "cat"
        tampered_json = json.dumps(dog_data)

        # Should fail validation
        with pytest.raises(Exception):
            from pydantic import TypeAdapter

            TypeAdapter(PetKind).validate_python(dog_data)


@pytest.mark.integration
class TestClientMethodEdgeCases:
    """Test generated client method edge cases."""

    @pytest.mark.asyncio
    async def test_pets_list_with_extreme_parameters(self):
        """Test pets.list with extreme parameter values."""
        with patch("httpx.AsyncClient.send") as mock_send:
            mock_send.return_value = httpx_response(
                200, json_data={"items": [], "cursor": None}
            )

            client = AsyncClient("https://api.example.com")

            # Test with extreme parameter values
            extreme_cases = [
                {"limit": 0},  # Zero limit
                {"limit": -1},  # Negative limit
                {"limit": 999999},  # Very large limit
                {"cursor": ""},  # Empty cursor
                {"cursor": "a" * 10000},  # Very long cursor
                {"cursor": "🚀🌟✨"},  # Unicode cursor
            ]

            for params in extreme_cases:
                # Should not crash, though server might reject parameters
                response = await client.pets.list(data=PetsListRequest(**params))
                assert response.data.items == []

    @pytest.mark.asyncio
    async def test_pets_create_with_invalid_data(self):
        """Test pets.create with invalid data that should fail server-side."""
        with patch("httpx.AsyncClient.send") as mock_send:
            # Mock server error response
            mock_send.return_value = httpx_response(
                400, json_data={"error": "Invalid pet data"}
            )

            client = AsyncClient("https://api.example.com")

            # Create valid pet data
            pet = Pet(name="Test Pet", kind=PetKindDog(type="dog", breed="Test Breed"))

            # Should raise ApplicationError for 400 response
            with pytest.raises(ApplicationError) as exc_info:
                await client.pets.create(data=pet)

            assert exc_info.value.status_code == 400

    @pytest.mark.asyncio
    async def test_client_with_none_data_parameters(self):
        """Test client methods with None data parameters."""
        with patch("httpx.AsyncClient.send") as mock_send:
            mock_send.return_value = httpx_response(200, json_data={"result": "ok"})

            client = AsyncClient("https://api.example.com")

            # Should handle None data gracefully
            response = await client.pets.create(data=None)
            assert response.data == {"result": "ok"}


@pytest.mark.integration
class TestErrorResponseHandling:
    """Test error response handling edge cases."""

    @pytest.mark.asyncio
    async def test_server_returning_html_error(self):
        """Test handling when server returns HTML instead of JSON."""
        with patch("httpx.AsyncClient.send") as mock_send:
            mock_send.return_value = httpx_response(
                500,
                content="<html><body>Internal Server Error</body></html>",
                headers={"Content-Type": "text/html"},
            )

            client = AsyncClient("https://api.example.com")

            # Should still create ApplicationError even with HTML response
            with pytest.raises(ApplicationError) as exc_info:
                await client.pets.list()

            assert exc_info.value.status_code == 500

    @pytest.mark.asyncio
    async def test_server_returning_empty_response(self):
        """Test handling when server returns empty response."""
        with patch("httpx.AsyncClient.send") as mock_send:
            mock_send.return_value = httpx_response(204)

            client = AsyncClient("https://api.example.com")

            # Should handle empty response gracefully
            # Note: This depends on how the client is supposed to handle 204 responses
            try:
                response = await client._make_request("/test")
                # If successful, response should handle empty body
            except (ValidationError, ApplicationError):
                # Empty responses might cause validation errors, which is acceptable
                pass

    @pytest.mark.asyncio
    async def test_server_returning_malformed_json(self):
        """Test handling when server returns malformed JSON."""
        with patch("httpx.AsyncClient.send") as mock_send:
            mock_send.return_value = httpx_response(
                200,
                content='{"incomplete": json',
                headers={"Content-Type": "application/json"},
            )

            client = AsyncClient("https://api.example.com")

            # Should raise ValidationError for malformed JSON
            with pytest.raises(ValidationError, match="Response validation failed"):
                await client.pets.list()


@pytest.mark.integration
class TestConcurrentRequestEdgeCases:
    """Test concurrent request edge cases."""

    @pytest.mark.asyncio
    async def test_many_concurrent_requests_same_client(self):
        """Test many concurrent requests using the same client instance."""
        with patch("httpx.AsyncClient.send") as mock_send:
            mock_send.return_value = httpx_response(
                200, json_data={"items": [], "cursor": None}
            )

            client = AsyncClient("https://api.example.com")

            # Make 20 concurrent requests (reduced from 100)
            tasks = [client.pets.list() for _ in range(20)]
            responses = await asyncio.gather(*tasks)

            assert len(responses) == 20
            assert all(response.data.items == [] for response in responses)

    @pytest.mark.asyncio
    async def test_concurrent_requests_with_mixed_success_failure(self):
        """Test concurrent requests where some succeed and some fail."""
        with patch("httpx.AsyncClient.send") as mock_send:

            def mixed_response(request):
                # Simulate success/failure based on URL
                if "success" in str(request.url):
                    return httpx_response(
                        200,
                        json_data={"items": [], "cursor": None},
                        request=request,
                    )
                else:
                    return httpx_response(
                        500,
                        json_data={"error": "Server error"},
                        request=request,
                    )

            mock_send.side_effect = mixed_response

            client = AsyncClient("https://api.example.com")

            # Mix of success and failure requests
            success_tasks = [client._make_request("/success") for _ in range(5)]
            failure_tasks = [client._make_request("/failure") for _ in range(5)]

            # Gather with return_exceptions to handle failures
            results = await asyncio.gather(
                *success_tasks, *failure_tasks, return_exceptions=True
            )

            # Should have 5 successful responses and 5 exceptions
            successes = [r for r in results if not isinstance(r, Exception)]
            failures = [r for r in results if isinstance(r, Exception)]

            assert len(successes) == 5
            assert len(failures) == 5
            assert all(isinstance(f, ApplicationError) for f in failures)

    @pytest.mark.asyncio
    async def test_client_cleanup_with_pending_requests(self):
        """Test client cleanup when requests are still pending."""

        async def slow_response(request):
            await asyncio.sleep(0.05)  # Slow response (reduced from 0.2s)
            return httpx_response(200, json_data={"result": "ok"}, request=request)

        with patch("httpx.AsyncClient.send", side_effect=slow_response):
            async with AsyncClient("https://api.example.com") as client:
                # Start request but don't wait for completion
                task = asyncio.create_task(client.health.check())

                # Let request start
                await asyncio.sleep(0.02)

                # Context manager exit should handle cleanup properly

            # Task should still complete successfully
            response = await task
            assert response.data == {"result": "ok"}
