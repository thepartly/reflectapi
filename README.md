<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/src/reflectapi-wordmark-dark.svg">
    <img src="docs/src/reflectapi-wordmark.svg" alt="reflectapi" width="360">
  </picture>
</div>

<div align="center">
  <a href="https://crates.io/crates/reflectapi">
    <img src="https://img.shields.io/crates/v/reflectapi.svg" alt="Crates.io">
  </a>
  <a href="https://docs.rs/reflectapi">
    <img src="https://docs.rs/reflectapi/badge.svg" alt="Documentation">
  </a>
  <a href="https://crates.io/crates/reflectapi">
    <img src="https://img.shields.io/crates/l/reflectapi.svg" alt="License">
  </a>
</div>

<br>

<p align="center">
  <strong>Define your web API in Rust, generate typed TypeScript, Python and Rust clients, typed streams included. Serve it with axum, Cloudflare Workers or any HTTP server.</strong>
</p>

<p align="center">
  <strong>Built and used in production at <a href="https://partly.com">Partly</a>.</strong>
</p>

<p align="center">
  <a href="https://thepartly.github.io/reflectapi/">Website</a> ·
  <a href="https://thepartly.github.io/reflectapi/getting-started/quick-start.html">Quick start</a> ·
  <a href="https://docs.rs/reflectapi">API reference</a> ·
  <a href="https://www.youtube.com/watch?v=6Scgq9fBZQM&amp;t=21675s">Talk: Reflect your Rust API into the world</a>
</p>

## Example

A route is a Rust function:

```rust
use reflectapi::{Builder, Empty, Input, Output};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Input)]
struct CreateUser { name: String, email: String }

#[derive(Serialize, Output)]
struct User { id: u32, name: String, email: String }

async fn create_user(_: (), req: CreateUser, _: Empty) -> User {
    User { id: 1, name: req.name, email: req.email }
}

let (schema, routers) = Builder::new()
    .route(create_user, |r| r.name("users.create"))
    .build()?;
```

`reflectapi codegen` turns the schema into clients, and each client gets the
same function, typed:

```typescript
import { client } from "./generated";

const api = client("https://api.example.com");
const result = await api.users.create({ name: "Ada", email: "ada@example.com" }, {});
if (result.is_ok()) {
  console.log(result.unwrap_ok().id);
}
```

## What you get

- **Clients as precise as your Rust types**: generics, enums with data,
  absent-or-null fields and serde's attributes reach the clients as they are.
- **Typed errors**: a route's error enum chooses each response's status, and
  every client matches on it; a missed variant is a compile error in
  TypeScript and Rust.
- **Typed streams**: a route can stream typed events over Server-Sent Events,
  read as an async iterator or a `Stream` in each client.
- **TypeScript, Python and Rust clients**, plus an OpenAPI document for every
  other language and tool.
- **Deprecations that reach the editor**, and open enums for clients built
  before a variant was added.
- **Serve it anywhere**: axum in one call, Cloudflare Workers, or any HTTP
  server through a framework-free dispatcher.

## Get started

```bash
cargo add reflectapi --features builder,axum
cargo binstall reflectapi-cli   # the codegen CLI
```

Then follow the [Quick start](https://thepartly.github.io/reflectapi/getting-started/quick-start.html).
The [`reflectapi-demo`](reflectapi-demo) crate is a complete example.

## Alternatives

Other ways to connect a Rust server to its clients, and when each fits.

- **[utoipa](https://github.com/juhaku/utoipa)**: you keep your web
  framework's handlers (axum, actix-web, Rocket and others) and want an
  OpenAPI document generated from annotations on them.
- **[aide](https://github.com/tamasfe/aide)**: you build with axum and want
  the OpenAPI document derived from your routes and extractors.
- **OpenAPI client generators**, such as
  [openapi-generator](https://openapi-generator.tech),
  [openapi-typescript](https://openapi-ts.dev),
  [orval](https://orval.dev) and
  [progenitor](https://github.com/oxidecomputer/progenitor): an OpenAPI
  document is the contract, or you need clients in languages reflectapi
  doesn't generate. reflectapi can write that document too; see
  [Supported Outputs](https://thepartly.github.io/reflectapi/clients/#supported-outputs).
- **[ts-rs](https://github.com/Aleph-Alpha/ts-rs),
  [specta](https://github.com/specta-rs/specta) and
  [typeshare](https://github.com/1Password/typeshare)**: you only need your
  Rust types in TypeScript or other languages, and write the calls yourself.
- **[tonic](https://github.com/hyperium/tonic)** with Protocol Buffers: the
  contract is a `.proto` file, and clients and servers speak gRPC.

reflectapi fits when you own the server and its clients, routes can be
named calls rather than REST resources, and the Rust types and their serde
attributes should be the only definition of the API.
[Why reflectapi?](https://thepartly.github.io/reflectapi/introduction.html#why-reflectapi) explains why it doesn't
build on OpenAPI.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT), at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in reflectapi by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for building, testing, the docs and
releases.
