# Changelog

## 0.17.2 — transport-shape refactor

The Rust, TypeScript, and Python clients now share a single `Request` DTO shape:
`{ path, headers, body }`. The base URL lives on the transport, not on the
generated `Interface`.

### Rust

The trait gained a `base_url` method and `Request::url` was renamed to `Request::path`:

```rust
pub trait Client {
    type Error;
    fn base_url(&self) -> &Url;
    fn request(&self, request: Request) -> impl Future<Output = Result<Response, Self::Error>>;
}

pub struct Request {
    pub path: String,        // was: url: Url
    pub headers: HeaderMap,
    pub body: Bytes,
}
```

`reqwest::Client` no longer implements `Client` directly — it doesn't carry a
base URL. Pick the path that matches your setup:

**Plain `reqwest::Client`, generated crate has a `reqwest` feature.** The
generated `Interface::try_new(reqwest::Client, base_url)` convenience
constructor keeps working unchanged:

```rust
let api = MyClient::try_new(reqwest::Client::new(), base_url)?;
```

The constructor is gated on the generated crate enabling its own `reqwest`
feature (which should re-export `reflectapi/reqwest`). If you don't see
`try_new`, this is why.

**`reqwest_middleware::ClientWithMiddleware` (the common path with
otel/retry/etc.).** Compose the transport explicitly — there's no
convenience constructor for this case because of method-resolution
ambiguity:

```rust
let transport = reflectapi::rt::ReqwestMiddlewareClient::try_new(
    mw_client,
    base_url,
)?;
let api = MyClient::new(transport);
```

`ReqwestMiddlewareClient` is a type alias for
`ReqwestClient<reqwest_middleware::ClientWithMiddleware>`.

**Custom transport.** Wrap with `ReqwestClient::try_new(...)` (or implement
the `Client` trait yourself) and pass the result to `MyClient::new`:

```rust
let api = MyClient::new(
    reflectapi::rt::ReqwestClient::try_new(reqwest::Client::new(), base_url)?
);
```

`ReqwestClient::try_new` returns `Result<Self, url::ParseError>` because it
validates that `base_url` is a valid HTTP base. There is no infallible
`ReqwestClient::new` constructor.

#### `reqwest-middleware` feature implies `reqwest` (alpha.4)

In 0.17.2-alpha.3 enabling only `reqwest-middleware` failed to compile
because the wrapper struct lives behind the `reqwest` feature. Fixed in
0.17.2-alpha.4: `reqwest-middleware` now implies `reqwest`. Consumers that
already enable both don't need to change anything.

### TypeScript (alpha.4)

Codegen now emits two files: `generated.ts` (the API surface) and
`generated.transport.ts` (the transport contract). Most consumers only
need `generated.ts`. Custom transports import from the transport
submodule:

```ts
import type { Client, Request, Response } from './generated.transport';
```

The bare `Request` / `Response` / `Headers` names live behind the
`./generated.transport` import path, so they no longer shadow the DOM
globals of the same name when imported from `generated.ts`. See #143
for the rationale.

If your build tooling assumed a single generated file, update it to
include the new sibling. Typical pnpm/npm workflows pick it up
automatically (the file sits in the same directory).

### Python

`Request` / `Response` / `Client` / `AsyncClient` are exported from the
`reflectapi_runtime.transport` module. They are also re-exported from the
top-level `reflectapi_runtime` package for convenience. No collision with
standard-library names.
