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
- Stream routes respond with Server-Sent Events, one `data:` event per item,
  with an `id:` for items the route wraps in `reflectapi::Event`. An error
  returned before the stream opens is a normal HTTP error response.

## Keep idle streams open

Proxies and load balancers close a connection that sends nothing for a while,
often after 60 to 120 seconds. A stream route that only emits on change is
then reconnected over and over, and each reconnect repeats the handler's
connect work. `into_router_with` can write an SSE comment (`:` and a blank
line) whenever a stream has sent nothing for an interval:

```rust,ignore
use std::time::Duration;
use reflectapi::axum::ServeOptions;

let options = ServeOptions::default()
    .sse_keep_alive(Duration::from_secs(15), tokio::time::sleep);
let app = reflectapi::axum::into_router_with(app_state, routers, options, |_name, router| router);
```

- The interval restarts after every event and every comment, so a busy
  stream carries no extra bytes.
- Clients that follow the SSE spec ignore comments, and every generated
  client does; the schema is unchanged.
- It is off by default. 15 seconds is well inside common proxy timeouts.
- reflectapi depends on no async runtime, so you pass the timer:
  `tokio::time::sleep` on tokio.
- The default is set where the server is assembled, because the interval
  depends on the proxies in front of a deployment, and the same route
  definitions may be served in several places. Leave it off when a proxy in
  front already sends heartbeats.

A stream route that needs a different interval, or none, sets its own in the
`Builder`. It overrides the server's default, and `Duration::ZERO` turns
keep-alive off for that route:

```rust,ignore
builder.stream_route(watch_jobs, |b| {
    b.name("jobs.watch").sse_keep_alive(Duration::from_secs(5))
})
```

- The route's interval changes only how it is served: the schema and the
  generated clients are the same. Setting it on a route that isn't a stream
  route is a build error.
- Serving needs a timer for it. `ServeOptions::sse_keep_alive` provides one
  along with the default; to keep alive only the routes that ask, pass just
  the timer with `ServeOptions::default().sse_timer(tokio::time::sleep)`.
  `into_router_with` panics if a route sets an interval and there is no
  timer, and `into_router` has none.

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
