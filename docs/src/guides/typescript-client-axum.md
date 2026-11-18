# Generate a TypeScript Client from a Rust API Served with Axum

You have a web API in Rust, served with axum, and a TypeScript frontend or
service that calls it. You want the TypeScript request and response types, and
the functions that call each endpoint, generated from the Rust code instead of
written and kept in sync by hand.

`reflectapi` does this. You declare the API in Rust: serde types, async handler
functions, and a `reflectapi::Builder` that names the routes. The builder
produces a schema and the routes. `reflectapi::axum::into_router` turns the
routes into an axum `Router`, and the `reflectapi` CLI turns the schema into a
typed TypeScript client.

`reflectapi` does not read existing axum handlers or extractors. Handlers have
`reflectapi`'s own signature and are registered with its `Builder`; axum is one
way to serve them. The same API can also run on
[Cloudflare Workers or celld](../serving/workers.md), or on
[any other HTTP server](../serving/other-servers.md). If you want to keep
axum-native handlers and document them, see the
[comparison with utoipa and aide](./comparison.md).

## 1. Add dependencies

```bash
cargo new my-api
cd my-api
cargo add reflectapi --features builder,axum
cargo add serde --features derive
cargo add serde_json
cargo add tokio --features full
cargo add axum
```

## 2. Define types and handlers

```rust,ignore
use reflectapi::{Builder, Input, Output};
use serde::{Deserialize, Serialize};

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
    User {
        id: 1,
        name: req.name,
        email: req.email,
    }
}

async fn get_user(_state: (), id: u32, _headers: reflectapi::Empty) -> Option<User> {
    (id == 1).then(|| User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    })
}
```

- Types the server returns derive `Serialize` and `Output`. Types it accepts
  derive `Deserialize` and `Input`. A type used both ways derives all four.
- A handler takes the application state, the request body, and a struct of
  request headers. `reflectapi::Empty` means the route reads no headers.
- A handler can return `Result<T, E>` to give the route a typed error; see
  [Errors](../concepts/errors.md).
- Serde attributes (`rename`, `tag`, `flatten`, `default` and so on) are
  reflected, so the generated types match what goes over the wire.

## 3. Build the schema and serve it with axum

```rust,ignore
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let builder = Builder::new()
        .name("User API")
        .description("A simple user management API")
        .route(create_user, |route| {
            route.name("users.create").description("Create a new user")
        })
        .route(get_user, |route| {
            route.name("users.get").description("Get a user by ID")
        });

    let (schema, routers) = builder.build()?;

    // The schema is the input to client generation.
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

```bash
cargo run
```

Each route is served at `POST /{name}`: here `POST /users.create` and
`POST /users.get`. The last argument of `into_router` is where to add tower
middleware. [Serving with axum](../serving/axum.md) covers routing, state,
middleware and streaming.

## 4. Generate the TypeScript client

```bash
cargo install reflectapi-cli
mkdir -p clients/typescript
reflectapi codegen --language typescript --schema reflectapi.json --output clients/typescript/
```

This writes two files:

- `generated.ts`: the request and response types and the `client(base)`
  function.
- `generated.transport.ts`: the transport interface, for custom transports.

The files import nothing but each other. The default transport uses the
global `fetch`, so it runs in browsers and in current Node.js.

## 5. Call the API from TypeScript

```typescript
import { client } from "./clients/typescript/generated";

const c = client("http://localhost:3000");

// Each method takes the typed input and the typed request headers.
const created = await c.users.create(
  { name: "Bob", email: "bob@example.com" },
  {},
);
if (created.is_ok()) {
  console.log(created.unwrap_ok());
}

const user = (await c.users.get(1, {})).unwrap_ok();
console.log(user?.name);
```

A call returns a result instead of throwing: `is_ok()`, `unwrap_ok()`, and an
`Err` that separates the route's typed error from network and server
failures. See [Generated Clients](../clients/index.md#typescript) for errors,
response headers, streaming and custom transports.

## Keep the client in sync

Regenerate the client whenever the API changes. A common setup commits the
schema and the generated files, and has CI regenerate them and fail on a
difference, so an API change cannot merge without its client. See
[Generate the schema and clients at build time](../serving/workers.md#generate-the-schema-and-clients-at-build-time)
for doing this from a test, and [build modes](../serving/other-servers.md#build-modes)
for building only the routes in the server binary.

The same schema also produces a [Python client](./python-client.md), a Rust
client and an OpenAPI document; see [Generated Clients](../clients/index.md).
