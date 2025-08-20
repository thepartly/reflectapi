# ReflectAPI

Code-first web service API declaration and client generation for TypeScript, Rust, and Python.

## Overview

ReflectAPI is a Rust library for declaring web service APIs in a code-first manner and generating client libraries for multiple target languages. Define your API using Rust types and traits, then generate everything else automatically.

## Quick Start

Add ReflectAPI to your `Cargo.toml`:

```toml
[dependencies]
reflectapi = { version = "0.15", features = ["builder", "axum"] }
serde = { version = "1.0", features = ["derive"] }
```

Define your API types:

```rust
use reflectapi::{Builder, Input, Output};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Input, Output)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Serialize, Deserialize, Input)]
struct CreateUserRequest {
    name: String,
    email: String,
}

fn create_user(req: CreateUserRequest) -> User {
    User { 
        id: 1, 
        name: req.name, 
        email: req.email 
    }
}
```

Build your API:

```rust
let builder = Builder::new()
    .name("User API")
    .description("A simple user management API")
    .route(create_user, |route| {
        route
            .name("users.create")
            .description("Create a new user")
    });

let (schema, routers) = builder.build()?;
```

## Features

- **Type-safe API definition**: Use Rust types to define your API
- **Multi-language client generation**: Generate TypeScript, Rust, and Python clients
- **Web framework integration**: Built-in support for Axum
- **OpenAPI compatible**: Generate OpenAPI specifications
- **Rich type system**: Support for generics, enums, and complex types

## Code Generation

Generate client libraries for your API:

```bash
# TypeScript client
cargo run --bin reflectapi-cli -- codegen --language typescript --schema api.json --output ./clients/typescript

# Python client  
cargo run --bin reflectapi-cli -- codegen --language python --schema api.json --output ./clients/python

# Rust client
cargo run --bin reflectapi-cli -- codegen --language rust --schema api.json --output ./clients/rust
```

## Documentation

- [API Documentation](https://docs.rs/reflectapi)
- [Examples](https://github.com/thepartly/reflectapi/tree/main/reflectapi-demo)

## License

MIT
