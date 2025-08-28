# Python Client

The Python client provides runtime type validation with Pydantic, async/await support, and comprehensive error handling for consuming `reflectapi` services.

## Features

- **Pydantic models** for request/response validation and type coercion
- **Async httpx-based HTTP client** with connection pooling
- **Type hints** for full IDE support and mypy compatibility
- **Three-state option handling** with `reflectapi::Option<T>`
- **Automatic retry and timeout** handling
- **Comprehensive error types** with status code information

## Requirements

- **Python 3.12+** (uses modern union syntax `str | None`)
- **pydantic >= 2.0.0**
- **httpx >= 0.24.0**
- **reflectapi-runtime** (high-performance runtime library)

## Generation

Generate a Python client from your `reflectapi` schema:

```bash
reflectapi-cli codegen --language python --schema api-schema.json --output clients/python
```

This creates a complete Python package with:
- Pydantic models for all your API types
- Async and sync client implementations
- Request/response type definitions
- Requirements.txt with dependencies

## Installation

Install the generated client and its dependencies:

```bash
# Install the `reflectapi` Python runtime
pip install reflectapi-runtime

# Install the generated client
pip install -e ./clients/python

# Or install dependencies directly
pip install -r clients/python/requirements.txt
```

## Basic Usage

### Async Client (Recommended)

```python
import asyncio
from clients.python import AsyncClient
from clients.python.models import CreateUserRequest

async def main():
    client = AsyncClient(base_url='https://api.example.com')
    
    # Type-safe API calls with Pydantic models
    user = await client.users.get(123)
    print(f"User: {user.name} ({user.email})")
    
    # Create new user
    new_user = await client.users.create(CreateUserRequest(
        name='Alice',
        email='alice@example.com'
    ))
    print(f"Created user: {new_user.id}")

asyncio.run(main())
```

### Sync Client

```python
from clients.python import SyncClient
from clients.python.models import CreateUserRequest

client = SyncClient(base_url='https://api.example.com')

# Synchronous API calls
user = client.users.get(123)
new_user = client.users.create(CreateUserRequest(
    name='Alice',
    email='alice@example.com'
))
```

## Configuration

### Client Options

```python
from clients.python import AsyncClient
import httpx

# Custom configuration
client = AsyncClient(
    base_url='https://api.example.com',
    headers={'Authorization': 'Bearer your-token'},
    timeout=30.0,
    retries=3,
    retry_delay=1.0
)

# Using custom HTTP client
async with httpx.AsyncClient(
    timeout=30.0,
    verify=False,  # Disable SSL verification (not recommended)
    proxy="http://proxy.example.com:8080"
) as http_client:
    client = AsyncClient(
        base_url='https://api.example.com',
        http_client=http_client
    )
```

### Context Manager Support

```python
async def main():
    async with AsyncClient(base_url='https://api.example.com') as client:
        user = await client.users.get(123)
        # Client automatically closes when exiting context
```

## Error Handling

The Python client provides structured error handling with detailed exception types:

```python
from clients.python import AsyncClient, ApiError, NetworkError

async def main():
    client = AsyncClient(base_url='https://api.example.com')
    
    try:
        user = await client.users.get(999)
    except ApiError as e:
        if e.status_code == 404:
            print("User not found")
        elif e.status_code == 401:
            print("Authentication required")
        elif e.status_code >= 500:
            print(f"Server error: {e.message}")
        else:
            print(f"API error {e.status_code}: {e.message}")
    except NetworkError as e:
        print(f"Network error: {e}")
    except Exception as e:
        print(f"Unexpected error: {e}")
```

### Exception Hierarchy

```python
# Exception hierarchy
Exception
└── ReflectApiError
    ├── ApiError              # HTTP errors from the API
    │   ├── BadRequestError   # 400 errors
    │   ├── UnauthorizedError # 401 errors
    │   ├── ForbiddenError    # 403 errors
    │   ├── NotFoundError     # 404 errors
    │   └── ServerError       # 5xx errors
    ├── NetworkError          # Connection/timeout errors
    ├── ValidationError       # Request validation errors
    └── SerializationError    # JSON parsing errors
```

## Type Safety and Validation

### Pydantic Model Validation

```python
from clients.python.models import CreateUserRequest, User
from pydantic import ValidationError

# Valid request
try:
    request = CreateUserRequest(
        name='Alice',
        email='alice@example.com',
        age=25  # Optional field
    )
    user = await client.users.create(request)
    print(f"Created user: {user.name}")
except ValidationError as e:
    print(f"Validation error: {e}")

# Invalid request - triggers validation error
try:
    request = CreateUserRequest(
        name='',  # Invalid: empty name
        email='invalid-email'  # Invalid: not an email format
    )
except ValidationError as e:
    print(f"Validation failed: {e}")
    for error in e.errors():
        print(f"  {error['loc']}: {error['msg']}")
```

### Type Annotations

```python
from typing import List, Optional
from clients.python.models import User, UserListResponse

# Full type annotations for IDE support
async def get_all_users(client: AsyncClient) -> List[User]:
    response: UserListResponse = await client.users.list()
    return response.users

async def find_user_by_email(
    client: AsyncClient, 
    email: str
) -> Optional[User]:
    try:
        users = await get_all_users(client)
        return next((u for u in users if u.email == email), None)
    except ApiError:
        return None
```

## Advanced Usage

### Request Middleware

```python
from clients.python import AsyncClient
import time
import logging

class LoggingClient(AsyncClient):
    async def _make_request(self, method: str, path: str, **kwargs):
        start_time = time.time()
        logging.info(f"Starting {method} {path}")
        
        try:
            response = await super()._make_request(method, path, **kwargs)
            duration = time.time() - start_time
            logging.info(f"Completed {method} {path} in {duration:.3f}s")
            return response
        except Exception as e:
            duration = time.time() - start_time
            logging.error(f"Failed {method} {path} after {duration:.3f}s: {e}")
            raise

# Usage
async with LoggingClient(base_url='https://api.example.com') as client:
    user = await client.users.get(123)
```

### Batch Operations

```python
import asyncio
from typing import List

async def get_multiple_users(
    client: AsyncClient, 
    user_ids: List[int]
) -> List[User]:
    # Concurrent requests with semaphore for rate limiting
    semaphore = asyncio.Semaphore(10)  # Max 10 concurrent requests
    
    async def get_user(user_id: int) -> Optional[User]:
        async with semaphore:
            try:
                return await client.users.get(user_id)
            except NotFoundError:
                return None
    
    tasks = [get_user(user_id) for user_id in user_ids]
    results = await asyncio.gather(*tasks, return_exceptions=True)
    
    # Filter out None results and exceptions
    return [user for user in results 
            if isinstance(user, User)]

# Usage
users = await get_multiple_users(client, [1, 2, 3, 4, 5])
```

### Authentication Handling

```python
class AuthenticatedClient(AsyncClient):
    def __init__(self, base_url: str, token: str):
        super().__init__(
            base_url=base_url,
            headers={'Authorization': f'Bearer {token}'}
        )
        self._token = token
        self._refresh_token = None
    
    async def _make_request(self, method: str, path: str, **kwargs):
        try:
            return await super()._make_request(method, path, **kwargs)
        except UnauthorizedError:
            # Attempt token refresh
            if await self._refresh_auth_token():
                # Retry with new token
                return await super()._make_request(method, path, **kwargs)
            raise
    
    async def _refresh_auth_token(self) -> bool:
        # Token refresh logic
        try:
            # Call refresh endpoint
            response = await self._http_client.post(
                f"{self._base_url}/auth/refresh",
                json={'refresh_token': self._refresh_token}
            )
            data = response.json()
            
            # Update token
            self._token = data['access_token']
            self._headers['Authorization'] = f'Bearer {self._token}'
            return True
        except Exception:
            return False
```

## Three-State Option Handling

`reflectapi` supports three-state options (Undefined/None/Some) that map to Python naturally:

```python
from clients.python.models import UpdateUserRequest
from reflectapi_runtime import Option

# Three states:
# 1. Undefined (field not set) - won't be sent in request
# 2. None (explicit null) - will send null
# 3. Some(value) - will send the value

# Don't update the name (field omitted)
request = UpdateUserRequest(
    email='newemail@example.com'  # Only email is updated
)

# Set name to null explicitly
request = UpdateUserRequest(
    name=None,  # Explicitly set to null
    email='newemail@example.com'
)

# Set name to a value
request = UpdateUserRequest(
    name='New Name',  # Set to a value
    email='newemail@example.com'
)

# Using the Option type explicitly
request = UpdateUserRequest(
    name=Option.some('New Name'),  # Explicit Some
    email=Option.some('newemail@example.com')
)
```

## Data Science Integration

### Pandas Integration

```python
import pandas as pd
from clients.python import AsyncClient

async def load_users_dataframe(client: AsyncClient) -> pd.DataFrame:
    users = await client.users.list()
    
    # Convert Pydantic models to dict for pandas
    user_data = [user.dict() for user in users.users]
    df = pd.DataFrame(user_data)
    
    # Convert timestamps
    df['created_at'] = pd.to_datetime(df['created_at'])
    
    return df

# Usage
df = await load_users_dataframe(client)
print(df.head())
print(df.describe())
```

### Jupyter Notebook Support

```python
# Enable async in Jupyter notebooks
import nest_asyncio
nest_asyncio.apply()

from clients.python import AsyncClient

# Now you can use async/await in notebook cells
client = AsyncClient(base_url='https://api.example.com')
users = await client.users.list()

# Display results
for user in users.users[:5]:
    print(f"{user.name}: {user.email}")
```

## Testing

### Unit Testing with pytest

```python
import pytest
from unittest.mock import AsyncMock
from clients.python import AsyncClient
from clients.python.models import User

@pytest.fixture
def mock_client():
    client = AsyncClient(base_url='http://test')
    client._make_request = AsyncMock()
    return client

@pytest.mark.asyncio
async def test_get_user(mock_client):
    # Mock response
    mock_user = User(id=123, name='Alice', email='alice@example.com')
    mock_client._make_request.return_value = mock_user.dict()
    
    # Test
    user = await mock_client.users.get(123)
    
    # Assertions
    assert user.id == 123
    assert user.name == 'Alice'
    assert user.email == 'alice@example.com'
    mock_client._make_request.assert_called_once_with(
        'GET', '/users/123'
    )

@pytest.mark.asyncio
async def test_api_error_handling(mock_client):
    from clients.python import NotFoundError
    
    # Mock 404 error
    mock_client._make_request.side_effect = NotFoundError(
        "User not found", status_code=404
    )
    
    # Test error handling
    with pytest.raises(NotFoundError):
        await mock_client.users.get(999)
```

### Integration Testing

```python
import pytest
import httpx
from clients.python import AsyncClient

@pytest.mark.integration
class TestUserApi:
    @pytest.fixture
    async def client(self):
        async with AsyncClient(base_url='http://localhost:3000') as client:
            yield client
    
    @pytest.mark.asyncio
    async def test_user_crud(self, client):
        # Create user
        create_request = CreateUserRequest(
            name='Test User',
            email='test@example.com'
        )
        
        created_user = await client.users.create(create_request)
        assert created_user.name == 'Test User'
        assert created_user.email == 'test@example.com'
        assert created_user.id is not None
        
        # Get user
        retrieved_user = await client.users.get(created_user.id)
        assert retrieved_user.id == created_user.id
        assert retrieved_user.name == created_user.name
        
        # Update user
        update_request = UpdateUserRequest(name='Updated Name')
        updated_user = await client.users.update(
            created_user.id, 
            update_request
        )
        assert updated_user.name == 'Updated Name'
        
        # Delete user
        await client.users.delete(created_user.id)
        
        # Verify deletion
        with pytest.raises(NotFoundError):
            await client.users.get(created_user.id)
```

## Performance Optimization

### Connection Pooling

```python
import httpx
from clients.python import AsyncClient

# Configure connection pooling
limits = httpx.Limits(
    max_keepalive_connections=20,
    max_connections=100
)

timeout = httpx.Timeout(
    connect=5.0,  # Connection timeout
    read=30.0,    # Read timeout
    write=10.0,   # Write timeout
    pool=5.0      # Pool acquisition timeout
)

async with httpx.AsyncClient(
    limits=limits,
    timeout=timeout
) as http_client:
    client = AsyncClient(
        base_url='https://api.example.com',
        http_client=http_client
    )
    
    # All requests use the same connection pool
    users = await client.users.list()
```

### Caching Strategies

```python
from functools import wraps
from typing import Any, Dict
import time

class CachedClient(AsyncClient):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._cache: Dict[str, tuple[Any, float]] = {}
        self._cache_ttl = 300  # 5 minutes
    
    def _cache_key(self, method: str, path: str, params: dict = None) -> str:
        return f"{method}:{path}:{hash(str(params or {}))}"
    
    async def _make_request(self, method: str, path: str, **kwargs):
        if method == 'GET':
            cache_key = self._cache_key(method, path, kwargs.get('params'))
            
            # Check cache
            if cache_key in self._cache:
                cached_data, timestamp = self._cache[cache_key]
                if time.time() - timestamp < self._cache_ttl:
                    return cached_data
        
        # Make request
        result = await super()._make_request(method, path, **kwargs)
        
        # Cache GET requests
        if method == 'GET':
            cache_key = self._cache_key(method, path, kwargs.get('params'))
            self._cache[cache_key] = (result, time.time())
        
        return result
```

## Troubleshooting

### Common Issues

**Import errors:**
```bash
# Install the runtime library
pip install reflectapi-runtime

# Verify installation
python -c "import reflectapi_runtime; print('OK')"
```

**Type checking issues:**
```bash
# Install type checker dependencies
pip install mypy types-requests

# Run type checking
mypy clients/python/
```

**SSL/Certificate errors:**
```python
import ssl
import httpx

# Disable SSL verification (not recommended for production)
client = AsyncClient(
    base_url='https://api.example.com',
    http_client=httpx.AsyncClient(verify=False)
)

# Or provide custom CA bundle
client = AsyncClient(
    base_url='https://api.example.com',
    http_client=httpx.AsyncClient(verify='/path/to/ca-bundle.pem')
)
```

**Performance issues:**
```python
# Enable debug logging
import logging
logging.basicConfig(level=logging.DEBUG)

# Monitor request times
import time

start = time.time()
result = await client.users.list()
print(f"Request took {time.time() - start:.3f}s")
```

### Debug Mode

Enable comprehensive debugging:

```python
import logging
import httpx

# Enable debug logging
logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger('httpx')
logger.setLevel(logging.DEBUG)

# Create client with debug transport
client = AsyncClient(
    base_url='https://api.example.com',
    http_client=httpx.AsyncClient(
        event_hooks={
            'request': [lambda request: print(f"Request: {request}")],
            'response': [lambda response: print(f"Response: {response}")]
        }
    )
)
```

## Best Practices

1. **Use async clients** for better performance in I/O-heavy applications
2. **Context managers** for proper resource cleanup
3. **Type annotations** for better IDE support and maintainability
4. **Pydantic validation** - let models validate inputs automatically
5. **Error handling** - always handle specific exception types
6. **Connection pooling** for high-throughput applications
7. **Caching** for read-heavy workloads
8. **Testing** - use both unit tests and integration tests

## Next Steps

- [TypeScript Client Guide](./typescript.md) - Compare with TypeScript implementation
- [Rust Client Guide](./rust.md) - Compare with Rust implementation
- [Client Comparison](./README.md#client-comparison) - Feature comparison across languages