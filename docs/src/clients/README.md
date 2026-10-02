# Client Generation

`reflectapi` can generate client code from a reflected schema JSON file.

## Supported Outputs

| Output | Status | Notes |
|--------|--------|-------|
| TypeScript | Stable | Two generated files: API surface + transport contract |
| Rust | Stable | Single generated file |
| Python | Experimental | Package-style output with real namespace submodules |

OpenAPI generation is also supported by the CLI, but it is documented separately as an API description format rather than a client library.

## Workflow

1. Define your API server using `reflectapi` derives and the builder API.
2. Write the schema JSON from your Rust application.
3. Run `reflectapi codegen` for the target language.
4. Commit or consume the generated client code from your application.

The CLI defaults to `reflectapi.json` if `--schema` is omitted. The demo project uses that filename. If your application writes a different filename such as `reflectapi-schema.json`, pass that path explicitly.

```bash
# Create output directories first. TypeScript and Rust write a single file
# unless the output path already exists as a directory or ends with a slash.
mkdir -p clients/typescript clients/python clients/rust

# Generate TypeScript client -> clients/typescript/generated.ts
#                          and clients/typescript/generated.transport.ts
cargo run --bin reflectapi -- codegen \
  --language typescript \
  --schema reflectapi.json \
  --output clients/typescript/

# Generate Python client -> clients/python/api_client/__init__.py,
# clients/python/api_client/_client.py, and namespace packages such as
# clients/python/api_client/myapi/model/
cargo run --bin reflectapi -- codegen \
  --language python \
  --schema reflectapi.json \
  --output clients/python/api_client/ \
  --python-sync

# Generate Rust client -> clients/rust/generated.rs
cargo run --bin reflectapi -- codegen \
  --language rust \
  --schema reflectapi.json \
  --output clients/rust/
```

If you installed the CLI separately, replace `cargo run --bin reflectapi --` with `reflectapi`.

## Output Shape

The generators do not all emit the same file layout:

| Output | Files written by the generator |
|--------|--------------------------------|
| TypeScript | `generated.ts`, `generated.transport.ts` |
| Rust | `generated.rs` |
| Python | A package directory containing `__init__.py`, `generated.py`, `_client.py`, `_rebuild.py`, and namespace package files |

The demo repository includes extra project scaffolding around some generated clients, but that scaffolding is not produced by `reflectapi codegen` itself.

## Language Behavior

### TypeScript

- Emits two files alongside each other: `generated.ts` (the API
  surface — types, functions, the `client(base)` factory) and
  `generated.transport.ts` (the transport contract — `Request`,
  `Response`, `Headers`, `Client`, `RequestOptions`, `ClientInstance`).
  The split keeps the bare DTO names from shadowing the DOM globals of
  the same name when imported from `generated.ts`. Custom transports
  import from `./generated.transport`.
- Uses generated TypeScript types and function wrappers.
- Uses a `fetch`-based default client implementation.
- Parses JSON responses, but does not generate runtime schema validators today.
- Supports custom client implementations via the generated client interface.
- Calls return a `CallResult`, a `Result` that also carries the HTTP response:
  `status_code()`, the API's declared [response headers](#response-headers)
  via `headers()`, and all of them via `raw_headers()`. The response may come
  from the server or from a proxy or rate limiter in front of it; all three
  are `undefined` for network failures and aborts.
- Failed calls hold an `Err`. `err.err()` holds the endpoint's typed error
  (non-5xx responses with a JSON body); anything else, such as a 5xx, a
  non-JSON body or a network failure, is in `err.other_err()`. `Err` has the
  same `status_code()`, `headers()` and `raw_headers()`. `Result.unwrap_ok()` throws an
  `Error` whose `cause` is the `Err`, so code that only sees the thrown error,
  such as a query library's retry callback, can still classify it:

  ```ts
  retry: (failureCount, error) => {
    if (failureCount >= 2 || !(error.cause instanceof Err)) return false;
    const status = error.cause.status_code();
    // no response at all, rate limited, or upstream unavailable
    return status === undefined || status === 429 || status >= 502;
  }
  ```

### Python

- Generates Pydantic-based models and client code.
- Generates an async client by default.
- Adds a sync client only when `--python-sync` is passed.
- Emits reflected Rust namespaces as real Python packages. `generated.py` is kept
  as a temporary compatibility facade inside the package.
- Each namespace exposes its types under short, ergonomic names (`order.Item`).
  When a namespace defines a type whose short name clashes with a top-level type
  of the same name (e.g. both a root `IfConflictOnUpdate` and a
  `nomatches::IfConflictOnUpdate`), the namespace keeps the short name bound to
  the imported top-level type and exposes its own type under a disambiguated,
  namespace-prefixed name (`nomatches.NomatchesIfConflictOnUpdate`). This keeps
  one Python class per logical type so `model.<X>` resolves consistently.
- Uses `reflectapi_runtime` for client base classes and runtime helpers.

### Rust

- Generates typed async client methods.
- Integrates with `reflectapi::rt::Client`. The transport carries the
  base URL (`Client::base_url`); the per-request `Request` DTO carries
  only `path`, `headers`, and `body` — same shape as TypeScript and
  Python.
- Built-in transports: `reflectapi::rt::ReqwestClient` (a thin wrapper
  around `reqwest::Client` + base URL) and the type alias
  `ReqwestMiddlewareClient` for `reqwest_middleware::ClientWithMiddleware`.
- Generated `Interface<C>` exposes:
    - `Interface::new(client: C)` — generic, takes any `Client` impl.
    - `Interface::try_new(reqwest::Client, base_url) -> Result<Self, UrlParseError>` —
      convenience constructor that hides the `ReqwestClient` adapter for
      the most common case. Available when the generated crate enables
      its own `reqwest` feature (which should re-export
      `reflectapi/reqwest`).
- Supports optional tracing instrumentation through `--instrument`.
- Generates serde-compatible types and request helpers for JSON-based transport.

## Streaming Endpoints

Endpoints registered with `Builder::stream_route` produce a stream of items
rather than a single response. The wire format is Server-Sent Events: each
item is sent as a `data: <json>\n\n` event. Errors raised before the stream
opens are returned as a normal HTTP 4xx/5xx response, not as SSE events; the
server does not emit heartbeats or end-of-stream markers, so streams end
when the connection closes.

| Output | Streaming client surface |
|--------|--------------------------|
| TypeScript | Method returns `Promise<Result<AsyncIterable<Item>, Err<Error>>>`; consume with `for await`. |
| Rust | Method returns `reflectapi::rt::StreamResponse<Item, AppError, NetError>`. The outer `Result` reports init failures (application or network); inner items report per-item transport/decode failures only — application errors cannot occur after the stream is open. Requires the `rt-sse` Cargo feature on the `reflectapi` dependency. |
| Python | Method returns `AsyncIterator[Item]` on the async client and `Iterator[Item]` on the sync client. Init 4xx/5xx raise `ApplicationError` (with the typed `error_model` if declared); per-event problems raise `NetworkError` / `TimeoutError` / `ValidationError` and terminate the iterator without a leaked socket. |
| OpenAPI | Operation is described with `text/event-stream` response content. |

## Response Headers

Some response headers matter to callers: a request ID to quote in a support
ticket, or `retry-after` from a rate limiter. Declare them on the builder as a
struct with one `Option<String>` field per header; the field's serde name is
the header name and must be lowercase:

```rust,ignore
#[derive(serde::Serialize, reflectapi::Output)]
struct ResponseHeaders {
    /// Request ID to quote when reporting a problem
    #[serde(rename = "x-request-id")]
    request_id: Option<String>,
    /// Seconds, or an HTTP date, after which to retry
    #[serde(rename = "retry-after")]
    retry_after: Option<String>,
}

let builder = reflectapi::Builder::new()
    .response_headers::<ResponseHeaders>() // applies to routes added after this
    .route(handler, |b| b.name("pets.list"));
```

The headers apply to successful and failed responses alike. Declaring a header
doesn't mean the server sends it: it may be added by infrastructure in front of
the server, and clients read it from whatever response arrived. Use
`RouteBuilder::response_headers` to declare a different set for one route.

| Output | Response headers |
|--------|------------------|
| TypeScript | `headers()` on the `CallResult` and on its `Err` returns the declared headers, typed: each is `string \| null`, `null` when that header is absent; `headers()` itself is `undefined` when no response arrived. `raw_headers()` returns all response headers, untyped, for ones the API doesn't declare, such as `cf-ray` from a CDN. |
| Python | `ApiResponse.headers` and `ApiError.headers` hold the declared headers as the generated model (each field `None` when absent); `None` when the API declares none or no response arrived. All headers, untyped, are in `.metadata.headers`. Streaming methods expose them on errors only. |
| Rust | The headers struct is generated as a type, but the client doesn't populate it yet. |
| OpenAPI | Documented as optional headers on the operation's `200` and `default` responses. |

## Shared Characteristics

The generated clients all aim to provide:

- Types derived from the Rust-reflected schema
- Function wrappers with generated documentation
- Structured handling of application errors versus transport/protocol failures
- Good IDE support through generated type information

They do not currently all provide the same runtime validation guarantees or the same runtime transport abstractions, so those details should be considered language-specific.
