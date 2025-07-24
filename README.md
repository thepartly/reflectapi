# ReflectAPI

ReflectAPI is a library for Rust code-first web service API declaration and corresponding clients code generation tools.

## Features

- **Code-First API Design**: Define your API in Rust using traits and generate clients automatically
- **Transport Metadata**: Rich access to HTTP status codes, headers, timing, and raw transport objects
- **Multi-Language Support**: Generate TypeScript and Rust clients from the same API definition
- **Error Handling**: Comprehensive error types with transport metadata for advanced error handling
- **Backward Compatible**: Existing code continues to work while new features are opt-in

More documentation will follow later.

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

To release

```
cargo release --exclude reflectapi-demo --exclude reflectapi-demo-client --exclude reflectapi-demo-client-generated minor --execute
```
