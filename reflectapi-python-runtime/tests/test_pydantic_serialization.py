"""Tests for Pydantic request serialization in client classes."""

from unittest.mock import AsyncMock, Mock

import httpx
import pytest
from pydantic import BaseModel, Field

from reflectapi_runtime import (
    ApiResponse,
    AsyncClientBase,
    ClientBase,
    TransportMetadata,
)


class RequestModel(BaseModel):
    """Test model for request serialization."""

    name: str = Field(description="User name")
    age: int = Field(ge=0, description="User age")
    email: str = Field(description="User email")
    active: bool = Field(default=True, description="Whether user is active")


class ResponseModel(BaseModel):
    """Test model for response deserialization."""

    id: int
    message: str


@pytest.fixture
def mock_httpx_client():
    """Mock httpx.Client for sync tests."""
    mock_client = Mock(spec=httpx.Client)
    mock_response = Mock(spec=httpx.Response)
    mock_response.status_code = 201
    mock_response.headers = httpx.Headers({"Content-Type": "application/json"})
    mock_response.reason_phrase = "Created"
    mock_response.json.return_value = {"id": 123, "message": "Created successfully"}
    mock_response.content = b'{"id": 123, "message": "Created successfully"}'

    mock_request = Mock(spec=httpx.Request)
    mock_client.build_request.return_value = mock_request
    mock_client.send.return_value = mock_response

    return mock_client, mock_request, mock_response


@pytest.fixture
def mock_async_httpx_client():
    """Mock httpx.AsyncClient for async tests."""
    mock_client = AsyncMock(spec=httpx.AsyncClient)
    mock_response = Mock(spec=httpx.Response)
    mock_response.status_code = 201
    mock_response.headers = httpx.Headers({"Content-Type": "application/json"})
    mock_response.reason_phrase = "Created"
    mock_response.json.return_value = {"id": 123, "message": "Created successfully"}
    mock_response.content = b'{"id": 123, "message": "Created successfully"}'

    mock_request = Mock(spec=httpx.Request)
    mock_client.build_request.return_value = mock_request
    mock_client.send = AsyncMock(return_value=mock_response)

    return mock_client, mock_request, mock_response


class TestClientBasePydanticSerialization:
    """Test Pydantic model serialization in sync ClientBase."""

    def test_make_request_with_pydantic_model(self, mock_httpx_client):
        """Test making a request with a Pydantic model for JSON serialization."""
        mock_client, mock_request, mock_response = mock_httpx_client

        client = ClientBase("http://example.com", client=mock_client)

        request_data = RequestModel(
            name="John Doe", age=30, email="john@example.com", active=True
        )

        result = client._make_request(
            "POST", "/users", json_model=request_data, response_model=ResponseModel
        )

        # Verify response
        assert isinstance(result, ApiResponse)
        assert isinstance(result.value, ResponseModel)
        assert result.value.id == 123
        assert result.value.message == "Created successfully"
        assert isinstance(result.metadata, TransportMetadata)
        assert result.metadata.status_code == 201

        # Verify request was built with correct parameters
        mock_client.build_request.assert_called_once()
        call_args = mock_client.build_request.call_args
        assert call_args[1]["method"] == "POST"
        assert call_args[1]["url"] == "http://example.com/users"
        assert call_args[1]["params"] is None

        # Verify content and headers
        expected_json = (
            '{"name":"John Doe","age":30,"email":"john@example.com","active":true}'
        )
        assert call_args[1]["content"] == expected_json.encode("utf-8")
        assert call_args[1]["headers"] == {"Content-Type": "application/json"}

        # Ensure json parameter was not used
        assert "json" not in call_args[1]

    def test_make_request_with_pydantic_model_no_response_model(
        self, mock_httpx_client
    ):
        """Test making a request with Pydantic model but no response model."""
        mock_client, mock_request, mock_response = mock_httpx_client
        mock_response.json.return_value = {"success": True}
        mock_response.content = b'{"success": true}'

        client = ClientBase("http://example.com", client=mock_client)

        request_data = RequestModel(name="Jane Smith", age=25, email="jane@example.com")

        result = client._make_request("POST", "/users", json_model=request_data)

        # Should return dict when no response model specified
        assert isinstance(result, ApiResponse)
        assert result.value == {"success": True}

    def test_make_request_cannot_specify_both_json_data_and_model(
        self, mock_httpx_client
    ):
        """Test that specifying both json_data and json_model raises ValueError."""
        mock_client, mock_request, mock_response = mock_httpx_client

        client = ClientBase("http://example.com", client=mock_client)

        request_data = RequestModel(name="Test User", age=20, email="test@example.com")

        with pytest.raises(
            ValueError, match="Cannot specify both json_data and json_model"
        ):
            client._make_request(
                "POST",
                "/users",
                json_data={"name": "dict data"},
                json_model=request_data,
                response_model=ResponseModel,
            )

    def test_make_request_with_json_data_still_works(self, mock_httpx_client):
        """Test that traditional json_data parameter still works."""
        mock_client, mock_request, mock_response = mock_httpx_client

        client = ClientBase("http://example.com", client=mock_client)

        result = client._make_request(
            "POST",
            "/users",
            json_data={"name": "Dictionary User", "age": 35},
            response_model=ResponseModel,
        )

        # Verify response
        assert isinstance(result, ApiResponse)
        assert isinstance(result.value, ResponseModel)

        # Verify request was built with json parameter (not content)
        mock_client.build_request.assert_called_once()
        call_args = mock_client.build_request.call_args
        assert call_args[1]["json"] == {"name": "Dictionary User", "age": 35}
        assert "content" not in call_args[1]
        assert "headers" not in call_args[1]

    def test_make_request_with_params_and_pydantic_model(self, mock_httpx_client):
        """Test making a request with both query params and Pydantic model."""
        mock_client, mock_request, mock_response = mock_httpx_client

        client = ClientBase("http://example.com", client=mock_client)

        request_data = RequestModel(
            name="Param User", age=40, email="param@example.com"
        )

        result = client._make_request(
            "POST",
            "/users",
            params={"source": "api", "version": "v1"},
            json_model=request_data,
            response_model=ResponseModel,
        )

        # Verify response
        assert isinstance(result, ApiResponse)
        assert isinstance(result.value, ResponseModel)

        # Verify request was built with both params and content
        mock_client.build_request.assert_called_once()
        call_args = mock_client.build_request.call_args
        assert call_args[1]["params"] == {"source": "api", "version": "v1"}

        expected_json = (
            '{"name":"Param User","age":40,"email":"param@example.com","active":true}'
        )
        assert call_args[1]["content"] == expected_json.encode("utf-8")
        assert call_args[1]["headers"] == {"Content-Type": "application/json"}


class TestAsyncClientBasePydanticSerialization:
    """Test Pydantic model serialization in async AsyncClientBase."""

    @pytest.mark.asyncio
    async def test_make_request_with_pydantic_model(self, mock_async_httpx_client):
        """Test making an async request with a Pydantic model for JSON serialization."""
        mock_client, mock_request, mock_response = mock_async_httpx_client

        client = AsyncClientBase("http://example.com", client=mock_client)

        request_data = RequestModel(
            name="Async User", age=28, email="async@example.com", active=False
        )

        result = await client._make_request(
            "PUT", "/users/123", json_model=request_data, response_model=ResponseModel
        )

        # Verify response
        assert isinstance(result, ApiResponse)
        assert isinstance(result.value, ResponseModel)
        assert result.value.id == 123
        assert result.value.message == "Created successfully"

        # Verify request was built with correct parameters
        mock_client.build_request.assert_called_once()
        call_args = mock_client.build_request.call_args
        assert call_args[1]["method"] == "PUT"
        assert call_args[1]["url"] == "http://example.com/users/123"

        # Verify content and headers
        expected_json = (
            '{"name":"Async User","age":28,"email":"async@example.com","active":false}'
        )
        assert call_args[1]["content"] == expected_json.encode("utf-8")
        assert call_args[1]["headers"] == {"Content-Type": "application/json"}

    @pytest.mark.asyncio
    async def test_make_request_cannot_specify_both_json_data_and_model(
        self, mock_async_httpx_client
    ):
        """Test that specifying both json_data and json_model raises ValueError in async client."""
        mock_client, mock_request, mock_response = mock_async_httpx_client

        client = AsyncClientBase("http://example.com", client=mock_client)

        request_data = RequestModel(
            name="Conflict User", age=30, email="conflict@example.com"
        )

        with pytest.raises(
            ValueError, match="Cannot specify both json_data and json_model"
        ):
            await client._make_request(
                "POST",
                "/users",
                json_data={"name": "dict data"},
                json_model=request_data,
                response_model=ResponseModel,
            )

    @pytest.mark.asyncio
    async def test_make_request_with_json_data_still_works(
        self, mock_async_httpx_client
    ):
        """Test that traditional json_data parameter still works in async client."""
        mock_client, mock_request, mock_response = mock_async_httpx_client

        client = AsyncClientBase("http://example.com", client=mock_client)

        result = await client._make_request(
            "POST",
            "/users",
            json_data={"name": "Async Dict User", "age": 33},
            response_model=ResponseModel,
        )

        # Verify response
        assert isinstance(result, ApiResponse)
        assert isinstance(result.value, ResponseModel)

        # Verify request was built with json parameter (not content)
        mock_client.build_request.assert_called_once()
        call_args = mock_client.build_request.call_args
        assert call_args[1]["json"] == {"name": "Async Dict User", "age": 33}
        assert "content" not in call_args[1]


class TestPydanticSerializationEdgeCases:
    """Test edge cases and error scenarios for Pydantic serialization."""

    def test_complex_nested_model_serialization(self, mock_httpx_client):
        """Test serialization of complex nested Pydantic models."""

        class AddressModel(BaseModel):
            street: str
            city: str
            country: str = "USA"

        class ComplexRequestModel(BaseModel):
            user: RequestModel
            address: AddressModel
            tags: list[str] = []
            metadata: dict[str, str] = {}

        mock_client, mock_request, mock_response = mock_httpx_client
        client = ClientBase("http://example.com", client=mock_client)

        complex_data = ComplexRequestModel(
            user=RequestModel(name="Complex User", age=45, email="complex@example.com"),
            address=AddressModel(street="123 Main St", city="Anytown"),
            tags=["premium", "verified"],
            metadata={"source": "test", "priority": "high"},
        )

        client._make_request(
            "POST", "/complex", json_model=complex_data, response_model=ResponseModel
        )

        # Verify the complex model was serialized correctly
        mock_client.build_request.assert_called_once()
        call_args = mock_client.build_request.call_args

        # Parse the serialized content to verify structure
        import json

        serialized_content = call_args[1]["content"].decode("utf-8")
        parsed_data = json.loads(serialized_content)

        assert parsed_data["user"]["name"] == "Complex User"
        assert parsed_data["user"]["age"] == 45
        assert parsed_data["address"]["street"] == "123 Main St"
        assert parsed_data["address"]["country"] == "USA"  # Default value
        assert parsed_data["tags"] == ["premium", "verified"]
        assert parsed_data["metadata"]["source"] == "test"

    def test_empty_pydantic_model(self, mock_httpx_client):
        """Test serialization of empty Pydantic model."""

        class EmptyModel(BaseModel):
            pass

        mock_client, mock_request, mock_response = mock_httpx_client
        client = ClientBase("http://example.com", client=mock_client)

        empty_data = EmptyModel()

        client._make_request(
            "POST", "/empty", json_model=empty_data, response_model=ResponseModel
        )

        # Verify the empty model serializes to empty JSON object
        mock_client.build_request.assert_called_once()
        call_args = mock_client.build_request.call_args
        assert call_args[1]["content"] == b"{}"
        assert call_args[1]["headers"] == {"Content-Type": "application/json"}
