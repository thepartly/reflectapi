# Client Comparison

Detailed comparison of features across generated client languages.

## Language Support Matrix

| Feature | TypeScript | Python | Rust |
|---------|------------|--------|------|
| Type Safety | ✅ Compile-time | ✅ Runtime (Pydantic) | ✅ Compile-time |
| Async/Await | ✅ Promise-based | ✅ asyncio | ✅ tokio |
| Error Handling | ✅ Structured | ✅ Structured | ✅ Comprehensive |
| Documentation | ✅ TSDoc | ✅ Docstrings | ✅ rustdoc |
| IDE Support | ✅ Full IntelliSense | ✅ Type hints | ✅ Full analysis |
| Tree Shaking | ✅ ES modules | ❌ | ✅ Dead code elimination |
| Binary Size | N/A | N/A | ✅ Minimal |

## HTTP Client Libraries

| Language | HTTP Library | Features |
|----------|--------------|----------|
| TypeScript | fetch API | Native browser/Node.js support |
| Python | httpx | Async/sync, HTTP/2, connection pooling |
| Rust | reqwest | Async, HTTP/2, TLS, middleware |

## Serialization

| Language | Serialization | Validation |
|----------|---------------|------------|
| TypeScript | JSON.parse/stringify | Runtime type checking |
| Python | Pydantic | Schema validation, type coercion |
| Rust | serde | Compile-time, zero-cost |

## Error Types

### TypeScript
```typescript
interface ApiError {
  status: number;
  message: string;
  data?: any;
}
```

### Python
```python
class ApiError(Exception):
    def __init__(self, status: int, message: str, data: Any = None):
        self.status = status
        self.message = message
        self.data = data
```

### Rust
```rust,ignore
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("API error: {status}")]
    Api { status: u16, body: String },
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
```

## Performance Characteristics

| Aspect | TypeScript | Python | Rust |
|--------|------------|--------|------|
| Cold Start | Fast | Medium | Fast |
| Runtime Performance | V8 optimized | Interpreted | Native |
| Memory Usage | Medium | High | Low |
| Binary Size | Small (gzipped) | N/A | Very small |
| Build Time | Fast | N/A | Medium |

## Ecosystem Integration

### TypeScript
- **Bundlers**: webpack, vite, rollup
- **Testing**: jest, vitest, playwright
- **Frameworks**: React, Vue, Angular, Node.js

### Python
- **Async Frameworks**: FastAPI, aiohttp, Quart
- **Testing**: pytest, httpx test client
- **Data Science**: pandas, numpy (type-safe APIs)

### Rust
- **Web Frameworks**: axum, warp, actix-web
- **Testing**: tokio-test, wiremock
- **CLI Tools**: clap, excellent for CLI clients

## When to Use Which Client

### Choose TypeScript when:
- Building web applications or Node.js services
- Team is familiar with JavaScript/TypeScript
- Need excellent IDE support and debugging
- Rapid prototyping and development

### Choose Python when:
- Integrating with data science workflows
- Team prefers Python ecosystem
- Need dynamic runtime flexibility
- Building automation scripts or data pipelines

### Choose Rust when:
- Performance is critical
- Building CLI tools or system services
- Want maximum type safety
- Memory efficiency matters
- Long-running services

## Migration Between Clients

All clients share the same underlying schema, making migration straightforward:

1. **API Compatibility**: Same endpoints and data structures
2. **Error Handling**: Similar error patterns across languages
3. **Documentation**: Consistent API reference
4. **Testing**: Same test scenarios apply

Example: TypeScript to Python migration:

```typescript
// TypeScript
const user = await client.users.get(123);
```

```python
# Python equivalent
user = await client.users.get(123)
```

```rust,ignore
// Rust equivalent
let user = client.users().get(123).await?;
```