# Performance Optimization

Learn how to optimize ReflectAPI applications for maximum performance.

## Schema Generation Performance

### Compile-Time Optimization

ReflectAPI uses derive macros to generate schemas at compile time, avoiding runtime overhead:

```rust,ignore
// This generates zero runtime code
#[derive(reflectapi::Input)]
struct UserRequest {
    name: String,
    email: String,
}

// Schema is built once at startup
let builder = Builder::new()
    .name("My API")
    .input::<UserRequest>()
    .output::<UserResponse>();
```

### Reducing Build Times

For large APIs with many types, consider organizing schemas:

```rust,ignore
// Split large schemas into modules
mod user_schema {
    use super::*;
    
    pub fn build_user_routes(builder: Builder) -> Builder {
        builder
            .input::<CreateUserRequest>()
            .input::<UpdateUserRequest>()
            .output::<User>()
            .output::<UserList>()
    }
}

// Combine in main builder
let builder = Builder::new()
    .name("My API");
let builder = user_schema::build_user_routes(builder);
let builder = order_schema::build_order_routes(builder);
```

## Runtime Performance

### HTTP Client Optimization

#### Connection Reuse

Generated clients automatically use connection pooling:

```typescript
// TypeScript - automatically pools connections
const client = new ApiClient('https://api.example.com', {
  // Reuses HTTP/2 connections
  keepAlive: true,
  maxConnections: 10
});
```

```python
# Python - configure httpx for optimal performance
import asyncio
import httpx

async with httpx.AsyncClient(
    limits=httpx.Limits(max_keepalive_connections=10, max_connections=100),
    http2=True
) as http_client:
    client = AsyncClient(
        base_url='https://api.example.com',
        http_client=http_client
    )
```

```rust,ignore
// Rust - reqwest automatically optimizes connections
let client = ClientBuilder::new("https://api.example.com")
    .pool_max_idle_per_host(10)
    .http2_prior_knowledge()
    .build()?;
```

#### Batch Operations

For multiple requests, use concurrent patterns:

```typescript
// TypeScript - concurrent requests
const userIds = [1, 2, 3, 4, 5];
const users = await Promise.all(
  userIds.map(id => client.users.get(id))
);
```

```python
# Python - asyncio.gather for concurrency
user_ids = [1, 2, 3, 4, 5]
users = await asyncio.gather(*[
    client.users.get(user_id) for user_id in user_ids
])
```

```rust,ignore
// Rust - futures::future::join_all
use futures::future::join_all;

let user_ids = vec![1, 2, 3, 4, 5];
let futures = user_ids.into_iter().map(|id| client.users().get(id));
let users = join_all(futures).await;
```

### Serialization Performance

#### Zero-Copy Deserialization

Use `&str` and `&[u8]` for borrowed data when possible:

```rust,ignore
#[derive(reflectapi::Input)]
struct SearchRequest<'a> {
    query: &'a str,        // Borrows from input
    filters: &'a [&'a str], // Borrows array of borrowed strings
}
```

#### Streaming for Large Data

For large responses, consider streaming:

```rust,ignore
// Server side - stream large datasets
async fn get_large_dataset() -> impl Stream<Item = DataPoint> {
    // Return async stream instead of collecting all data
    futures::stream::iter(database_results)
}
```

## Memory Optimization

### Client-Side Caching

Implement intelligent caching for frequently accessed data:

```typescript
// TypeScript - simple caching wrapper
class CachedApiClient {
  private cache = new Map<string, { data: any, expiry: number }>();
  
  async getUser(id: number): Promise<User> {
    const key = `user:${id}`;
    const cached = this.cache.get(key);
    
    if (cached && cached.expiry > Date.now()) {
      return cached.data;
    }
    
    const user = await this.client.users.get(id);
    this.cache.set(key, {
      data: user,
      expiry: Date.now() + 300_000 // 5 minutes
    });
    
    return user;
  }
}
```

### Schema Size Optimization

Keep generated schemas minimal:

```rust,ignore
// Prefer specific types over generic catch-alls
#[derive(reflectapi::Output)]
struct UserSummary {
    id: u32,
    name: String,
    // Only include necessary fields
}

// Instead of always returning full User objects
#[derive(reflectapi::Output)]
struct FullUser {
    id: u32,
    name: String,
    email: String,
    profile: UserProfile,
    settings: UserSettings,
    // ... many more fields
}
```

## Network Optimization

### Compression

Enable compression for large payloads:

```rust,ignore
// Server-side compression with axum
use tower_http::compression::CompressionLayer;

let app = Router::new()
    .route("/api/users", get(get_users))
    .layer(CompressionLayer::new());
```

### Request/Response Size

#### Use Projections

Only request the fields you need:

```rust,ignore
// Define lightweight response types
#[derive(reflectapi::Output)]
struct UserListItem {
    id: u32,
    name: String,
    avatar_url: Option<String>,
}

// Instead of returning full User objects in lists
async fn list_users() -> Vec<UserListItem> {
    // Return minimal data for list views
}
```

#### Pagination

Always paginate large datasets:

```rust,ignore
#[derive(reflectapi::Input)]
struct ListUsersRequest {
    page: u32,
    limit: u32, // Max 100
    sort_by: Option<UserSortField>,
}

#[derive(reflectapi::Output)]
struct UserListResponse {
    users: Vec<UserListItem>,
    total_count: u32,
    page: u32,
    has_next: bool,
}
```

## Database Performance

### Query Optimization

Structure your API to enable efficient database queries:

```rust,ignore
// Design endpoints that map to efficient queries
async fn get_user_with_posts(user_id: u32) -> UserWithPosts {
    // Single query with JOIN instead of N+1 queries
    let result = sqlx::query_as!(
        UserWithPostsRow,
        "SELECT u.*, p.id as post_id, p.title as post_title 
         FROM users u 
         LEFT JOIN posts p ON u.id = p.user_id 
         WHERE u.id = ?",
        user_id
    ).fetch_all(&pool).await?;
    
    // Transform to nested structure
    transform_to_user_with_posts(result)
}
```

### Connection Pooling

Configure database connection pools appropriately:

```rust,ignore
// Configure SQLx pool for high concurrency
let pool = PgPoolOptions::new()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(3))
    .connect(&database_url)
    .await?;
```

## Monitoring and Profiling

### Performance Metrics

Add timing to your endpoints:

```rust,ignore
use std::time::Instant;

async fn timed_handler() -> Response {
    let start = Instant::now();
    
    // Handle request
    let result = process_request().await;
    
    let duration = start.elapsed();
    log::info!("Request completed in {:?}", duration);
    
    result
}
```

### Load Testing

Test your generated clients under load:

```bash
# Use wrk for HTTP benchmarking
wrk -t12 -c400 -d30s --script=test.lua http://localhost:3000/api/users

# Use Artillery for complex scenarios
artillery run load-test.yml
```

Example Artillery config:

```yaml
config:
  target: 'http://localhost:3000'
  phases:
    - duration: 60
      arrivalRate: 10
      name: "Warm up"
    - duration: 300
      arrivalRate: 50
      name: "Load test"

scenarios:
  - name: "User API flow"
    flow:
      - get:
          url: "/api/users"
      - post:
          url: "/api/users"
          json:
            name: "Test User"
            email: "test@example.com"
```

## Best Practices Summary

1. **Design for Performance**: Structure your API to enable efficient queries and caching
2. **Use Appropriate Types**: Prefer specific, minimal types over large generic ones
3. **Leverage Async**: Use async/await patterns in all generated clients
4. **Monitor Everything**: Add timing and metrics to identify bottlenecks
5. **Test Under Load**: Regularly test performance under realistic load conditions
6. **Cache Strategically**: Implement caching at appropriate layers (client, server, database)
7. **Optimize Serialization**: Use zero-copy patterns where possible

## Next Steps

- Learn about [Adding Middleware](./middleware.md) for cross-cutting performance concerns
- Explore [OpenAPI Integration](./openapi.md) for API documentation
- See [Troubleshooting](../reference/troubleshooting.md) for common performance issues