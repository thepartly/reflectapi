# Generating & Using Clients

Learn how to generate and use type-safe client libraries from your `reflectapi` definition.

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
cargo run --bin reflectapi codegen --language typescript --schema api-schema.json --output clients/typescript

# Python
cargo run --bin reflectapi codegen --language python --schema api-schema.json --output clients/python

# Rust
cargo run --bin reflectapi codegen --language rust --schema api-schema.json --output clients/rust
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

## Development Workflow

### Client Updates

Regenerate clients when your API changes:

```bash
# Update schema
cargo run  # Regenerates api-schema.json
```


You can use a `build.rs` script to automate client generation during server builds.


## Best Practices

### Schema Generation

- **Export schema in build**: Include schema generation in your build process
- **Version control schemas**: Commit generated schemas for reproducibility

### Client Distribution

- **Package clients**: Create proper packages (npm, PyPI, crates.io)
- **Version alignment**: Keep client versions aligned with API versions
- **Documentation**: Include usage examples in client packages

### Error Handling

All clients provide structured error information:

- **HTTP status codes** for REST semantics
- **Error messages** from the server
- **Network errors** for connectivity issues

## Language-Specific Guides

For detailed documentation on each client language:

- **[TypeScript Client](../clients/typescript.md)** - Full type safety, async/await, framework integration
- **[Python Client](../clients/python.md)** - Pydantic models, async httpx, data science integration  
- **[Rust Client](../clients/rust.md)** - Zero-cost abstractions, compile-time safety, CLI tools

## Next Steps

- Explore [Client Comparison](../clients/README.md#client-comparison) for feature differences
- Learn about [Working with Custom Types](./custom-types.md)
- See [Validation and Error Handling](./validation.md) for production patterns