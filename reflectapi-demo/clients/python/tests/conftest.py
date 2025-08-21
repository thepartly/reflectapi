"""Shared test fixtures and configuration for Python demo client tests."""

# Centralized gating for e2e tests that require the running demo server.
# If RUN_DEMO_E2E is not set to "1", skip tests explicitly marked `e2e`
# and tests in the client-server e2e module.
import os
import pytest

_RUN_DEMO_E2E = os.environ.get("RUN_DEMO_E2E") == "1"
_SKIP_E2E_REASON = "Requires running demo server at http://localhost:3000; set RUN_DEMO_E2E=1 to enable"

def pytest_collection_modifyitems(config, items):
    if _RUN_DEMO_E2E:
        return
    for item in items:
        is_e2e_marked = "e2e" in item.keywords
        is_server_e2e_file = str(getattr(item, "fspath", "")).endswith("tests/integration/test_client_server_e2e.py")
        if is_e2e_marked or is_server_e2e_file:
            item.add_marker(pytest.mark.skip(reason=_SKIP_E2E_REASON))

from typing import Any

# No local sys.path manipulation; runtime is installed via uv
from reflectapi_runtime import ReflectapiOption
from generated import (
    OutputPet as PetDetails,
    OutputKindDogVariant as DogVariant,
    OutputBehavior as Behavior,
    OutputBehaviorAggressiveVariant as BehaviorAggressive,
    PetsUpdateRequest,
    Paginated,
    ApiClient as AsyncClient
)

# Test configuration
# pytest_plugins = ["pytest_asyncio"]  # Uncomment if using async tests


@pytest.fixture
def sample_dog() -> DogVariant:
    """Create a sample dog variant."""
    return DogVariant(breed='Golden Retriever')


@pytest.fixture  
def sample_update_request() -> PetsUpdateRequest:
    """Create a sample PetsUpdateRequest with ReflectapiOption fields."""
    return PetsUpdateRequest(
        name="TestPet"
        # Other fields are ReflectapiOption and default to None
    )


@pytest.fixture
def undefined_update_request() -> PetsUpdateRequest:
    """Create a PetsUpdateRequest with undefined fields."""
    return PetsUpdateRequest(
        name="TestPet",
        # All optional fields left undefined
    )


@pytest.fixture
def mock_async_client() -> AsyncClient:
    """Create a mock AsyncClient for testing."""
    return AsyncClient("https://api.example.com")


# Marks for test categorization
pytest.mark.unit = pytest.mark.unit
pytest.mark.integration = pytest.mark.integration
pytest.mark.e2e = pytest.mark.e2e
pytest.mark.slow = pytest.mark.slow


# Test helpers
def assert_reflectapi_option_some(option: ReflectapiOption, expected_value: Any) -> None:
    """Assert that a ReflectapiOption contains the expected value."""
    assert option.is_some
    assert not option.is_undefined
    assert not option.is_none
    assert option.unwrap() == expected_value


def assert_reflectapi_option_undefined(option: ReflectapiOption) -> None:
    """Assert that a ReflectapiOption is undefined."""
    assert option.is_undefined
    assert not option.is_some
    assert not option.is_none


def assert_reflectapi_option_none(option: ReflectapiOption) -> None:
    """Assert that a ReflectapiOption is explicitly None."""
    assert option.is_none
    assert not option.is_undefined
    assert not option.is_some
    assert option.value is None
