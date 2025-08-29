# Client Generation

`reflectapi` automatically generates type-safe client libraries for multiple programming languages from your Rust API server. This section covers how to generate and use clients in different languages.

## Supported Languages

| Language   | Status   |
|------------|----------|
| TypeScript | ✅ Stable |
| Rust       | ✅ Stable |
| Python     | ✅ Experiemental    |

## Code Generation Workflow

See demo project setup [https://github.com/thepartly/reflectapi/tree/main/reflectapi-demo](https://github.com/thepartly/reflectapi/tree/main/reflectapi-demo)

1. **Define your API server** using `reflectapi` traits and builder
2. **Generate schema** as JSON from your Rust application
3. **Run the CLI** to generate client libraries
4. **Use the clients** in your applications with full type safety

```bash
# Generate schema (from your Rust app)
cargo run  # Your app should save reflectapi-schema.json

# Generate clients
cargo run reflectapi codegen --language typescript --schema reflectapi-schema.json --output clients/typescript
cargo run reflectapi codegen --language python --schema reflectapi-schema.json --output clients/python
cargo run reflectapi codegen --language rust --schema reflectapi-schema.json --output clients/rust
```

## Common Features

All generated clients share these characteristics:

### Type Safety
- Native type definitions for each language
- Compile-time or runtime type checking
- IDE support with autocompletion

### Extensibility
- Default base client implementation is provided
- Which can be replaced or extended with features, such opentelemetry instrumentation or playwrite tracing 

### Error Handling
- Structured error types
- Network error handling

### Async Support
- Modern async/await patterns

### Documentation
- Generated from your Rust documentation
- Type information in IDE tooltips

### HTTP Client Libraries

| Language | HTTP Library | Features |
|----------|--------------|----------|
| TypeScript | fetch API | Native browser/Node.js support |
| Python | httpx | Async/sync, HTTP/2, connection pooling |
| Rust | reqwest | Async, HTTP/2, TLS, middleware |

### Serialization

| Language | Serialization | Validation |
|----------|---------------|------------|
| TypeScript | JSON.parse/stringify | Runtime type checking |
| Python | Pydantic | Schema validation, type coercion |
| Rust | JSON or MessagePack (serde) | Compile-time, zero-cost |
