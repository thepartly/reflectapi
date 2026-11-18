# Generate a Python Client from a Rust API

You have a web API written in Rust and Python code that calls it: scripts,
tests, data jobs or another service. You want typed Python models and client
methods generated from the Rust code, so the Python side does not hand-write
request payloads and parse untyped JSON.

`reflectapi` does this. You declare the API in Rust with serde types, async
handler functions and a `reflectapi::Builder`. The builder produces a schema,
and the `reflectapi` CLI generates a Python package from it: Pydantic models
for the request and response types, and an async client (optionally a sync
one too) built on `httpx`.

Python output is **experimental**. TypeScript and Rust clients are stable; see
[Supported Outputs](../clients/README.md#supported-outputs).

## 1. Define the API in Rust

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

fn api() -> Builder<()> {
    Builder::new()
        .name("User API")
        .route(create_user, |route| route.name("users.create"))
        .route(get_user, |route| route.name("users.get"))
}
```

`build()` returns the schema and the routes. Write the schema to a file for
client generation:

```rust,ignore
let (schema, routers) = api().build()?;
std::fs::write("reflectapi.json", serde_json::to_string_pretty(&schema)?)?;
```

Serve `routers` with [axum](../serving/axum.md),
[Cloudflare Workers or celld](../serving/workers.md), or
[any other HTTP server](../serving/other-servers.md). The
[TypeScript guide](./typescript-client-axum.md) has a complete axum server.

## 2. Generate the Python client

```bash
cargo install reflectapi-cli
mkdir -p clients/python
reflectapi codegen \
  --language python \
  --schema reflectapi.json \
  --output clients/python/api_client/ \
  --python-sync
```

The output directory is a Python package named `api_client`. It contains the
clients (`AsyncClient` always, `Client` with `--python-sync`) and a module for
each Rust namespace of the reflected types. For a crate named `my-api`, the
models above are in `api_client.my_api`. See
[Client Libraries](../clients/README.md#python) for the full layout.

## 3. Install the runtime

Generated code imports `reflectapi_runtime`. Install the `reflectapi-runtime`
release that matches the CLI that generated the client. It needs Python 3.12
or later.

```bash
pip install reflectapi-runtime
```

Make the generated `api_client` package importable: put it on the import path,
or package it as part of your project.

## 4. Call the API

Async:

```python
import asyncio

from api_client import AsyncClient
from api_client.my_api import CreateUserRequest


async def main() -> None:
    async with AsyncClient("http://localhost:3000") as client:
        created = await client.users.create(
            CreateUserRequest(name="Bob", email="bob@example.com")
        )
        print(created.value)

        found = await client.users.get(1)
        print(found.value)


asyncio.run(main())
```

Sync, when generated with `--python-sync`:

```python
from api_client import Client
from api_client.my_api import CreateUserRequest

with Client("http://localhost:3000") as client:
    created = client.users.create(CreateUserRequest(name="Bob", email="bob@example.com"))
    print(created.value.name)
```

Each method returns an `ApiResponse` whose `value` is the validated response
model. A non-2xx response raises `ApplicationError`, which carries the route's
typed error when it declares one. Transport failures raise `NetworkError` or
`TimeoutError`, and a response that does not match the model raises
`ValidationError`. All four are exported by `reflectapi_runtime`.

## Details worth knowing

- Requests omit fields you did not set and send an explicit `None` as `null`;
  see [Missing fields and explicit null](../clients/README.md#missing-fields-and-explicit-null).
- Streaming routes return iterators; see
  [Streaming Endpoints](../clients/README.md#streaming-endpoints).
- Regenerate the client when the API changes. Committing the generated
  package and having CI check it is up to date keeps the two in step; see
  [Generate the schema and clients at build time](../serving/workers.md#generate-the-schema-and-clients-at-build-time).
