<div align="center">
  <img src=".github/reflectapi-text.png" alt="reflectapi" width="400">
  <br>
  <img src=".github/reflectapi-logo.png" alt="reflectapi logo" width="150">
</div>

<div align="center">
  <a href="https://crates.io/crates/reflectapi">
    <img src="https://img.shields.io/crates/v/reflectapi.svg" alt="Crates.io">
  </a>
  <a href="https://docs.rs/reflectapi">
    <img src="https://docs.rs/reflectapi/badge.svg" alt="Documentation">
  </a>
  <a href="https://github.com/thepartly/reflectapi/blob/main/LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License">
  </a>
</div>

<br>

`reflectapi` is a library and a toolkit for writing web API services in Rust and generating compatible corresponding clients, delivering best possible development experience and efficiency.

## Features

- **Code-first definition of services with API over HTTP**
- **100% compatible type-safe and extensible clients** delivering same simplicity and intent of the server API
- **Multi-language client generation**:
  - TypeScript - production
  - Rust - production
  - Python - experimental
- **Documentation rendering** using Redoc tool
- **Web framework agnostic** with plugable Axum support included, other frameworks are possible
- **Full support for all serde attributes**
- **Full enablement for all Rust types**, including standard library, common popular crates and adding support for 3rd party and custom types is straightforward


## Installation

Add `reflectapi` to your `Cargo.toml`:

```toml
[dependencies]
reflectapi = "*"
```

Or install via cargo:

```bash
cargo add reflectapi
```

## Documentation

- 📦 [Crates.io](https://crates.io/crates/reflectapi) - Package information and versions
- 📖 [API Documentation](https://docs.rs/reflectapi) - Complete API reference  
- 📚 [User Guide](https://thepartly.github.io/reflectapi/) - Tutorials and examples
- 🚀 [Quick Start](https://thepartly.github.io/reflectapi/getting-started/quick-start.html) - Get up and running in 5 minutes

## Development notes

### Building and running

Ensure that you have `prettier` and `rustfmt` available in your PATH to format generated code.

To run the demo server:

```
cargo run --bin reflectapi-demo
```

To generate client in Typescript for demo server:

```
cargo run --bin reflectapi -- codegen --language typescript --schema reflectapi-demo/reflectapi.json --output reflectapi-demo/clients/typescript
```

To run the Typescript generated client. Note: requires the demo server running

```
cd reflectapi-demo/clients/typescript/
npm install
npm run start
```

To generate client in Rust for demo server:

```
cargo run --bin reflectapi -- codegen --language rust --schema reflectapi-demo/reflectapi.json --output reflectapi-demo/clients/rust/generated/src/
```

To run the Rust generated client. Note: requires the demo server running

```
cd reflectapi-demo/clients/rust/
cargo run --all-features
```


To generate client in Python for demo server:

```
cargo run --bin reflectapi -- codegen --language python --schema reflectapi-demo/reflectapi.json --output reflectapi-demo/clients/python
```

### Updating Snapshots

This project uses `insta` for snapshot testing to ensure code generation output is correct and stable. When tests fail due to snapshot mismatches:

1. Review the changes first to ensure they are expected
2. Update snapshots using one of these commands:

```bash
# Interactive review (recommended)
cargo insta review

# Auto-accept all changes (use with caution)
cargo insta accept
```

3. Re-run tests to verify they pass:

```bash
cargo test
```

### Building Documentation

```bash
# Install required tools (one-time setup)
cargo install mdbook mdbook-keeper

# Build documentation
cd docs
mdbook build

# Serve documentation locally
mdbook serve  # Opens at http://localhost:3000

# Note: mdbook-keeper automatically runs doctests during build
# The build command both generates HTML and tests code examples
```

### Releasing

```
cargo release --exclude reflectapi-demo --exclude reflectapi-demo-client --exclude reflectapi-demo-client-generated minor --execute
```
