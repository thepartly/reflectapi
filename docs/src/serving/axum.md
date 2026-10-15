# Serving with axum

`reflectapi::axum::into_router` turns the routers of a built API into an
[axum](https://docs.rs/axum) `Router`. Use it for a native server on tokio.

## Dependencies

```bash
cargo add reflectapi --features builder,axum
cargo add axum
cargo add tokio --features full
```

reflectapi's `axum` feature does not enable axum's default features. Your own
`axum` dependency enables what your server uses: its defaults include `tokio`
and `http1`, which `axum::serve` needs.

## Mount the routes

```rust,ignore
let (schema, routers) = api().build()?;

let app = reflectapi::axum::into_router(app_state, routers, |name, router| {
    // Called once per router: `name` is the name of the builder that defined it,
    // including builders merged with `Builder::extend` and `Builder::nest`.
    router.layer(tower_http::trace::TraceLayer::new_for_http())
});

let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
axum::serve(listener, app).await?;
```

- Every route is mounted at `POST {path}/{name}`, and readonly routes at
  `GET {path}/{name}` as well (axum also answers `HEAD` for them, without a
  body). Another method gets `405 Method Not Allowed` with an `allow` header
  (`POST`, or `GET,HEAD,POST`); an unknown path gets `404`.
- The callback receives each router with its name, so middleware can differ per
  API module: authentication for one, a different timeout for another.
- `app_state` is cloned into every handler call. Use cheap clones, such as an
  `Arc`.
- Stream routes respond with Server-Sent Events, one `data:` event per item.
  An error returned before the stream opens is a normal HTTP error response.

## The schema

`build()` returns the schema next to the routes. Write it out for client
generation, or serve it, for example as OpenAPI:

```rust,ignore
let openapi = reflectapi::codegen::openapi::Spec::from(&schema); // `codegen` feature
let app = app.route("/openapi.json", axum::routing::get(|| async { axum::Json(openapi) }));
```

A server that doesn't need the schema at runtime can build the routes alone:
`api::<reflectapi::DispatchOnly>().build()?` returns the same routers, without
reflecting any type, and `into_router` accepts them. Generate the schema at
build time instead. See [build modes](./other-servers.md#build-modes).
