"""Test the generated client classes."""

import pytest
import warnings
from unittest.mock import Mock, AsyncMock, patch

from generated import AsyncClient, Client, Pet, PetKind, PetsListRequest, PetsRemoveRequest
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
        
        # Check all expected methods exist
        assert hasattr(client, 'pets_create')
        assert hasattr(client, 'pets_list')
        assert hasattr(client, 'pets_update')
        assert hasattr(client, 'pets_remove')
        assert hasattr(client, 'pets_delete')  # deprecated
        assert hasattr(client, 'pets_get_first')
        assert hasattr(client, 'health_check')
    
    @pytest.mark.asyncio
    async def test_async_client_pets_create_docstring(self):
        """Test pets_create has proper docstring."""
        client = AsyncClient("http://test.example")
        docstring = client.pets_create.__doc__
        
        assert "Create a new pet" in docstring
        assert "Args:" in docstring
        assert "Returns:" in docstring
        assert "ApiResponse[Any]" in docstring
    
    @pytest.mark.asyncio
    async def test_deprecated_method_warning(self):
        """Test deprecated method shows warning."""
        client = AsyncClient("http://test.example")
        
        with patch.object(client, '_make_request', return_value=Mock()) as mock_request:
            with warnings.catch_warnings(record=True) as w:
                warnings.simplefilter("always")
                await client.pets_delete(PetsRemoveRequest(name="test"))
                
                # Check deprecation warning was raised
                assert len(w) == 1
                assert issubclass(w[0].category, DeprecationWarning)
                assert "pets_delete is deprecated" in str(w[0].message)


class TestSyncClient:
    """Test synchronous Client functionality."""
    
    def test_sync_client_creation(self):
        """Test creating sync Client."""
        client = Client("http://test.example")
        assert client.base_url == "http://test.example"
    
    def test_sync_client_has_all_methods(self):
        """Test sync Client has all expected methods."""
        client = Client("http://test.example")
        
        # Check all expected methods exist
        assert hasattr(client, 'pets_create')
        assert hasattr(client, 'pets_list')
        assert hasattr(client, 'pets_update')
        assert hasattr(client, 'pets_remove')
        assert hasattr(client, 'pets_delete')  # deprecated
        assert hasattr(client, 'pets_get_first')
        assert hasattr(client, 'health_check')
    
    def test_sync_client_pets_list_docstring(self):
        """Test pets_list has proper docstring."""
        client = Client("http://test.example")
        docstring = client.pets_list.__doc__
        
        assert "List available pets" in docstring
        assert "Args:" in docstring
        assert "Returns:" in docstring
        assert "ApiResponse[Paginated]" in docstring
    
    def test_deprecated_method_warning_sync(self):
        """Test deprecated method shows warning in sync client."""
        client = Client("http://test.example")
        
        with patch.object(client, '_make_request', return_value=Mock()) as mock_request:
            with warnings.catch_warnings(record=True) as w:
                warnings.simplefilter("always")
                client.pets_delete(PetsRemoveRequest(name="test"))
                
                # Check deprecation warning was raised
                assert len(w) == 1
                assert issubclass(w[0].category, DeprecationWarning)
                assert "pets_delete is deprecated" in str(w[0].message)


class TestClientMethodSignatures:
    """Test client method signatures are correct."""
    
    def test_pets_create_signature(self):
        """Test pets_create method signature."""
        client = AsyncClient("http://test.example")
        
        # Should accept Pet as data parameter
        import inspect
        sig = inspect.signature(client.pets_create)
        params = list(sig.parameters.keys())
        
        # Note: inspect.signature on bound methods doesn't include 'self'
        assert 'data' in params
    
    def test_pets_get_first_no_params(self):
        """Test pets_get_first has no data parameter."""
        client = AsyncClient("http://test.example")
        
        import inspect
        sig = inspect.signature(client.pets_get_first)
        params = list(sig.parameters.keys())
        
        # Should have no parameters (no data)
        assert params == []
    
    def test_health_check_no_params(self):
        """Test health_check has no data parameter."""
        client = Client("http://test.example")
        
        import inspect
        sig = inspect.signature(client.health_check)
        params = list(sig.parameters.keys())
        
        # Should have no parameters (no data)
        assert params == []


class TestClientBaseUrls:
    """Test client base URL handling."""
    
    def test_async_client_base_url_storage(self):
        """Test AsyncClient stores base URL correctly."""
        client = AsyncClient("https://api.example.com/v1")
        assert client.base_url == "https://api.example.com/v1"
    
    def test_sync_client_base_url_storage(self):
        """Test sync Client stores base URL correctly."""
        client = Client("https://api.example.com/v1")
        assert client.base_url == "https://api.example.com/v1"


class TestClientInheritance:
    """Test client inheritance from base classes."""
    
    def test_async_client_inheritance(self):
        """Test AsyncClient inherits from AsyncClientBase."""
        from reflectapi_runtime import AsyncClientBase
        client = AsyncClient("http://test.example")
        assert isinstance(client, AsyncClientBase)
    
    def test_sync_client_inheritance(self):
        """Test Client inherits from ClientBase."""
        from reflectapi_runtime import ClientBase
        client = Client("http://test.example")
        assert isinstance(client, ClientBase)