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
- Response enums with `#[reflectapi(open)]` include a branded `UnknownEnum`.
  `isUnknownEnum(value)` narrows it; `value[unknownEnum]` holds `tag` and `raw`
  without occupying any wire field names. Generated `match`
  calls handle it with the exported `[unknownEnum]` handler key or an `otherwise`
  fallback. Known values keep their wire shape. Runtime decoding is enabled
  for responses/errors that reach an open enum or `#[serde(other)]`, including
  nested values and SSE items.
- Uses a `fetch`-based default client implementation.
- Ordinary response paths parse JSON without runtime schema validation. Paths
  that reach an open enum or declared fallback use generated decoders.
  These check known payload shapes against the generated TypeScript types;
  they do not validate every Rust-specific constraint, such as UUID syntax
  for a field represented as a TypeScript string. A known tag is never
  reclassified as an unknown case when its payload fails those checks.
- Supports custom client implementations via the generated client interface.
  A custom transport's `Headers.get(name)` must match names case-insensitively,
  as `fetch`'s does: declared response headers are looked up by their
  lowercase names.
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
- Open response enum wrappers include a generated unknown model with `.tag`
  and `.raw`. Unknown values serialize to their original wire JSON. Known
  values still validate normally; `#[serde(other)]` without `open` maps to the
  declared unit fallback instead.

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
- Open response enums have a generated `Unknown { tag, raw }` variant and
  tag-first deserialization. Ordinary `#[serde(other)]` output variants keep
  that attribute. Unknown open error variants serialize to their original
  JSON for diagnostics.

## Handling Unfamiliar Enum Variants

An enum declared with `#[reflectapi(open)]` keeps known cases typed and adds an
explicit unknown case to generated responses. Handle it before reading known
variant fields. The following snippets assume the generated `PetKind` and
related types are imported from their generated namespace.

Rust uses a normal enum match:

```rust,ignore
match pet_kind {
    PetKind::Dog { breed } => println!("Dog: {breed}"),
    PetKind::Bird => println!("Bird"),
    PetKind::Unknown { tag, raw } => println!("New kind {tag}: {raw}"),
}
```

Python exposes the variant through the generated model's `.root`:

```python
variant = pet_kind.root
if isinstance(variant, PetKindUnknown):
    print(variant.tag, variant.raw)
elif isinstance(variant, PetKindDog):
    print(variant.breed)
else:
    print("Bird")
```

TypeScript narrows the union with the helper exported from `generated.ts`:

```typescript
import { isUnknownEnum, unknownEnum } from "./generated";

if (isUnknownEnum(petKind)) {
  console.log(petKind[unknownEnum].tag, petKind[unknownEnum].raw);
} else if (petKind.kind === "Dog") {
  console.log(petKind.breed);
} else {
  console.log("Bird");
}
```

Generated unknown names get a suffix when they would conflict with a declared
variant or another generated type. For externally tagged and unit enum unions
in TypeScript, `match` accepts an unknown handler under the exported
`[unknownEnum]` symbol; it is separate from a real wire variant named `Unknown`.
That handler receives `{ tag, raw }` directly. An `otherwise` handler receives
the original value and can use `isUnknownEnum` to narrow it.

`raw` is the parsed JSON value, rather than the original response bytes.
Python and TypeScript unknown cases serialize back to their retained wire
value. Rust does the same when `serde::Serialize` is requested through
`#[reflectapi(derive(serde::Serialize))]`; responses otherwise derive only
`Deserialize` by default. TypeScript stores its metadata under non-enumerable
symbols so `JSON.stringify` keeps wire fields intact. For a directly flattened enum,
Rust and Python expose a typed enum field and retain the remaining enum JSON;
TypeScript keeps the flat object and retains the whole original object in
`value[unknownEnum].raw`.

A declared `#[serde(other)]` uses its unit fallback instead and discards the
unfamiliar tag and payload. It cannot be combined with `open`. Neither policy
adds an unknown case to generated requests. See the
[attribute reference](../getting-started/attributes.md#enum-evolution) for
representation restrictions and migration requirements.

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
| Python | Method returns `AsyncApiStream[Item]` on the async client and `ApiStream[Item]` on the sync client: iterators that also expose the response's `metadata` and declared `headers` once iteration starts. Init 4xx/5xx raise `ApplicationError` (with the typed `error_model` if declared); per-event problems raise `NetworkError` / `TimeoutError` / `ValidationError` and terminate the iterator without a leaked socket. |
| OpenAPI | Operation is described with `text/event-stream` response content. |

## Response Headers

Some response headers matter to callers: a request ID to quote in a support
ticket, or `retry-after` from a rate limiter. Declare them on the builder as a
struct with one `Option<T>` field per header. `T` must be a string on the wire,
because the header's value is deserialized into it the same way request headers
are: `String`, a unit-variant enum, a newtype over one, or a type such as
`uuid::Uuid`, `chrono::DateTime` or `url::Url`. Numbers, booleans and
collections aren't supported. The field's serde name is the header name and must
be lowercase:

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
    .response_headers::<ResponseHeaders>()
    .route(handler, |b| b.name("pets.list"));
```

The headers apply to successful and failed responses alike. Declaring a header
doesn't mean the server sends it: it may be added by infrastructure in front of
the server, and clients read it from whatever response arrived. Handlers can't
set response headers yet, so today these come from that infrastructure.

A builder's declaration applies to every route of the builder that doesn't
declare its own, whether added before or after the call, including the routes
of builders merged in with `nest` or `extend` that have no declaration of their
own. Routes that already declare response headers keep them. Use
`RouteBuilder::response_headers` to declare a different set for one route. As
with routes, call `rename_types` after declaring response headers so the rename
applies to the headers type.

A header that appears more than once is read as its values joined with `, `.
`set-cookie` can't be read that way (cookie dates contain commas, and browsers
don't expose it to scripts), so don't declare it.

In TypeScript, the declared headers are typed on the `CallResult` and `Err`
returned by a call. Code that only sees a thrown error, such as a query
library's retry callback, gets the `Err` as `cause` with its headers type
erased, so cast it to the generated headers type:

```ts
retryDelay: (failureCount, error) => {
  const headers =
    error.cause instanceof Err
      ? (error.cause as Err<unknown, ResponseHeaders>).headers()
      : undefined;
  const seconds = headers?.["retry-after"] ? Number(headers["retry-after"]) : NaN;
  // Retry-After may also be an HTTP date; fall back to exponential backoff
  return Number.isFinite(seconds)
    ? seconds * 1000
    : Math.min(1000 * 2 ** failureCount, 30_000);
}
```

| Output | Response headers |
|--------|------------------|
| TypeScript | `headers()` on the `CallResult` and on its `Err` returns the declared headers: each has the field's generated type or is `null` when that header is absent; `headers()` itself is `undefined` when no response arrived. Open enum headers decode unfamiliar strings into `UnknownEnum`; ordinary enum headers retain the existing unvalidated string behavior. `raw_headers()` returns all response headers, untyped, for ones the API doesn't declare, such as `cf-ray` from a CDN. `result.map(...)` keeps the response; `map_err` returns a plain `Result`. |
| Python | `ApiResponse.headers` and `ApiError.headers` (including `ValidationError` when the response body is invalid) hold the declared headers validated into the generated model; a field is `None` when its header is absent or its value doesn't validate. `headers` is `None` when the API declares none or no response arrived. All headers, untyped, are in `.metadata.headers`. Streaming methods return an `ApiStream` / `AsyncApiStream` whose `headers` and `metadata` are set once iteration starts. |
| Rust | The headers struct is generated as a type, but the client doesn't populate it yet. The generated method signatures of routes with declared headers will change when it does. |
| OpenAPI | Documented as optional headers on the operation's `200` and `default` responses. |

## Shared Characteristics

The generated clients all aim to provide:

- Types derived from the Rust-reflected schema
- Function wrappers with generated documentation
- Structured handling of application errors versus transport/protocol failures
- Good IDE support through generated type information

They do not currently all provide the same runtime validation guarantees or the same runtime transport abstractions, so those details should be considered language-specific.
