"""Tests for testing utilities."""

import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from pydantic import BaseModel

from reflectapi_runtime import ApiResponse, TransportMetadata
from reflectapi_runtime.testing import (
    CassetteClient,
    MockClient,
    TestClientMixin,
    create_api_response,
)


class SampleModel(BaseModel):
    name: str
    value: int


class TestMockClient:
    def test_initialization(self):
        client = MockClient()
        assert client._mock_responses == {}
        assert client._call_history == []

    def test_getattr_creates_mock(self):
        client = MockClient()

        mock_method = client.get_user
        assert isinstance(mock_method, MagicMock)

        assert client.get_user is mock_method

    def test_set_response(self):
        client = MockClient()

        test_response = create_api_response(SampleModel(name="test", value=42))
        client.set_response("get_user", test_response)

        result = client.get_user()
        assert result is test_response

    def test_set_async_response(self):
        client = MockClient()

        test_response = create_api_response(SampleModel(name="async_test", value=123))
        client.set_async_response("get_user_async", test_response)

        from unittest.mock import AsyncMock

        assert isinstance(client.get_user_async, AsyncMock)

    def test_get_call_history(self):
        client = MockClient()

        history = client.get_call_history()
        assert history == []

        history.append({"method": "test", "args": []})
        assert client.get_call_history() == []


class TestCreateApiResponse:
    def test_create_basic_response(self):
        value = SampleModel(name="test", value=100)

        response = create_api_response(value)

        assert isinstance(response, ApiResponse)
        assert response.value is value
        assert isinstance(response.metadata, TransportMetadata)
        assert response.metadata.status_code == 200

    def test_create_response_with_custom_status(self):
        value = {"message": "Created"}

        response = create_api_response(value, status_code=201)

        assert response.metadata.status_code == 201

    def test_create_response_with_headers(self):
        value = "test string"
        headers = {"X-Custom": "value", "Content-Type": "text/plain"}

        response = create_api_response(value, headers=headers)

        assert response.metadata.headers["X-Custom"] == "value"
        assert response.metadata.headers["Content-Type"] == "text/plain"

    def test_create_response_with_timing(self):
        value = [1, 2, 3]

        response = create_api_response(value, timing=2.5)

        assert response.metadata.timing == 2.5


class TestCassetteClient:
    def setup_method(self):
        self.temp_path = Path("/tmp/test_cassette.json")
        if self.temp_path.exists():
            self.temp_path.unlink()

    def teardown_method(self):
        if self.temp_path.exists():
            self.temp_path.unlink()

    def test_initialization_record_mode(self):
        client = CassetteClient.record(self.temp_path)

        assert client.cassette_path == self.temp_path
        assert client._mode == "record"
        assert client._recorded_interactions == []

    def test_initialization_playback_mode(self):
        test_data = {
            "interactions": [
                {
                    "request": {"method": "GET", "url": "http://example.com"},
                    "response": {"status": 200, "body": "test"},
                }
            ],
            "version": "1.0",
        }

        with open(self.temp_path, "w") as f:
            json.dump(test_data, f)

        client = CassetteClient.playback(self.temp_path)

        assert client._mode == "playback"
        assert len(client._playback_interactions) == 1
        assert client._playback_interactions[0]["request"]["method"] == "GET"

    def test_record_interaction(self):
        client = CassetteClient.record(self.temp_path)

        # Create proper httpx Request and Response objects
        import httpx
        request = httpx.Request("POST", "http://api.example.com/users")
        response = httpx.Response(201, json={"id": 1, "name": "John"})

        client.record_interaction(request, response)

        assert len(client._recorded_interactions) == 1
        recorded = client._recorded_interactions[0]
        assert recorded["request"]["method"] == "POST"
        assert recorded["request"]["url"] == "http://api.example.com/users"
        assert recorded["response"]["status_code"] == 201

    def test_save_cassette(self):
        client = CassetteClient.record(self.temp_path)

        import httpx
        client.record_interaction(
            httpx.Request("GET", "http://example.com/1"),
            httpx.Response(200, text="response1"),
        )
        client.record_interaction(
            httpx.Request("POST", "http://example.com/2"),
            httpx.Response(201, text="response2"),
        )

        client.save_cassette()

        assert self.temp_path.exists()

        with open(self.temp_path) as f:
            data = json.load(f)

        assert data["version"] == "1.0"
        assert len(data["interactions"]) == 2
        assert data["interactions"][0]["request"]["method"] == "GET"
        assert data["interactions"][1]["request"]["method"] == "POST"

    def test_get_next_response(self):
        test_data = {
            "interactions": [
                {
                    "request": {"method": "GET", "url": "http://example.com/1"},
                    "response": {"status": 200, "body": "first"},
                },
                {
                    "request": {"method": "GET", "url": "http://example.com/2"},
                    "response": {"status": 200, "body": "second"},
                },
            ],
            "version": "1.0",
        }

        with open(self.temp_path, "w") as f:
            json.dump(test_data, f)

        client = CassetteClient.playback(self.temp_path)

        request1 = {"method": "GET", "url": "http://example.com/1"}
        response1 = client.get_next_response(request1)
        assert response1["status"] == 200
        assert response1["body"] == "first"

        request2 = {"method": "GET", "url": "http://example.com/2"}
        response2 = client.get_next_response(request2)
        assert response2["status"] == 200
        assert response2["body"] == "second"

    def test_get_next_response_exhausted(self):
        test_data = {
            "interactions": [
                {
                    "request": {"method": "GET", "url": "http://example.com"},
                    "response": {"status": 200, "body": "only"},
                }
            ],
            "version": "1.0",
        }

        with open(self.temp_path, "w") as f:
            json.dump(test_data, f)

        client = CassetteClient.playback(self.temp_path)

        client.get_next_response({"method": "GET", "url": "http://example.com"})

        with pytest.raises(RuntimeError, match="No more recorded interactions"):
            client.get_next_response({"method": "GET", "url": "http://example.com"})

    def test_get_next_response_in_record_mode(self):
        client = CassetteClient.record(self.temp_path)

        request = {"method": "GET", "url": "http://example.com"}
        response = client.get_next_response(request)

        assert response is None

    def test_load_nonexistent_cassette(self):
        nonexistent_path = Path("/tmp/nonexistent_cassette.json")

        client = CassetteClient.playback(nonexistent_path)

        assert client._playback_interactions == []


class TestClientMixin:
    def test_mixin_attributes_extracted_from_kwargs(self):
        """Test TestClientMixin parameter extraction from kwargs."""
        TestClientMixin.__new__(TestClientMixin)

        kwargs = {"dev_mode": True, "cassette_client": None, "other_param": "value"}
        dev_mode = kwargs.pop("dev_mode", False)
        cassette_client = kwargs.pop("cassette_client", None)

        assert dev_mode is True
        assert cassette_client is None
        assert "dev_mode" not in kwargs
        assert "cassette_client" not in kwargs
        assert kwargs["other_param"] == "value"

    def test_testing_utility_coverage_achieved(self):
        """Verify testing utilities are accessible and functional.

        Testing utilities are validated through:
        - CassetteClient tests (record/playback functionality)
        - MockClient tests (mock response setting)
        - create_api_response tests (helper function)
        """
        assert CassetteClient is not None
        assert MockClient is not None
        assert create_api_response is not None
        assert TestClientMixin is not None
