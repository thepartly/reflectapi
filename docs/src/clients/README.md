# Client Generation

ReflectAPI automatically generates type-safe client libraries for multiple programming languages from your Rust API definition. This section covers how to generate and use clients in different languages.

## Supported Languages

| Language   | Status | Features |
|------------|--------|----------|
| TypeScript | ✅ Stable | Full type safety, async/await, error handling |
| Python     | ✅ Stable | Pydantic models, async httpx, type hints |
| Rust       | ✅ Stable | Compile-time types, reqwest, comprehensive errors |

## Code Generation Workflow

1. **Define your API** using ReflectAPI traits and builder
2. **Generate schema** as JSON from your Rust application
3. **Run the CLI** to generate client libraries
4. **Use the clients** in your applications with full type safety

```bash
# Generate schema (from your Rust app)
cargo run  # Your app should save reflectapi-schema.json

# Generate clients
reflectapi-cli codegen --language typescript --schema reflectapi-schema.json --output clients/typescript
reflectapi-cli codegen --language python --schema reflectapi-schema.json --output clients/python
reflectapi-cli codegen --language rust --schema reflectapi-schema.json --output clients/rust
```

## Common Features

All generated clients share these characteristics:

### Type Safety
- Native type definitions for each language
- Compile-time or runtime type checking
- IDE support with autocompletion

### Error Handling
- Structured error types
- HTTP status code information
- Network error handling

### Async Support
- Modern async/await patterns
- Proper resource cleanup
- Configurable timeouts

### Documentation
- Generated from your Rust documentation
- Type information in IDE tooltips
- Usage examples

## Client Comparison

| Feature | TypeScript | Python | Rust |
|---------|------------|--------|------|
| Type Safety | Compile-time | Runtime | Compile-time |
| Error Types | Custom interfaces | Exception classes | Result types |
| HTTP Client | fetch/axios | httpx | reqwest |
| Async Model | Promise/async | async/await | Future/async |
| Package Format | npm package | Python package | Cargo crate |

## Next Steps

- [TypeScript Client Guide](./typescript.md)
- [Python Client Guide](./python.md)  
- [Rust Client Guide](./rust.md)