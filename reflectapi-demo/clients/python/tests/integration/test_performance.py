"""Performance and stress tests for generated clients."""

import pytest
import time
import asyncio
from typing import List

from generated import (
    MyapiModelInputPet as Pet,
    MyapiModelKindDog as PetKindDog,
    MyapiModelKindCat as PetKindCat,
    MyapiModelBehavior as Behavior,
    MyapiModelBehaviorAggressiveVariant as BehaviorAggressive,
    AsyncClient
)
from reflectapi_runtime import ReflectapiOption

# For externally tagged enums, unit variants are just string literals
BehaviorCalm = "Calm"


@pytest.mark.slow
class TestPerformance:
    """Test performance characteristics of generated code."""

    def test_model_creation_performance(self):
        """Test performance of creating many model instances."""
        start_time = time.time()

        pets = []
        for i in range(1000):
            if i % 2 == 0:
                kind = PetKindDog(type='dog', breed=f'Breed_{i}')
            else:
                kind = PetKindCat(type='cat', lives=i % 9 + 1)

            pet = Pet(
                name=f'Pet_{i}',
                kind=kind,
                age=i % 20,
                behaviors=[BehaviorCalm, {"Aggressive": [1.0, "test"]}]
            )
            pets.append(pet)

        end_time = time.time()
        duration = end_time - start_time

        assert len(pets) == 1000
        assert duration < 1.0  # Should create 1000 pets in under 1 second
        print(f"Created 1000 pets in {duration:.3f} seconds ({1000/duration:.0f} pets/sec)")

    def test_serialization_performance(self):
        """Test performance of serializing many models."""
        # Create test data
        pets = []
        for i in range(100):
            kind = PetKindDog(type='dog', breed='Labrador') if i % 2 == 0 else PetKindCat(type='cat', lives=9)
            pets.append(Pet(name=f'Pet_{i}', kind=kind, age=i % 20))

        # Test JSON serialization performance
        start_time = time.time()
        for pet in pets:
            json_data = pet.model_dump_json()
            assert len(json_data) > 0
        end_time = time.time()

        duration = end_time - start_time
        assert duration < 0.1  # Should serialize 100 pets in under 0.1 seconds
        print(f"Serialized 100 pets in {duration:.3f} seconds")

    def test_deserialization_performance(self):
        """Test performance of deserializing many models."""
        # Create test JSON data
        json_data_list = []
        for i in range(100):
            if i % 2 == 0:
                json_data = f'{{"name": "Pet_{i}", "kind": {{"type": "dog", "breed": "Labrador"}}, "age": {i % 20}}}'
            else:
                json_data = f'{{"name": "Pet_{i}", "kind": {{"type": "cat", "lives": 9}}, "age": {i % 20}}}'
            json_data_list.append(json_data)

        # Test deserialization performance
        start_time = time.time()
        pets = []
        for json_data in json_data_list:
            pet = Pet.model_validate_json(json_data)
            pets.append(pet)
        end_time = time.time()

        duration = end_time - start_time
        assert len(pets) == 100
        assert duration < 0.1  # Should deserialize 100 pets in under 0.1 seconds
        print(f"Deserialized 100 pets in {duration:.3f} seconds")

    def test_reflectapi_option_performance(self):
        """Test performance of ReflectapiOption operations."""
        from reflectapi_runtime import ReflectapiOption, Undefined

        start_time = time.time()

        # Create many ReflectapiOption instances
        options = []
        for i in range(1000):
            if i % 3 == 0:
                option = ReflectapiOption(i)  # Some value
            elif i % 3 == 1:
                option = ReflectapiOption(None)  # Explicit None
            else:
                option = ReflectapiOption(Undefined)  # Undefined
            options.append(option)

        # Test operations
        some_count = sum(1 for opt in options if opt.is_some)
        none_count = sum(1 for opt in options if opt.is_none)
        undefined_count = sum(1 for opt in options if opt.is_undefined)

        end_time = time.time()
        duration = end_time - start_time

        assert some_count + none_count + undefined_count == 1000
        assert duration < 0.1  # Should handle 1000 operations in under 0.1 seconds
        print(f"Processed 1000 ReflectapiOption instances in {duration:.3f} seconds")


@pytest.mark.slow
class TestStress:
    """Stress tests for generated code."""

    def test_large_model_creation(self):
        """Test creating models with large amounts of data."""
        # Create a pet with large behavior list
        large_behaviors = [BehaviorCalm] * 500 + [{"Aggressive": [2.0, "perf"]}]

        pet = Pet(
            name="Large Pet",
            kind=PetKindDog(type='dog', breed='Large Breed'),
            behaviors=large_behaviors
        )

        assert len(pet.behaviors) == 501
        assert pet.name == "Large Pet"

    def test_deep_nesting_serialization(self):
        """Test serialization with deeply nested data structures."""
        from generated import MyapiProtoPaginated as Paginated

        # Create nested paginated responses
        pets = []
        for i in range(50):
            kind = PetKindCat(type='cat', lives=9) if i % 2 else PetKindDog(type='dog', breed='Breed')
            pets.append(Pet(name=f'Pet_{i}', kind=kind))

        paginated = Paginated[Pet](items=pets, cursor="test_cursor")

        # Should be able to serialize large nested structure
        json_data = paginated.model_dump_json()
        assert len(json_data) > 1000

        # Should be able to deserialize back
        reconstructed = Paginated[Pet].model_validate_json(json_data)
        assert len(reconstructed.items) == 50
        assert reconstructed.cursor == "test_cursor"

    def test_many_client_instances(self):
        """Test creating many client instances."""
        clients = []
        for i in range(100):
            client = AsyncClient(f"https://api{i}.example.com")
            clients.append(client)

        assert len(clients) == 100

        # All clients should be independent
        urls = {client.base_url for client in clients}
        assert len(urls) == 100


@pytest.mark.slow
class TestConcurrency:
    """Test concurrent operations with generated code."""

    @pytest.mark.asyncio
    async def test_concurrent_model_operations(self):
        """Test concurrent model creation and serialization."""

        async def create_and_serialize_pet(pet_id: int) -> str:
            """Create a pet and serialize it."""
            kind = PetKindDog(type='dog', breed=f'Breed_{pet_id}')
            pet = Pet(name=f'Pet_{pet_id}', kind=kind, age=pet_id % 20)
            return pet.model_dump_json()

        # Run many operations concurrently
        tasks = [create_and_serialize_pet(i) for i in range(10000)]
        start_time = time.time()
        results = await asyncio.gather(*tasks)
        end_time = time.time()

        assert len(results) == 10000
        assert all(len(result) > 0 for result in results)

        duration = end_time - start_time
        print(f"Concurrent creation and serialization of 100 pets took {duration:.3f} seconds")

    @pytest.mark.asyncio
    async def test_concurrent_validation(self):
        """Test concurrent validation of many models."""

        async def validate_pet_data(pet_data: dict) -> Pet:
            """Validate pet data."""
            return Pet.model_validate(pet_data)

        # Create test data
        test_data = []
        for i in range(10000):
            if i % 2 == 0:
                data = {
                    "name": f"Pet_{i}",
                    "kind": {"type": "dog", "breed": "Labrador"},
                    "age": i % 20
                }
            else:
                data = {
                    "name": f"Pet_{i}",
                    "kind": {"type": "cat", "lives": 9},
                    "age": i % 20
                }
            test_data.append(data)

        # Validate concurrently
        tasks = [validate_pet_data(data) for data in test_data]
        start_time = time.time()
        pets = await asyncio.gather(*tasks)
        end_time = time.time()

        assert len(pets) == 10000
        assert all(isinstance(pet, Pet) for pet in pets)

        duration = end_time - start_time
        print(f"Concurrent validation of 100 pets took {duration:.3f} seconds")


# Test configuration
@pytest.fixture(autouse=True)
def performance_test_config():
    """Configure performance test environment."""
    import gc

    # Run garbage collection before each test
    gc.collect()
    yield
    # Run garbage collection after each test
    gc.collect()
