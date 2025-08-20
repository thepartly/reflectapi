# TypeScript Client

The TypeScript client provides full type safety, modern async/await patterns, and excellent IDE support for consuming ReflectAPI services.

## Features

- **Full TypeScript type definitions** with compile-time checking
- **Async/await Promise-based API** for modern JavaScript
- **Automatic JSON serialization/deserialization** 
- **Built-in error handling** with HTTP status codes
- **Tree-shakable ES modules** for minimal bundle size
- **Native browser and Node.js support**

## Generation

Generate a TypeScript client from your ReflectAPI schema:

```bash
reflectapi-cli codegen --language typescript --schema api-schema.json --output clients/typescript
```

This creates a complete npm package with:
- Type definitions for all your API types
- Client classes for making API calls
- Error types and handling
- Package.json with dependencies

## Installation

Install the generated client in your project:

```bash
cd clients/typescript
npm install

# In your application
npm install ./path/to/clients/typescript
```

## Basic Usage

```typescript
import { ApiClient } from './clients/typescript';

const client = new ApiClient('https://api.example.com');

// Type-safe API calls
const user = await client.users.get(123);
const newUser = await client.users.create({
  name: 'Alice',
  email: 'alice@example.com'
});
```

## Configuration

### Custom Headers and Timeouts

```typescript
const client = new ApiClient('https://api.example.com', {
  headers: { 
    'Authorization': 'Bearer your-token',
    'X-API-Version': '1.0'
  },
  timeout: 30000  // 30 seconds
});
```

### Custom Fetch Implementation

```typescript
// Use with a custom fetch (e.g., node-fetch in Node.js)
import fetch from 'node-fetch';

const client = new ApiClient('https://api.example.com', {
  fetch: fetch as any
});
```

## Error Handling

The TypeScript client provides structured error handling:

```typescript
interface ApiError {
  status: number;
  message: string;
  data?: any;
}

try {
  const user = await client.users.get(999);
} catch (error) {
  if (error.status === 404) {
    console.log('User not found');
  } else if (error.status >= 500) {
    console.log('Server error:', error.message);
  }
}
```

### Handling Different Error Types

```typescript
try {
  const result = await client.users.create(userData);
} catch (error) {
  if (error.status === 400) {
    // Validation errors
    console.log('Validation failed:', error.data?.validation_errors);
  } else if (error.status === 401) {
    // Authentication required
    console.log('Please log in');
  } else if (error.status === 403) {
    // Permission denied
    console.log('Access denied');
  } else {
    // Network or other errors
    console.log('Request failed:', error.message);
  }
}
```

## Type Safety

The generated TypeScript client provides full compile-time type checking:

```typescript
// Compile-time type checking
interface User {
  id: number;
  name: string;
  email: string;
  created_at: string;
}

interface CreateUserRequest {
  name: string;
  email: string;
}

// TypeScript will enforce these types
const user: User = await client.users.get(123);
const newUser: User = await client.users.create({
  name: 'Alice',      // ✅ Required field
  email: 'alice@example.com'  // ✅ Required field
  // Missing fields will cause compile errors
});

// Invalid types cause compile errors
await client.users.create({
  name: 123,  // ❌ Type error: Expected string
  email: 'alice@example.com'
});
```

## Advanced Usage

### Request Interceptors

```typescript
class AuthenticatedClient extends ApiClient {
  constructor(baseUrl: string, private token: string) {
    super(baseUrl, {
      headers: {
        'Authorization': `Bearer ${token}`
      }
    });
  }

  // Override request method for custom logic
  protected async request<T>(
    method: string, 
    path: string, 
    data?: any
  ): Promise<T> {
    try {
      return await super.request(method, path, data);
    } catch (error) {
      if (error.status === 401) {
        // Handle token refresh
        await this.refreshToken();
        return await super.request(method, path, data);
      }
      throw error;
    }
  }

  private async refreshToken() {
    // Token refresh logic
  }
}
```

### Batching Requests

```typescript
// Concurrent requests with Promise.all
const [user, posts, comments] = await Promise.all([
  client.users.get(123),
  client.posts.list({ user_id: 123 }),
  client.comments.list({ user_id: 123 })
]);

// Sequential requests with error handling
async function getUserData(userId: number) {
  try {
    const user = await client.users.get(userId);
    const posts = await client.posts.list({ user_id: userId });
    
    return { user, posts };
  } catch (error) {
    console.error('Failed to load user data:', error);
    throw error;
  }
}
```

## Framework Integration

### React Integration

```typescript
import React, { useEffect, useState } from 'react';
import { ApiClient } from './clients/typescript';

const client = new ApiClient(process.env.REACT_APP_API_URL);

function UserProfile({ userId }: { userId: number }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    client.users.get(userId)
      .then(setUser)
      .catch(err => setError(err.message))
      .finally(() => setLoading(false));
  }, [userId]);

  if (loading) return <div>Loading...</div>;
  if (error) return <div>Error: {error}</div>;
  if (!user) return <div>User not found</div>;

  return (
    <div>
      <h1>{user.name}</h1>
      <p>{user.email}</p>
    </div>
  );
}
```

### Next.js API Route

```typescript
// pages/api/users/[id].ts
import { NextApiRequest, NextApiResponse } from 'next';
import { ApiClient } from '../../../clients/typescript';

const client = new ApiClient(process.env.INTERNAL_API_URL);

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse
) {
  const { id } = req.query;

  try {
    const user = await client.users.get(Number(id));
    res.status(200).json(user);
  } catch (error) {
    res.status(error.status || 500).json({ 
      error: error.message 
    });
  }
}
```

### Vue.js Composition API

```typescript
import { ref, onMounted } from 'vue';
import { ApiClient } from './clients/typescript';

export function useUser(userId: number) {
  const user = ref<User | null>(null);
  const loading = ref(true);
  const error = ref<string | null>(null);

  const client = new ApiClient(import.meta.env.VITE_API_URL);

  onMounted(async () => {
    try {
      user.value = await client.users.get(userId);
    } catch (err) {
      error.value = err.message;
    } finally {
      loading.value = false;
    }
  });

  return { user, loading, error };
}
```

## Testing

### Unit Testing with Jest

```typescript
import { ApiClient } from './clients/typescript';

// Mock the client for testing
jest.mock('./clients/typescript');

describe('UserService', () => {
  const mockClient = new ApiClient('http://test') as jest.Mocked<ApiClient>;

  beforeEach(() => {
    jest.clearAllMocks();
  });

  test('should get user by id', async () => {
    const mockUser = { id: 123, name: 'Alice', email: 'alice@example.com' };
    mockClient.users.get.mockResolvedValue(mockUser);

    const user = await mockClient.users.get(123);

    expect(user).toEqual(mockUser);
    expect(mockClient.users.get).toHaveBeenCalledWith(123);
  });

  test('should handle errors', async () => {
    const mockError = { status: 404, message: 'User not found' };
    mockClient.users.get.mockRejectedValue(mockError);

    await expect(mockClient.users.get(999)).rejects.toEqual(mockError);
  });
});
```

### Integration Testing with Playwright

```typescript
import { test, expect } from '@playwright/test';
import { ApiClient } from './clients/typescript';

test.describe('API Integration', () => {
  let client: ApiClient;

  test.beforeEach(() => {
    client = new ApiClient('http://localhost:3000');
  });

  test('should create and retrieve user', async () => {
    // Create user
    const createRequest = {
      name: 'Test User',
      email: 'test@example.com'
    };
    
    const createdUser = await client.users.create(createRequest);
    expect(createdUser.name).toBe(createRequest.name);
    expect(createdUser.email).toBe(createRequest.email);

    // Retrieve user
    const retrievedUser = await client.users.get(createdUser.id);
    expect(retrievedUser).toEqual(createdUser);
  });

  test('should handle not found errors', async () => {
    try {
      await client.users.get(999999);
      expect.fail('Should have thrown an error');
    } catch (error) {
      expect(error.status).toBe(404);
    }
  });
});
```

## Performance Optimization

### Bundle Size Optimization

The TypeScript client is designed for tree-shaking:

```typescript
// Only import what you need
import { UsersApi } from './clients/typescript/users';
import { PostsApi } from './clients/typescript/posts';

// Instead of importing the entire client
// import { ApiClient } from './clients/typescript';
```

### Caching Strategies

```typescript
class CachedClient extends ApiClient {
  private cache = new Map<string, { data: any; expires: number }>();

  protected async request<T>(
    method: string, 
    path: string, 
    data?: any
  ): Promise<T> {
    if (method === 'GET') {
      const cacheKey = `${method}:${path}`;
      const cached = this.cache.get(cacheKey);
      
      if (cached && cached.expires > Date.now()) {
        return cached.data;
      }
    }

    const result = await super.request<T>(method, path, data);

    if (method === 'GET') {
      this.cache.set(`${method}:${path}`, {
        data: result,
        expires: Date.now() + 60000  // 1 minute cache
      });
    }

    return result;
  }
}
```

## Troubleshooting

### Common Issues

**Type errors during compilation:**
- Ensure the schema is up-to-date and regenerate the client
- Check that all required dependencies are installed

**Network errors:**
- Verify the base URL is correct
- Check CORS configuration for browser requests
- Ensure the server is running and accessible

**Authentication errors:**
- Verify API keys or tokens are correct
- Check that headers are properly set

**Build errors:**
- Ensure TypeScript version compatibility
- Check that all peer dependencies are installed

### Debug Mode

Enable debug logging to troubleshoot issues:

```typescript
const client = new ApiClient('https://api.example.com', {
  debug: true  // Logs all requests and responses
});
```

## Best Practices

1. **Type Safety**: Always use the generated types, never `any`
2. **Error Handling**: Always wrap API calls in try-catch blocks
3. **Configuration**: Centralize client configuration
4. **Testing**: Mock the client for unit tests, use real API for integration tests
5. **Caching**: Implement appropriate caching strategies for read-heavy operations
6. **Authentication**: Use secure token storage and refresh mechanisms

## Next Steps

- [Python Client Guide](./python.md) - Compare with Python implementation
- [Rust Client Guide](./rust.md) - Compare with Rust implementation
- [Client Comparison](./README.md#client-comparison) - Feature comparison across languages