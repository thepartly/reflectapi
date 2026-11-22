# Installation

## The crate

A server that defines and serves an API with axum needs:

```bash
cargo add reflectapi --features builder,axum
```

Add features for the third-party types your API uses, for example
`--features builder,axum,chrono,uuid`. Without its feature, a type such as
`chrono::DateTime` can't be part of an [`Input`](https://docs.rs/reflectapi/latest/reflectapi/trait.Input.html) or [`Output`](https://docs.rs/reflectapi/latest/reflectapi/trait.Output.html) type.

| Feature | Enables |
|---------|---------|
| `builder` | [`Builder`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html) and the routers: defining an API |
| `axum` | [`reflectapi::axum`](https://docs.rs/reflectapi/latest/reflectapi/axum/index.html): serving the routes with axum (implies `builder`) |
| `msgpack` | MessagePack request and response bodies beside JSON (implies `builder`) |
| `codegen` | [`reflectapi::codegen`](https://docs.rs/reflectapi/latest/reflectapi/codegen/index.html): generating clients or OpenAPI from Rust code instead of the CLI |
| `glob` (default) | [`Builder::glob_rename_types`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.glob_rename_types) |
| `chrono`, `chrono-tz` | `chrono` dates and times, `chrono_tz::Tz` |
| `uuid` | `uuid::Uuid` |
| `url` | `url::Url` |
| `rust_decimal` | `rust_decimal::Decimal` |
| `indexmap` | `IndexMap` and `IndexSet` |
| `json` | `serde_json::Value` as an untyped value |
| `rt` | The runtime of generated Rust clients |
| `reqwest`, `reqwest-middleware` | A generated Rust client calling over `reqwest`, or `reqwest-middleware` |
| `rt-sse` | Streaming routes in generated Rust clients |

A crate holding a generated Rust client depends on `reflectapi` with `rt`,
plus `reqwest` to make the calls and `rt-sse` if the API streams. See
[Generated clients](../clients/index.md#rust).

## The CLI

The `reflectapi` CLI generates clients from a schema file:

```bash
cargo binstall reflectapi-cli   # a prebuilt binary
cargo install reflectapi-cli    # or build it
```

Use a CLI at least as new as the `reflectapi` crate that wrote the schema.
The two are often different releases, so the schema records the format it's
written in as `format_version`:

- **A CLI reads any schema format up to its own.** A newer CLI reads an older schema.
- **A CLI refuses a newer format** and says to upgrade it, rather than generating a client that's silently wrong.
- **Within a format, changes are additive.** An older CLI that ignores something new, such as a deprecation or a `distinct` type, still generates a client that makes the same requests and accepts the same responses. It just has less type information. Anything an older CLI would get wrong by ignoring it moves to a new format: a new kind of route, a change to a type's JSON, or a change to how a route is called, such as its HTTP method.

So when you upgrade `reflectapi` on the server, upgrade `reflectapi-cli` wherever clients are generated from its schema.

## The Python runtime

Generated Python clients import the `reflectapi-runtime` package; see
[Generate a Python Client](../guides/python-client.md).
