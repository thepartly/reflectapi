# ReflectAPI Python Runtime

Python runtime library for ReflectAPI-generated clients.

## Features

- **ReflectapiOption support**: Proper three-state handling (Undefined/None/Some) with Pydantic integration
- **Sync and async clients** with context manager support and proper resource cleanup
- **Middleware system** for request/response transformation, logging, and retry logic
- **Batch operations** for efficient concurrent requests with semaphore-based control
- **Testing utilities** including mock clients and response factories

## Installation

_Package name on PyPi to be confirmed_

```bash
# With pip
pip install reflectapi-python-runtime

# With uv (recommended for development)
uv add reflectapi-python-runtime
```


## Usage

This package is typically used as a dependency for generated ReflectAPI Python clients. While you don't usually import 
it directly, understanding its components helps with advanced usage scenarios.

### Basic Client Usage

```python
from my_generated_api import AsyncMyApiClient, MyApiClient
from my_generated_api.types import CreateUserRequest

# Async client (recommended)
async with AsyncMyApiClient("https://api.example.com") as client:
    # Create user with proper types
    request = CreateUserRequest(name="Alice", email="alice@example.com")
    user = await client.create_user(request)
    
    print(f"User: {user.name}")  # Direct attribute access
    print(f"Status: {user.metadata.status_code}")  # Transport metadata
    print(f"Response time: {user.metadata.elapsed_time}ms")

# Sync client
with MyApiClient("https://api.example.com") as client:
    user = client.create_user(request)
    print(f"Created: {user.name}")
```

### Advanced Features

**ReflectapiOption for optional fields:**
```python
from reflectapi_runtime import ReflectapiOption
from my_generated_api.types import UpdateUserRequest

# Three-state optional fields
request = UpdateUserRequest(
    user_id=123,
    name=ReflectapiOption("New Name"),  # Update name
    email=ReflectapiOption(None),       # Set email to null
    bio=ReflectapiOption()              # Leave bio unchanged (undefined)
)

user = await client.update_user(request)
```


### Middleware

```python
from reflectapi_runtime.middleware import LoggingMiddleware, RetryMiddleware

middleware = [
    LoggingMiddleware("my_app.api"),
    RetryMiddleware(max_retries=3, backoff_factor=2.0)
]

async with AsyncMyApiClient("https://api.example.com", middleware=middleware) as client:
    user = await client.get_user(123)
```

### Testing

```python
from my_generated_api.testing import MockClient, create_user_response
from my_generated_api.types import User

# Mock responses for testing
mock_client = MockClient()
mock_client.set_async_response(
    "get_user", 
    create_user_response(User(id=1, name="Test User", email="test@example.com"))
)

# Use in tests
user = await mock_client.get_user(123)
assert user.name == "Test User"
assert user.metadata.status_code == 200

# Test error scenarios
mock_client.set_async_error(
    "get_user",
    ApplicationError("User not found", status_code=404)
)

with pytest.raises(ApplicationError) as exc_info:
    await mock_client.get_user(999)
assert exc_info.value.status_code == 404
```

### Batch Operations

```python
from reflectapi_runtime.batch import BatchClient

# Process multiple requests concurrently
async with BatchClient(client, max_concurrency=10) as batch:
    results = await batch.execute([
        ("get_user", {"user_id": 1}),
        ("get_user", {"user_id": 2}),
        ("get_user", {"user_id": 3}),
    ])
    
    for result in results:
        if isinstance(result, User):
            print(f"User: {result.name}")
        else:  # Error case
            print(f"Error: {result.message}")
```

## Development

### Requirements

- Python 3.12+
- `uv` for dependency management

### Setup

```bash
cd reflectapi-python-runtime
uv sync
```

### Testing

```bash
# Run all tests with coverage
uv run pytest tests/ -xvs --cov=src/reflectapi_runtime

# Run specific test modules
uv run pytest tests/test_client.py -xvs
```

### Code Quality

```bash
# Format and lint code
uv run ruff format src/ tests/
uv run ruff check src/ tests/

# Type checking
uv run ty check
```

### Architecture

The runtime is built around several core components:

- **`client.py`**: Base client classes with refactored helper methods
- **`option.py`**: ReflectapiOption with Pydantic V2 integration
- **`auth.py`**: Comprehensive authentication system
- **`middleware.py`**: Request/response middleware chains
- **`exceptions.py`**: Structured error hierarchy
- **`response.py`**: ApiResponse wrapper with metadata
- **`batch.py`**: Concurrent batch operations
- **`testing.py`**: Testing utilities and mocks
