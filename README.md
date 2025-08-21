# ReflectAPI

ReflectAPI is a library for Rust code-first web service API declaration and corresponding clients code generation tools.

## Features

- **Code-first API definition** using Rust types and derive macros
- **Multi-language client generation**: TypeScript, Rust, and Python
- **OpenAPI spec generation** for documentation and tooling
- **Type-safe** clients with full IntelliSense support
- **Web framework integration** (Axum supported)

## Documentation

- [API Documentation](https://docs.rs/reflectapi) - Complete API reference
- [User Guide](https://thepartly.github.io/reflectapi/) - Tutorials and examples

### Development notes

Ensure that you have `prettier` and `rustfmt` available in your PATH to format generated code.

To run the demo server:

```
cargo run --bin reflectapi-demo
```

To generate client in Typescript for demo server:

```
cargo run --bin reflectapi-cli -- codegen --language typescript --schema reflectapi-demo/reflectapi.json --output reflectapi-demo/clients/typescript
```

To run the Typescript generated client. Note: requires the demo server running

```
cd reflectapi-demo/clients/typescript/
pnpm install
pnpm run start
```

To generate client in Rust for demo server:

```
cargo run --bin reflectapi-cli -- codegen --language rust --schema reflectapi-demo/reflectapi.json --output reflectapi-demo/clients/rust/generated/src/
```

To run the Rust generated client. Note: requires the demo server running

```
cargo run --bin reflectapi-demo-client --all-features
```

To generate client in Python for demo server:

```
cargo run --bin reflectapi-cli -- codegen --language python --schema reflectapi-demo/reflectapi.json --output reflectapi-demo/clients/python
```

To release

```
cargo release --exclude reflectapi-demo --exclude reflectapi-demo-client --exclude reflectapi-demo-client-generated minor --execute
```
