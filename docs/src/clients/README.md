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

## Required Headers

Some deployments need every request to carry a header the service itself
never declares — an API key checked by a gateway, a tenant id read by a
routing layer. Because no handler takes them, they are absent from the
schema, and nothing stops a caller from forgetting them.

`--required-headers` names those headers at generation time. The
generated client then cannot be constructed without them and sends them
on every request, as middleware around the transport:

```bash
cargo run --bin reflectapi -- codegen \
    --language typescript \
    --schema reflectapi.json \
    --output clients/typescript \
    --required-headers x-api-key,x-tenant-id
```

Header names are matched case-insensitively and lowercased in the
generated code; a name that is not a valid HTTP field name is rejected.
A header supplied for an individual call always wins over the
client-level value.

| Output | Surface |
|--------|---------|
| TypeScript | `client(base, required_headers)` takes a `RequiredHeaders` object; the transport is wrapped in `__with_required_headers`. |
| Rust | `Interface::new(client, RequiredHeaders::new(..))` returns `Interface<WithRequiredHeaders<C>>`. `RequiredHeaders::new` takes one `reflectapi::rt::HeaderValue` per header, in declaration order; the fields are public too. |
| Python | `Client(base_url, *, x_api_key=...)` — one keyword-only argument per header, installed as a `SyncRequiredHeadersMiddleware` / `AsyncRequiredHeadersMiddleware` so they survive a caller-supplied transport. |
| OpenAPI | Each operation gains a required `in: header` parameter. A header a handler already declares is left as the handler declared it. |

The equivalent library-level option is `required_headers` on each
language's codegen `Config`.

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
      Takes a second `RequiredHeaders` argument when the client was
      generated with `--required-headers`.
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
