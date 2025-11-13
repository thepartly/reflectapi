"""Tests for enhanced features like ApiResponse.__dir__ and testing utilities."""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))

import tempfile
from pathlib import Path
from unittest.mock import AsyncMock, Mock

import httpx
import pytest
from pydantic import BaseModel

from reflectapi_runtime import (
    HAS_HYPOTHESIS,
    ApiResponse,
    AsyncCassetteMiddleware,
    CassetteClient,
    CassetteMiddleware,
    TestClientMixin,
    TransportMetadata,
)

if HAS_HYPOTHESIS:
    import hypothesis.strategies as st
    from hypothesis import given

    from reflectapi_runtime import (
        api_model_strategy,
        enhanced_strategy_for_type,
        strategy_for_pydantic_model,
        strategy_for_type,
    )
else:
    # Provide dummy imports for when Hypothesis is not available
    def given(*args, **kwargs):
        return lambda func: func

    class MockSt:
        @staticmethod
        def just(value):
            return f"MockStrategy({value})"

    st = MockSt()

    def strategy_for_type(x):
        return None

    def strategy_for_pydantic_model(x):
        return None

    def api_model_strategy(model, **kwargs):
        return lambda: None

    def enhanced_strategy_for_type(x):
        return None


class TestModel(BaseModel):
    """Test Pydantic model."""

    name: str
    value: int
    metadata: dict = {}


class TestApiResponseDir:
    """Test enhanced ApiResponse.__dir__ method."""

    def test_dir_with_dict_value(self):
        """Test __dir__ with dictionary value."""
        metadata = TransportMetadata(
            status_code=200,
            headers=httpx.Headers({"content-type": "application/json"}),
            timing=0.1,
            raw_response=Mock(),
        )

        value = {"name": "test", "id": 123, "active": True}
        response = ApiResponse(value, metadata)

        dir_result = dir(response)

        # Should contain ApiResponse attributes
        assert "value" in dir_result
        assert "metadata" in dir_result

        # Should contain dict keys
        assert "name" in dir_result
        assert "id" in dir_result
        assert "active" in dir_result

        # Should be sorted
        assert dir_result == sorted(dir_result)

    def test_dir_with_pydantic_model_v2(self):
        """Test __dir__ with Pydantic V2 model."""
        metadata = TransportMetadata(
            status_code=200, headers=httpx.Headers({}), timing=0.1, raw_response=Mock()
        )

        model = TestModel(name="test", value=42, metadata={"key": "value"})
        response = ApiResponse(model, metadata)

        dir_result = dir(response)

        # Should contain ApiResponse attributes
        assert "value" in dir_result
        assert "metadata" in dir_result

        # Should contain Pydantic model fields
        assert "name" in dir_result
        assert "value" in dir_result

        # Should contain methods from the model class
        if hasattr(model, "model_dump"):
            assert "model_dump" in dir_result
        if hasattr(model, "model_validate"):
            assert "model_validate" in dir_result

    def test_dir_with_custom_object(self):
        """Test __dir__ with custom object."""

        class CustomObject:
            def __init__(self):
                self.name = "custom"
                self.id = 456

            def custom_method(self):
                return "method"

            @property
            def custom_property(self):
                return "property"

        metadata = TransportMetadata(
            status_code=200, headers=httpx.Headers({}), timing=0.1, raw_response=Mock()
        )

        obj = CustomObject()
        response = ApiResponse(obj, metadata)

        dir_result = dir(response)

        # Should contain ApiResponse attributes
        assert "value" in dir_result
        assert "metadata" in dir_result

        # Should contain object attributes
        assert "name" in dir_result
        assert "id" in dir_result
        assert "custom_method" in dir_result
        assert "custom_property" in dir_result

    def test_dir_deduplication(self):
        """Test that __dir__ properly deduplicates attributes."""

        # Create an object with an attribute named 'value' or 'metadata'
        class ConflictObject:
            def __init__(self):
                self.value = "object_value"  # Conflicts with ApiResponse.value
                self.metadata = "object_metadata"  # Conflicts with ApiResponse.metadata
                self.unique_attr = "unique"

        metadata = TransportMetadata(
            status_code=200, headers=httpx.Headers({}), timing=0.1, raw_response=Mock()
        )

        obj = ConflictObject()
        response = ApiResponse(obj, metadata)

        dir_result = dir(response)

        # Should only contain each attribute once
        assert dir_result.count("value") == 1
        assert dir_result.count("metadata") == 1
        assert "unique_attr" in dir_result


class TestCassetteClient:
    """Test CassetteClient functionality."""

    def test_record_mode_initialization(self):
        """Test CassetteClient in record mode."""
        with tempfile.TemporaryDirectory() as temp_dir:
            cassette_path = Path(temp_dir) / "test.json"
            client = CassetteClient.record(cassette_path)

            assert client.is_recording
            assert not client.is_playback
            assert client.allow_new_requests

    def test_playback_mode_initialization(self):
        """Test CassetteClient in playback mode."""
        with tempfile.TemporaryDirectory() as temp_dir:
            cassette_path = Path(temp_dir) / "test.json"

            # Create an empty cassette file
            cassette_path.write_text('{"interactions": []}')

            client = CassetteClient.playback(cassette_path)

            assert not client.is_recording
            assert client.is_playback

    def test_record_interaction(self):
        """Test recording HTTP interactions."""
        with tempfile.TemporaryDirectory() as temp_dir:
            cassette_path = Path(temp_dir) / "test.json"
            client = CassetteClient.record(cassette_path)

            # Create mock request and response
            request = Mock(spec=httpx.Request)
            request.method = "GET"
            request.url = "https://example.com/api"
            request.headers = {"Authorization": "Bearer token"}
            request.content = b""

            response = Mock(spec=httpx.Response)
            response.status_code = 200
            response.headers = {"Content-Type": "application/json"}
            response.content = b'{"result": "success"}'
            response.reason_phrase = "OK"

            # Record the interaction
            client.record_interaction(request, response)

            # Save and verify
            client.save_cassette()

            assert cassette_path.exists()

            # Load and verify content
            import json

            with open(cassette_path) as f:
                data = json.load(f)

            assert len(data["interactions"]) == 1
            interaction = data["interactions"][0]

            assert interaction["request"]["method"] == "GET"
            assert interaction["request"]["url"] == "https://example.com/api"
            assert interaction["response"]["status_code"] == 200

    def test_find_matching_response(self):
        """Test finding matching recorded responses."""
        with tempfile.TemporaryDirectory() as temp_dir:
            cassette_path = Path(temp_dir) / "test.json"

            # Create cassette with recorded interaction
            cassette_data = {
                "interactions": [
                    {
                        "request": {
                            "method": "GET",
                            "url": "https://example.com/api",
                            "headers": {},
                            "content": None,
                        },
                        "response": {
                            "status_code": 200,
                            "headers": {"Content-Type": "application/json"},
                            "content": '{"result": "success"}',
                            "reason_phrase": "OK",
                        },
                    }
                ]
            }

            import json

            with open(cassette_path, "w") as f:
                json.dump(cassette_data, f)

            client = CassetteClient.playback(cassette_path)

            # Create request that should match
            request = Mock(spec=httpx.Request)
            request.method = "GET"
            request.url = "https://example.com/api"

            # Find matching response
            response = client.find_matching_response(request)

            assert response is not None
            assert response.status_code == 200
            assert response.json() == {"result": "success"}

            # Test non-matching request
            request.url = "https://example.com/other"
            response = client.find_matching_response(request)
            assert response is None


class TestCassetteMiddleware:
    """Test CassetteMiddleware integration."""

    def test_sync_middleware_recording(self):
        """Test sync middleware in recording mode."""
        with tempfile.TemporaryDirectory() as temp_dir:
            cassette_path = Path(temp_dir) / "test.json"
            cassette_client = CassetteClient.record(cassette_path)
            middleware = CassetteMiddleware(cassette_client)

            # Mock request, client, and response
            request = Mock(spec=httpx.Request)
            request.method = "GET"
            request.url = "https://example.com/api"
            request.headers = {}
            request.content = b""

            client = Mock(spec=httpx.Client)
            response = Mock(spec=httpx.Response)
            response.status_code = 200
            response.headers = {}
            response.content = b'{"data": "test"}'
            response.reason_phrase = "OK"

            client.send.return_value = response

            # Process request through middleware
            result = middleware.process_request(request, client)

            # Should have made real request
            client.send.assert_called_once_with(request)
            assert result is response

            # Should have recorded interaction
            assert len(cassette_client._recorded_interactions) == 1

    @pytest.mark.asyncio
    async def test_async_middleware_playback(self):
        """Test async middleware in playback mode."""
        with tempfile.TemporaryDirectory() as temp_dir:
            cassette_path = Path(temp_dir) / "test.json"

            # Create cassette with recorded interaction
            cassette_data = {
                "interactions": [
                    {
                        "request": {
                            "method": "GET",
                            "url": "https://example.com/api",
                            "headers": {},
                            "content": None,
                        },
                        "response": {
                            "status_code": 200,
                            "headers": {"Content-Type": "application/json"},
                            "content": '{"data": "test"}',
                            "reason_phrase": "OK",
                        },
                    }
                ]
            }

            import json

            with open(cassette_path, "w") as f:
                json.dump(cassette_data, f)

            cassette_client = CassetteClient.playback(cassette_path)
            middleware = AsyncCassetteMiddleware(cassette_client)

            # Mock request and client
            request = Mock(spec=httpx.Request)
            request.method = "GET"
            request.url = "https://example.com/api"

            client = AsyncMock(spec=httpx.AsyncClient)

            # Process request through middleware
            result = await middleware.process_request(request, client)

            # Should not have made real request
            client.send.assert_not_called()

            # Should have returned recorded response
            assert result.status_code == 200
            assert result.json() == {"data": "test"}


@pytest.mark.skipif(not HAS_HYPOTHESIS, reason="Hypothesis not available")
class TestHypothesisStrategies:
    """Test Hypothesis strategy generation."""

    def test_strategy_for_basic_types(self):
        """Test strategy generation for basic types."""
        assert strategy_for_type(str) is not None
        assert strategy_for_type(int) is not None
        assert strategy_for_type(float) is not None
        assert strategy_for_type(bool) is not None

    def test_strategy_for_pydantic_model(self):
        """Test strategy generation for Pydantic models."""
        strategy = strategy_for_pydantic_model(TestModel)
        assert strategy is not None

        # Test that it generates valid instances
        example = strategy.example()
        assert isinstance(example, TestModel)
        assert isinstance(example.name, str)
        assert isinstance(example.value, int)
        assert isinstance(example.metadata, dict)

    @pytest.mark.skipif(not HAS_HYPOTHESIS, reason="Hypothesis not available")
    @given(api_model_strategy(TestModel, name=st.just("fixed_name")))
    def test_api_model_strategy_with_overrides(self, model_instance):
        """Test api_model_strategy with field overrides."""
        assert isinstance(model_instance, TestModel)
        assert model_instance.name == "fixed_name"
        assert isinstance(model_instance.value, int)

    @pytest.mark.skipif(not HAS_HYPOTHESIS, reason="Hypothesis not available")
    def test_enhanced_strategy_for_type(self):
        """Test enhanced strategy that supports custom strategies."""
        from reflectapi_runtime.hypothesis_strategies import register_custom_strategy

        # Register a custom strategy
        custom_strategy = st.just("custom_value")
        register_custom_strategy(str, custom_strategy)

        # Test that custom strategy is used
        strategy = enhanced_strategy_for_type(str)
        example = strategy.example()
        assert example == "custom_value"


class TestTestClientMixin:
    """Test TestClientMixin functionality."""

    def test_mixin_initialization(self):
        """Test mixin initialization with cassette client."""

        class TestClient(TestClientMixin):
            def __init__(self, base_url, **kwargs):
                super().__init__(**kwargs)
                self.base_url = base_url

        with tempfile.TemporaryDirectory() as temp_dir:
            cassette_path = Path(temp_dir) / "test.json"
            cassette_client = CassetteClient.record(cassette_path)

            client = TestClient(
                "https://api.example.com",
                cassette_client=cassette_client,
                dev_mode=True,
            )

            assert client._dev_mode is True
            assert client._cassette_client is cassette_client

    def test_playback_from_cassette_classmethod(self):
        """Test playback_from_cassette class method."""

        class TestClient(TestClientMixin):
            def __init__(self, base_url, **kwargs):
                super().__init__(**kwargs)
                self.base_url = base_url

        with tempfile.TemporaryDirectory() as temp_dir:
            cassette_path = Path(temp_dir) / "test.json"
            cassette_path.write_text('{"interactions": []}')

            client = TestClient.playback_from_cassette(cassette_path)

            assert client.base_url == "http://test.local"
            assert client._cassette_client is not None
            assert client._cassette_client.is_playback

    def test_record_to_cassette_classmethod(self):
        """Test record_to_cassette class method."""

        class TestClient(TestClientMixin):
            def __init__(self, base_url, **kwargs):
                super().__init__(**kwargs)
                self.base_url = base_url
                self.middleware = kwargs.get("middleware", [])

        with tempfile.TemporaryDirectory() as temp_dir:
            cassette_path = Path(temp_dir) / "test.json"

            client = TestClient.record_to_cassette(
                cassette_path, "https://api.example.com"
            )

            assert client.base_url == "https://api.example.com"
            assert client._cassette_client is not None
            assert client._cassette_client.is_recording
            assert len(client.middleware) > 0
            assert isinstance(client.middleware[0], CassetteMiddleware)


class TestIntegration:
    """Test integration between different enhanced features."""

    def test_option_serialization_with_apiresponse(self):
        """Test Option serialization working with ApiResponse."""
        from reflectapi_runtime import (
            ReflectapiOption,
            Undefined,
            serialize_option_dict,
        )

        # Create response data with Options
        response_data = {
            "name": "test",
            "age": ReflectapiOption(25),
            "email": ReflectapiOption(Undefined),
            "active": ReflectapiOption(None),
        }

        metadata = TransportMetadata(
            status_code=200, headers=httpx.Headers({}), timing=0.1, raw_response=Mock()
        )

        response = ApiResponse(response_data, metadata)

        # Test that we can access the data through delegation
        assert response["name"] == "test"
        assert response["age"].unwrap() == 25

        # Test serialization
        serialized = serialize_option_dict(response.value)
        expected = {
            "name": "test",
            "age": 25,
            "active": None,
            # email excluded because it's undefined
        }
        assert serialized == expected

    def test_complete_workflow(self):
        """Test complete workflow with all enhanced features."""
        # This would be a comprehensive integration test in a real scenario
        # For now, just verify that all components can be imported and instantiated

        from reflectapi_runtime import (
            ApiResponse,
            CassetteClient,
            ReflectapiOption,
            TestClientMixin,
            TransportMetadata,
            Undefined,
        )

        # Create Option
        option = ReflectapiOption(42)
        assert option.is_some

        # Create CassetteClient
        with tempfile.TemporaryDirectory() as temp_dir:
            cassette_path = Path(temp_dir) / "test.json"
            client = CassetteClient.record(cassette_path)
            assert client.is_recording

        # Create ApiResponse with enhanced __dir__
        metadata = TransportMetadata(
            status_code=200, headers=httpx.Headers({}), timing=0.1, raw_response=Mock()
        )
        response = ApiResponse({"data": "test"}, metadata)
        dir_result = dir(response)
        assert "data" in dir_result
        assert "value" in dir_result
        assert "metadata" in dir_result

        # All components working together
        assert True  # If we get here, everything imported and worked correctly
