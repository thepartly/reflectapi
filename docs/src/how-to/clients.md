# Generating & Using Clients

Learn how to generate and use type-safe client libraries from your ReflectAPI definition.

## Quick Start

### 1. Generate Schema

First, your Rust application needs to export a schema. Add this to your `main.rs`:

```rust,ignore
use reflectapi::{Builder, Input, Output};

// Your API definitions...
let builder = Builder::new().name("My API");
let (schema, _) = builder.build()?;

// Save schema to file
let schema_json = serde_json::to_string_pretty(&schema)?;
std::fs::write("api-schema.json", schema_json)?;
```

### 2. Generate Clients

Use the CLI to generate clients for your target languages:

```bash
# TypeScript (most common)
reflectapi-cli codegen --language typescript --schema api-schema.json --output clients/typescript

# Python
reflectapi-cli codegen --language python --schema api-schema.json --output clients/python

# Rust
reflectapi-cli codegen --language rust --schema api-schema.json --output clients/rust
```

### 3. Use the Generated Client

Each language follows similar patterns but uses idiomatic conventions:

#### TypeScript Example

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

#### Python Example

```python
from clients.python import AsyncClient

async def main():
    client = AsyncClient(base_url='https://api.example.com')
    
    # Type-safe API calls with pydantic models
    user = await client.users.get(123)
    new_user = await client.users.create(CreateUserRequest(
        name='Alice',
        email='alice@example.com'
    ))
```

#### Rust Example

```rust,ignore
use clients::rust::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("https://api.example.com");
    
    // Compile-time type safety
    let user = client.users().get(123).await?;
    let new_user = client.users().create(CreateUserRequest {
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    }).await?;
    
    Ok(())
}
```

## Language-Specific Details

### TypeScript Client

**Features:**
- Full TypeScript type definitions
- Async/await with Promise-based API
- Automatic JSON serialization/deserialization
- Built-in error handling with status codes

**Installation:**
```bash
cd clients/typescript
npm install
```

**Advanced Usage:**
```typescript
// Custom headers and configuration
const client = new ApiClient('https://api.example.com', {
  headers: { 'Authorization': 'Bearer token' },
  timeout: 30000
});

// Error handling
try {
  const user = await client.users.get(999);
} catch (error) {
  if (error.status === 404) {
    console.log('User not found');
  }
}
```

### Python Client

**Features:**
- Pydantic models for request/response validation
- Async httpx-based HTTP client
- Type hints for IDE support
- Automatic retry and timeout handling

**Installation:**
```bash
pip install -r clients/python/requirements.txt
```

**Advanced Usage:**
```python
from clients.python import AsyncClient
from httpx import AsyncClient as HttpxClient

# Custom HTTP client configuration
async with HttpxClient(timeout=30.0) as http_client:
    client = AsyncClient(
        base_url='https://api.example.com',
        http_client=http_client
    )
    
    # Use client...
```

### Rust Client

**Features:**
- Zero-cost abstractions with compile-time types
- reqwest-based HTTP client
- Comprehensive error types
- Serde integration

**Installation:**
Add to your `Cargo.toml`:
```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
# Generated client dependencies are included
```

**Advanced Usage:**
```rust,ignore
use clients::rust::{Client, ClientBuilder};
use std::time::Duration;

// Custom client configuration
let client = ClientBuilder::new("https://api.example.com")
    .timeout(Duration::from_secs(30))
    .header("Authorization", "Bearer token")
    .build()?;
```

## Best Practices

### 1. Schema Versioning

Keep your schema files versioned to track API changes:

```bash
# Tag schemas with versions
cp api-schema.json schemas/v1.0.0.json
```

### 2. Client Updates

Regenerate clients when your API changes:

```bash
# Update schema
cargo run  # Regenerates api-schema.json

# Regenerate all clients
make generate-clients  # or your build script
```

### 3. Error Handling

All clients provide structured error information:

- **HTTP status codes** for REST semantics
- **Error messages** from the server
- **Network errors** for connectivity issues

### 4. Documentation

Generated clients include documentation from your Rust code:

```rust,ignore
/// Get a user by ID
/// 
/// Returns the user information including profile details.
async fn get_user(id: u32) -> Result<User, UserError> {
    // This documentation appears in generated clients
}
```

## Troubleshooting

**Schema generation fails**: Ensure your builder includes all types and routes.

**Client compilation errors**: Check that the schema is up-to-date and valid JSON.

**Runtime errors**: Verify the base URL and network connectivity.

**Type mismatches**: Regenerate clients after API changes.

## Next Steps

- Learn about [Working with Custom Types](./custom-types.md)
- Explore [Validation and Error Handling](./validation.md)
- See [Performance Optimization](./performance.md) for production use