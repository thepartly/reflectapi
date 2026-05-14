# Changelog

## 0.17.3

Promotes the 0.17.3-alpha.1 work to stable and adds one OpenAPI codegen fix on top:

- **Root-level `Box<T>` (and other transparent pointer wrappers) now resolve in OpenAPI output.** A function taking or returning `Box<TreeNode>` previously emitted `$ref: #/components/schemas/Box<TreeNode>` — a name nothing in the spec registered. The converter now unwraps transparent pointer primitives (`Box`, `Rc`, `Arc`, `Cow`, ...) at the type-reference level so the ref points at the real component. Primitives that carry their own OpenAPI representation (`chrono::DateTime`, `HashSet<T>`, `usize`, ...) are left untouched.
- **Doc descriptions are scrubbed of embedded `<details><summary>JSON schema</summary>` blocks.** These are useful as human-readable annotations in Rust source but leaked into `info`, `paths`, and component `description` fields in the generated spec.

See the alpha.1 notes below for the rest of the 0.17.3 changes (Python `ReflectapiPartialModel`, TS SSE flush, etc.).

## 0.17.3-alpha.1

### Python — partial fields without a wrapper class

The hand-rolled `ReflectapiOption[T]` runtime class is gone. Generated clients now express `reflectapi::Option<T>` fields as plain `T | None` on a `ReflectapiPartialModel` base class, with the absent-vs-null distinction carried by Pydantic's `model_fields_set`:

```python
# Field omitted from the wire if you don't pass it
Item(name="rex").model_dump_json()             == '{"name":"rex"}'

# Null on the wire if you pass None explicitly
Item(name="rex", kind=None).model_dump_json()  == '{"name":"rex","kind":null}'

# "Was this field on the wire?" — analogue of TypeScript `obj.kind !== undefined`
"kind" in item.model_fields_set
```

Migration for hand-written consumer code that touched the wrapper API:

| Before | After |
|---|---|
| `ReflectapiOption(Undefined)` | don't set the field |
| `ReflectapiOption(None)` | `field=None` |
| `ReflectapiOption(value)` | `field=value` |
| `opt.is_undefined` | `"field" not in model.model_fields_set` |
| `opt.is_none` | `model.field is None and "field" in model.model_fields_set` |
| `opt.unwrap_or(default)` | `model.field if "field" in model.model_fields_set else default` |

The runtime drops `ReflectapiOption`, `Undefined`, `Option`, `some`, `none`, `undefined`, and `serialize_option_dict`. `ReflectapiDuration` is added as the wire-format adapter for `std::time::Duration` (`{secs, nanos}` ↔ `timedelta`).

Other Python codegen fixes in the same pass:

- `Vec<u8>` renders as `list[int]` to match serde's default JSON shape (was `bytes`, which Pydantic rejected against the wire form).
- `PhantomData<T>` fields are dropped from generated models (Rust-only type marker, no wire data).
- `std::tuple::TupleN` types render as Python `tuple[…]`.
- Field names that exact-match Pydantic methods (`model_dump`, `schema`, `json`, …) are renamed with a trailing underscore and aliased to the wire name; `model_*` field prefixes work without warnings via `protected_namespaces=()`.
- `_rebuild_models()` raises a structured `RuntimeError` listing every model whose annotations don't resolve, instead of silently swallowing the failures.

### TypeScript — SSE final-event flush

The generated `__EventSourceParserStream` gains a `flush()` that feeds `'\n\n'` to the parser on stream close. Without it, a server that closes the connection immediately after the final `data:` line — without the spec's trailing blank-line terminator — silently dropped the last event. Regenerated clients pick up the fix.

### Rust

No generated-code changes.

### Internals

- OpenAPI codegen now tracks in-progress conversions by full monomorphization, so generic self-referential types no longer recurse forever.
- `reflectapi-derive` reads `#[doc]` literals via `LitStr::value()` so doc comments with literal quotes land in the schema without source-level `\"` escaping.

## 0.17.2 — transport-shape refactor

The Rust, TypeScript, and Python clients now share a single `Request` DTO shape: `{ path, headers, body }`. The base URL lives on the transport, not on the generated `Interface`.

### Rust

The trait gained a `base_url` method and `Request::url` was renamed to `Request::path`:

```rust
pub trait Client {
    type Error;
    fn base_url(&self) -> &Url;
    fn request(
        &self,
        request: Request,
    ) -> impl Future<Output = Result<Response<Self::Error>, Self::Error>>;
}

pub struct Request {
    pub path: String,        // was: url: Url
    pub headers: HeaderMap,
    pub body: Bytes,
}
```

`reqwest::Client` no longer implements `Client` directly — it doesn't carry a base URL. Pick the path that matches your setup:

**Plain `reqwest::Client`, generated crate has a `reqwest` feature.** The generated `Interface::try_new(reqwest::Client, base_url)` convenience constructor keeps working unchanged:

```rust
let api = MyClient::try_new(reqwest::Client::new(), base_url)?;
```

The constructor is gated on the generated crate enabling its own `reqwest` feature (which should re-export `reflectapi/reqwest`). If you don't see `try_new`, this is why.

**`reqwest_middleware::ClientWithMiddleware` (the common path with otel/retry/etc.).** Compose the transport explicitly — there's no convenience constructor for this case because of method-resolution ambiguity:

```rust
let transport = reflectapi::rt::ReqwestMiddlewareClient::try_new(
    mw_client,
    base_url,
)?;
let api = MyClient::new(transport);
```

`ReqwestMiddlewareClient` is a type alias for `ReqwestClient<reqwest_middleware::ClientWithMiddleware>`.

**Custom transport.** Wrap with `ReqwestClient::try_new(...)` (or implement the `Client` trait yourself) and pass the result to `MyClient::new`:

```rust
let api = MyClient::new(
    reflectapi::rt::ReqwestClient::try_new(reqwest::Client::new(), base_url)?
);
```

`ReqwestClient::try_new` returns `Result<Self, reflectapi::rt::UrlParseError>` (a re-export of `url::ParseError`). It rejects URLs that can't have a base (`data:`, `mailto:`, etc.) via `Url::cannot_be_a_base`. There is no infallible `ReqwestClient::new` constructor.

#### `reqwest-middleware` feature implies `reqwest` (alpha.4)

In 0.17.2-alpha.3 enabling only `reqwest-middleware` failed to compile because the wrapper struct lives behind the `reqwest` feature. Fixed in 0.17.2-alpha.4: `reqwest-middleware` now implies `reqwest`. Consumers that already enable both don't need to change anything.

### TypeScript (alpha.4)

Codegen now emits two files: `generated.ts` (the API surface) and `generated.transport.ts` (the transport contract). Most consumers only need `generated.ts`. Custom transports import from the transport submodule:

```ts
import type { Client, Request, Response } from './generated.transport';
```

The bare `Request` / `Response` / `Headers` names live behind the `./generated.transport` import path, so they no longer shadow the DOM globals of the same name when imported from `generated.ts`. See #143 for the rationale.

If your build tooling assumed a single generated file, update it to include the new sibling. Typical pnpm/npm workflows pick it up automatically (the file sits in the same directory).

#### Library API: `typescript::generate` returns multiple files

For consumers calling the codegen library directly rather than through the `reflectapi` CLI, the signature changed:

```rust
// before
pub fn generate(schema: Schema, config: &Config) -> Result<String>;

// after
pub fn generate(schema: Schema, config: &Config) -> Result<BTreeMap<String, String>>;
```

The map is keyed by filename (`"generated.ts"`, `"generated.transport.ts"`). The CLI handles `--output <file>.ts` paths by writing the matching file at the requested path and the sibling in the parent directory; downstream callers wrapping the library should do the same.

### Python

End-user generated clients are unchanged. Authors of custom middleware or transports should target the transport-agnostic types in `reflectapi_runtime.transport` (`Request`, `Response`, `Client`, `AsyncClient`) rather than reaching for `httpx` types directly. They are also re-exported from the top-level `reflectapi_runtime` package.

#### Generic flatten correctness (alpha.4)

Previously, a Rust struct that used `serde(flatten)` over a generic parameter (e.g. `IdentityData<I, D>`, `UpdateOrElse<T, C>`, `InsertManyOrElse<T, C>`) generated a Pydantic model that silently dropped the inner type's wire fields, because the Python codegen couldn't resolve a TypeVar to a concrete struct at class-definition time. With `extra="ignore"` on the model, those fields were also discarded on parse — silent data loss for any endpoint using these patterns.

Fix: the Python codegen now monomorphizes — for each concrete `(struct, args)` instantiation it emits a specialized class with the flatten resolved against the concrete type. The mangled name is `OriginalStruct_Arg1_Arg2…`, e.g. `UpdateOrElse_Pet_Conflict`. Method signatures, namespace classes, and `model_rebuild` lists all use the mangled name consistently. Rust and TypeScript clients are unaffected — they handled this case correctly already (serde at runtime; intersection types at compile time).

The codegen also now hard-fails (rather than silently dropping fields) if it encounters an unresolved flatten target — a defence-in-depth check so any future regression is loud.
