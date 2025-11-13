"""Test the generated client classes."""

import pytest
import warnings
from unittest.mock import Mock, AsyncMock, patch

from generated import (
    AsyncClient,
    MyapiModelInputPet as Pet,
    MyapiModelKind as PetKind,
    MyapiProtoPetsListRequest as PetsListRequest,
    MyapiProtoPetsRemoveRequest as PetsRemoveRequest,
)
from reflectapi_runtime import ApiResponse


class TestAsyncClient:
    """Test AsyncClient functionality."""

    def test_async_client_creation(self):
        """Test creating AsyncClient."""
        client = AsyncClient("http://test.example")
        assert client.base_url == "http://test.example"

    def test_async_client_has_all_methods(self):
        """Test AsyncClient has all expected methods."""
        client = AsyncClient("http://test.example")

        # Check namespace structure exists
        assert hasattr(client, "pets")
        assert hasattr(client, "health")

        # Check all expected pets methods exist
        assert hasattr(client.pets, "create")
        assert hasattr(client.pets, "list")
        assert hasattr(client.pets, "update")
        assert hasattr(client.pets, "remove")
        assert hasattr(client.pets, "delete")  # deprecated
        assert hasattr(client.pets, "get_first")

        # Check health methods exist
        assert hasattr(client.health, "check")

    @pytest.mark.asyncio
    async def test_async_client_pets_create_docstring(self):
        """Test pets.create has proper docstring."""
        client = AsyncClient("http://test.example")
        docstring = client.pets.create.__doc__

        assert "Create a new pet" in docstring
        assert "Args:" in docstring
        assert "Returns:" in docstring
        assert "ApiResponse[Any]" in docstring

    @pytest.mark.asyncio
    async def test_deprecated_method_warning(self):
        """Test deprecated method shows warning."""
        client = AsyncClient("http://test.example")

        with patch.object(client, "_make_request", return_value=Mock()) as mock_request:
            with warnings.catch_warnings(record=True) as w:
                warnings.simplefilter("always")
                await client.pets.delete(PetsRemoveRequest(name="test"))

                # Check deprecation warning was raised
                assert len(w) == 1
                assert issubclass(w[0].category, DeprecationWarning)
                assert "pets_delete is deprecated" in str(w[0].message)


# Sync client tests removed - only AsyncClient is generated


class TestClientMethodSignatures:
    """Test client method signatures are correct."""

    def test_pets_create_signature(self):
        """Test pets.create method signature."""
        client = AsyncClient("http://test.example")

        # Should accept Pet as data parameter
        import inspect

        sig = inspect.signature(client.pets.create)
        params = list(sig.parameters.keys())

        # Note: inspect.signature on bound methods doesn't include 'self'
        assert "data" in params

    def test_pets_get_first_no_params(self):
        """Test pets.get_first has no data parameter."""
        client = AsyncClient("http://test.example")

        import inspect

        sig = inspect.signature(client.pets.get_first)
        params = list(sig.parameters.keys())

        # Should have headers parameter but no data parameter
        assert "data" not in params
        assert "headers" in params

    def test_health_check_no_params(self):
        """Test health.check has no data parameter."""
        client = AsyncClient("http://test.example")

        import inspect

        sig = inspect.signature(client.health.check)
        params = list(sig.parameters.keys())

        # Should have no parameters (no data)
        assert params == []


class TestClientBaseUrls:
    """Test client base URL handling."""

    def test_async_client_base_url_storage(self):
        """Test AsyncClient stores base URL correctly."""
        client = AsyncClient("https://api.example.com/v1")
        assert client.base_url == "https://api.example.com/v1"

    # Sync client test removed - only AsyncClient is generated


class TestClientInheritance:
    """Test client inheritance from base classes."""

    def test_async_client_inheritance(self):
        """Test AsyncClient inherits from AsyncClientBase."""
        from reflectapi_runtime import AsyncClientBase

        client = AsyncClient("http://test.example")
        assert isinstance(client, AsyncClientBase)

    # Sync client test removed - only AsyncClient is generated
