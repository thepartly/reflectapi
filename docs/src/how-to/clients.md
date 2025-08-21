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

## Development Workflow

### 1. Schema Versioning

Keep your schema files versioned to track API changes:

```bash
# Tag schemas with versions
cp api-schema.json schemas/v1.0.0.json

# Or use git tags
git tag -a v1.0.0 -m "API v1.0.0"
```

### 2. Client Updates

Regenerate clients when your API changes:

```bash
# Update schema
cargo run  # Regenerates api-schema.json

# Regenerate all clients
make generate-clients  # or your build script
```

### 3. Automation

Set up automatic client generation in your CI/CD pipeline:

```yaml
# .github/workflows/generate-clients.yml
name: Generate Clients
on:
  push:
    branches: [main]
    paths: ['src/**/*.rs']

jobs:
  generate:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: Generate Schema
      run: cargo run --bin generate-schema
    - name: Generate TypeScript Client
      run: reflectapi-cli codegen --language typescript --schema api-schema.json --output clients/typescript
    - name: Generate Python Client  
      run: reflectapi-cli codegen --language python --schema api-schema.json --output clients/python
    - name: Generate Rust Client
      run: reflectapi-cli codegen --language rust --schema api-schema.json --output clients/rust
```

## Best Practices

### Schema Generation

- **Export schema in build**: Include schema generation in your build process
- **Version control schemas**: Commit generated schemas for reproducibility
- **Validate schemas**: Ensure schemas are valid before generating clients

### Client Distribution

- **Package clients**: Create proper packages (npm, PyPI, crates.io)
- **Version alignment**: Keep client versions aligned with API versions
- **Documentation**: Include usage examples in client packages

### Error Handling

All clients provide structured error information:

- **HTTP status codes** for REST semantics
- **Error messages** from the server
- **Network errors** for connectivity issues

### Testing

- **Unit tests**: Test client functionality with mocked responses
- **Integration tests**: Test against real API instances
- **Generated tests**: Include tests in generated client packages

## Language-Specific Guides

For detailed documentation on each client language:

- **[TypeScript Client](../clients/typescript.md)** - Full type safety, async/await, framework integration
- **[Python Client](../clients/python.md)** - Pydantic models, async httpx, data science integration  
- **[Rust Client](../clients/rust.md)** - Zero-cost abstractions, compile-time safety, CLI tools

## Troubleshooting

### Common Issues

**Schema generation fails:**
- Ensure your builder includes all types and routes
- Check that all types implement required traits

**Client compilation errors:**
- Verify the schema is up-to-date and valid JSON
- Check that all dependencies are properly installed

**Runtime errors:**
- Verify the base URL and network connectivity
- Check authentication credentials and headers

**Type mismatches:**
- Regenerate clients after API changes
- Ensure schema and client versions are aligned

### Debug Tips

1. **Validate schema**: Use `reflectapi-cli validate` to check schema correctness
2. **Test generation**: Generate clients in a temporary directory first
3. **Check logs**: Enable debug logging in generated clients
4. **Version control**: Use git to track changes between client generations

## Next Steps

- Explore [Client Comparison](../clients/README.md#client-comparison) for feature differences
- Learn about [Working with Custom Types](./custom-types.md)
- See [Validation and Error Handling](./validation.md) for production patterns