# Quick Start

Build a small API in Rust, serve it with axum, and call it from a generated
TypeScript client.

## Create a project

```bash
cargo new my-api
cd my-api
cargo add reflectapi --features builder,axum
cargo add serde --features derive
cargo add serde_json
cargo add tokio --features full
cargo add axum
```

[Installation](./installation.md) lists the other crate features.

## Define the API

Replace `src/main.rs`:

```rust,no_run
use reflectapi::{Builder, Input, Output};
use serde::{Deserialize, Serialize};

/// A user of the system.
#[derive(Serialize, Output)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Deserialize, Input)]
struct CreateUserRequest {
    name: String,
    email: String,
}

async fn create_user(_state: (), req: CreateUserRequest, _headers: reflectapi::Empty) -> User {
    User { id: 1, name: req.name, email: req.email }
}

async fn get_user(_state: (), id: u32, _headers: reflectapi::Empty) -> Option<User> {
    (id == 1).then(|| User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (schema, routers) = Builder::new()
        .name("User API")
        .description("A simple user management API")
        .route(create_user, |route| {
            route.name("users.create").description("Create a new user")
        })
        .route(get_user, |route| {
            route.name("users.get").description("Get a user by ID")
        })
        .build()?;

    // The schema is what clients are generated from.
    std::fs::write("reflectapi.json", serde_json::to_string_pretty(&schema)?)?;

    let app = reflectapi::axum::into_router(
        (),
        routers,
        reflectapi::axum::RouterSettings::default(),
        |_name, router| router,
    );
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

- A type the server returns derives `Serialize` and `Output`; a type it
  accepts derives `Deserialize` and `Input`. A type used both ways derives
  all four.
- A handler takes the application state, the request input and the request
  headers. `reflectapi::Empty` means the route reads no headers.
- Doc comments on types and fields, and each route's `description`, are
  carried into the generated clients.

[How it works](../concepts/how-it-works.md) explains these pieces.

## Run it

```bash
cargo run
```

The server writes `reflectapi.json` and listens on port 3000. Each route is a
`POST` to its name:

```bash
curl -X POST localhost:3000/users.get -H 'content-type: application/json' -d '1'
```

[On the Wire](../concepts/wire.md) describes requests and responses in full.

## Generate a TypeScript client

```bash
cargo binstall reflectapi-cli   # or: cargo install reflectapi-cli
mkdir -p clients/typescript
reflectapi codegen --language typescript --schema reflectapi.json --output clients/typescript/
```

## Call the API

```typescript
import { client } from "./clients/typescript/generated";

const c = client("http://localhost:3000");

const created = await c.users.create({ name: "Bob", email: "bob@example.com" }, {});
if (created.is_ok()) {
  console.log(created.unwrap_ok());
}
```

The client's types follow the Rust ones: change a field in Rust, regenerate,
and the TypeScript compiler points at every call that needs updating.

## Next

- [Generate a Python client](../guides/python-client.md), or see
  [Generated clients](../clients/index.md) for all three languages.
- [Errors](../concepts/errors.md): give a route a typed error.
- [Serving with axum](../serving/axum.md): state, middleware and streaming.
