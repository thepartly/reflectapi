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
        is_server_e2e_file = str(getattr(item, "fspath", "")).endswith(
            "tests/integration/test_client_server_e2e.py"
        )
        if is_e2e_marked or is_server_e2e_file:
            item.add_marker(pytest.mark.skip(reason=_SKIP_E2E_REASON))


from typing import Any

# No local sys.path manipulation; runtime is installed via uv
from reflectapi_runtime import ReflectapiOption
from generated import (
    MyapiModelInputPet as Pet,
    MyapiModelOutputPet as PetDetails,
    MyapiModelKind as PetKind,
    MyapiModelKindDog as PetKindDog,
    MyapiModelKindCat as PetKindCat,
    MyapiModelBehavior as Behavior,
    MyapiModelBehaviorFactory as BehaviorFactory,
    MyapiProtoPetsUpdateRequest as PetsUpdateRequest,
    MyapiProtoPaginated as Paginated,
    AsyncClient,
)

# Test configuration
# pytest_plugins = ["pytest_asyncio"]  # Uncomment if using async tests


@pytest.fixture
def sample_dog() -> PetKindDog:
    """Create a sample dog variant."""
    return PetKindDog(type="dog", breed="Golden Retriever")


@pytest.fixture
def sample_cat() -> PetKindCat:
    """Create a sample cat variant."""
    return PetKindCat(type="cat", lives=9)


@pytest.fixture
def sample_pet(sample_dog: PetKindDog) -> Pet:
    """Create a sample Pet with dog variant."""
    return Pet(
        name="Buddy",
        kind=sample_dog,
        age=3,
        behaviors=[BehaviorFactory.CALM, BehaviorFactory.aggressive(5.0, "growls")],
    )


@pytest.fixture
def sample_pet_details(sample_cat: PetKindCat) -> PetDetails:
    """Create a sample PetDetails with cat variant."""
    from datetime import datetime

    return PetDetails(
        name="Whiskers",
        kind=sample_cat,
        age=2,
        updated_at=datetime.now(),
        behaviors=[BehaviorFactory.CALM],
    )


@pytest.fixture
def sample_update_request() -> PetsUpdateRequest:
    """Create a sample PetsUpdateRequest with ReflectapiOption fields."""
    return PetsUpdateRequest(
        name="TestPet",
        kind=PetKindDog(type="dog", breed="Labrador"),
        age=ReflectapiOption(5),
        behaviors=ReflectapiOption([BehaviorFactory.CALM]),
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


@pytest.fixture
def paginated_pets(sample_pet_details: PetDetails) -> Paginated[PetDetails]:
    """Create a sample paginated response."""
    return Paginated[PetDetails](items=[sample_pet_details], cursor="next_page_token")


# Test data collections
@pytest.fixture
def behavior_samples() -> list[Behavior]:
    """Sample behavior instances."""
    return [
        BehaviorFactory.CALM,
        BehaviorFactory.aggressive(5.0, "test"),
        BehaviorFactory.other("Custom", "Some notes"),
    ]


@pytest.fixture
def pet_kind_samples(sample_dog: PetKindDog, sample_cat: PetKindCat) -> list[PetKind]:
    """Sample PetKind union variants."""
    return [sample_dog, sample_cat]


# Test utilities
class TestDataFactory:
    """Factory for creating test data."""

    @staticmethod
    def create_pet(name: str = "TestPet", kind_type: str = "dog", **kwargs) -> Pet:
        """Create a Pet with specified parameters."""
        if kind_type == "dog":
            kind = PetKindDog(type="dog", breed=kwargs.get("breed", "Labrador"))
        else:
            kind = PetKindCat(type="cat", lives=kwargs.get("lives", 9))

        return Pet(
            name=name,
            kind=kind,
            age=kwargs.get("age"),
            behaviors=kwargs.get("behaviors"),
        )

    @staticmethod
    def create_update_request(
        name: str = "TestPet",
        with_age: bool = False,
        with_behaviors: bool = False,
        **kwargs,
    ) -> PetsUpdateRequest:
        """Create a PetsUpdateRequest with optional fields."""
        request = PetsUpdateRequest(name=name)

        if with_age:
            request.age = ReflectapiOption(kwargs.get("age", 5))
        if with_behaviors:
            request.behaviors = ReflectapiOption(
                kwargs.get("behaviors", [BehaviorFactory.CALM])
            )

        return request


@pytest.fixture
def test_factory() -> TestDataFactory:
    """Provide test data factory."""
    return TestDataFactory()


# Marks for test categorization
pytest.mark.unit = pytest.mark.unit
pytest.mark.integration = pytest.mark.integration
pytest.mark.e2e = pytest.mark.e2e
pytest.mark.slow = pytest.mark.slow


# Test helpers
def assert_petkind_dog(pet_kind: PetKind, expected_breed: str) -> None:
    """Assert that a PetKind is a dog with expected breed."""
    assert isinstance(pet_kind, PetKindDog)
    assert pet_kind.type == "dog"
    assert pet_kind.breed == expected_breed


def assert_petkind_cat(pet_kind: PetKind, expected_lives: int) -> None:
    """Assert that a PetKind is a cat with expected lives."""
    assert isinstance(pet_kind, PetKindCat)
    assert pet_kind.type == "cat"
    assert pet_kind.lives == expected_lives


def assert_reflectapi_option_some(
    option: ReflectapiOption, expected_value: Any
) -> None:
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


# Export test helpers for use in test modules
__all__ = [
    "assert_petkind_dog",
    "assert_petkind_cat",
    "assert_reflectapi_option_some",
    "assert_reflectapi_option_undefined",
    "assert_reflectapi_option_none",
    "TestDataFactory",
]
