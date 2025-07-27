"""Test the generated testing utilities."""

import pytest
from generated import (
    Pet, PetDetails, PetKind, Behavior, Paginated,
    create_pet_response, create_petdetails_response, 
    create_paginated_response, create_mock_client
)
from reflectapi_runtime.testing import MockClient
from reflectapi_runtime import ApiResponse


class TestMockResponseCreation:
    """Test mock response creation utilities."""
    
    def test_create_pet_response(self):
        """Test creating mock Pet response."""
        pet = Pet(name="test", kind=PetKind.CAT)
        response = create_pet_response(pet)
        
        assert isinstance(response, ApiResponse)
        assert response.value == pet
    
    def test_create_petdetails_response(self):
        """Test creating mock PetDetails response."""
        from datetime import datetime
        pet_details = PetDetails(
            name="test", 
            kind=PetKind.DOG, 
            updated_at=datetime.now()
        )
        response = create_petdetails_response(pet_details)
        
        assert isinstance(response, ApiResponse)
        assert response.value == pet_details
    
    def test_create_paginated_response(self):
        """Test creating mock Paginated response."""
        from datetime import datetime
        pets = [
            PetDetails(name="pet1", kind=PetKind.CAT, updated_at=datetime.now())
        ]
        paginated = Paginated[PetDetails](items=pets)
        response = create_paginated_response(paginated)
        
        assert isinstance(response, ApiResponse)
        assert response.value == paginated
        assert len(response.value.items) == 1


class TestMockClient:
    """Test MockClient functionality."""
    
    def test_create_mock_client(self):
        """Test creating mock client."""
        mock_client = create_mock_client()
        assert isinstance(mock_client, MockClient)
    
    def test_mock_client_methods(self):
        """Test mock client has necessary methods."""
        mock_client = create_mock_client()
        
        # MockClient should have methods for recording/replaying
        assert hasattr(mock_client, 'record')
        assert hasattr(mock_client, 'replay')
        assert hasattr(mock_client, 'add_response')


class TestResponseMetadata:
    """Test response metadata functionality."""
    
    def test_response_has_metadata(self):
        """Test that created responses have metadata."""
        pet = Pet(name="test", kind=PetKind.CAT)
        response = create_pet_response(pet)
        
        assert hasattr(response, 'metadata')
        assert response.metadata is not None
    
    def test_response_metadata_properties(self):
        """Test response metadata has expected properties."""
        pet = Pet(name="test", kind=PetKind.CAT)
        response = create_pet_response(pet)
        
        # Should have basic metadata properties
        assert hasattr(response.metadata, 'status_code')
        assert hasattr(response.metadata, 'headers')


class TestTestingIntegration:
    """Test integration between testing utilities and models."""
    
    def test_all_model_response_creators_exist(self):
        """Test that response creators exist for all main models."""
        from generated import (
            create_behavior_response,
            create_petkind_response,
            create_headers_response,
            create_petslistrequest_response,
            create_petsremoverequest_response,
            create_petsupdaterequest_response
        )
        
        # Test that all functions exist and are callable
        assert callable(create_behavior_response)
        assert callable(create_petkind_response)
        assert callable(create_headers_response)
        assert callable(create_petslistrequest_response)
        assert callable(create_petsremoverequest_response)
        assert callable(create_petsupdaterequest_response)
    
    def test_response_creators_work(self):
        """Test that response creators actually work."""
        from generated import (
            Headers, PetsListRequest,
            create_headers_response, create_petslistrequest_response
        )
        
        # Test Headers response creator
        headers = Headers(authorization="Bearer test")
        headers_response = create_headers_response(headers)
        assert isinstance(headers_response, ApiResponse)
        assert headers_response.value == headers
        
        # Test PetsListRequest response creator
        list_request = PetsListRequest(limit=10)
        list_response = create_petslistrequest_response(list_request)
        assert isinstance(list_response, ApiResponse)
        assert list_response.value == list_request