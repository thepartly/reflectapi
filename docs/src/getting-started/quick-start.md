# Quick Start

This guide will have you up and running with `reflectapi` in under 5 minutes.

## Prerequisites

- Rust 1.78.0 or later
- Basic familiarity with Rust and web APIs

## Create a New Project

```bash
cargo new my-api
cd my-api
```

## Add Dependencies

Add `reflectapi` to your `Cargo.toml`:

```toml
[dependencies]
reflectapi = { version = "0.15", features = ["builder", "axum"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
axum = "0.7"
```

## Define Your API

Replace the contents of `src/main.rs`:

```rust,ignore
// This is a complete example for src/main.rs
// Add these dependencies to your Cargo.toml first:
// [dependencies]
// reflectapi = { version = "0.15", features = ["builder", "axum"] }
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// tokio = { version = "1.0", features = ["full"] }

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

// Handler functions need specific signatures for reflectapi
async fn create_user(_state: (), req: CreateUserRequest, _headers: ()) -> User {
    // In a real app, you'd save to a database
    User { 
        id: 1, 
        name: req.name, 
        email: req.email 
    }
}

async fn get_user(_state: (), id: u32, _headers: ()) -> Option<User> {
    // In a real app, you'd query a database
    if id == 1 {
        Some(User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        })
    } else {
        None
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build the API schema
    let builder = Builder::new()
        .name("User API")
        .description("A simple user management API")
        .route(create_user, |route| {
            route
                .name("users.create")
                .description("Create a new user")
        })
        .route(get_user, |route| {
            route
                .name("users.get")
                .description("Get a user by ID")
        });

    let (schema, routers) = builder.build()?;
    
    // Save schema for client generation
    let schema_json = serde_json::to_string_pretty(&schema)?;
    std::fs::write("reflectapi-schema.json", schema_json)?;
    
    println!("✅ API schema generated at reflectapi-schema.json");
    
    // Start the HTTP server
    let app_state = (); // No state needed for this example
    let axum_app = reflectapi::axum::into_router(app_state, routers, |_name, r| r);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("🚀 Server running on http://0.0.0.0:3000");
    println!("📖 Ready to generate clients!");
    
    axum::serve(listener, axum_app).await?;
    
    Ok(())
}
```

## Run Your API Server

```bash
cargo run
```

You should see:
```text
✅ API schema generated at reflectapi-schema.json
🚀 Server running on http://0.0.0.0:3000
📖 Ready to generate clients!
```

🎉 **Congratulations!** You now have a running API server and generated client-ready schema.

## Generate a Client

First, install the CLI:

```bash
cargo install reflectapi-cli
```

Then generate a TypeScript client:

```bash
mkdir clients
reflectapi-cli codegen --language typescript --schema reflectapi-schema.json --output clients/typescript
```

## Use Your Generated Client

The generated TypeScript client will be fully typed:

```typescript
import { client } from './clients/typescript';

const c = client('http://localhost:3000');

// Create a user - fully type-safe!
const newUser = await c.users.create({
  name: 'Bob',
  email: 'bob@example.com'
});

// Get a user
const user = await c.users.get(1);
```
That's it!
