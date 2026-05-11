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

### Python

- Generates Pydantic-based models and client code.
- Generates an async client by default.
- Adds a sync client only when `--python-sync` is passed.
- Emits reflected Rust namespaces as real Python packages. `generated.py` is kept
  as a temporary compatibility facade inside the package.
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

## Shared Characteristics

The generated clients all aim to provide:

- Types derived from the Rust-reflected schema
- Function wrappers with generated documentation
- Structured handling of application errors versus transport/protocol failures
- Good IDE support through generated type information

They do not currently all provide the same runtime validation guarantees or the same runtime transport abstractions, so those details should be considered language-specific.
