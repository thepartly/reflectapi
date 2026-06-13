# Changelog

## Unreleased

- **Security:** instrumented Rust clients (`--instrument`) no longer record request bodies and headers as tracing span fields. The generated `#[tracing::instrument]` attribute now uses `skip_all`, so credentials in request payloads (passwords, tokens) can no longer reach logs in cleartext via their `Debug` output. Spans are still named after the endpoint path. Regenerate clients to pick this up.

## 0.17.6

- New `#[reflectapi(hidden)]` field attribute: the field stays in the schema (marked `"hidden": true`) and remains functional at runtime (e.g. header extraction by the axum adapter), but is excluded from generated clients, documentation, and OpenAPI specs. Not allowed on unnamed (tuple) fields, since removing a positional element would shift indices and break wire compatibility.
- Python codegen now emits nullable `Option<T>` fields as optional keys (`T | None = None`) in the per-variant models generated for structs with a flattened internally-tagged enum, matching the standalone-struct behavior. Previously such fields were required keys, so deserializing a valid payload that omitted the key (serde drops `None` values) raised `pydantic.ValidationError: Field required`. Doubled `str | None | None` annotations in the same paths are also fixed.
- Python codegen is now snapshot-tested by the same builder test samples as the other language backends.
- Python package codegen now emits sibling submodule imports in dependency order, so Python 3.14/Pydantic can import generated packages where one sibling model annotation references another sibling namespace.
- Python codegen now formats with a bundled Ruff wasm formatter, so generated Python output is deterministic even when `ruff` is not installed locally.
- Rust codegen snapshot typechecking now handles macOS proc-macro library names instead of assuming Linux-style `.so` files.

## 0.17.5

Python client generation improvements:

- Python codegen still falls back to basic formatting when Ruff is not installed, but now reports a clear error — including the formatter's own exit status and output — if `ruff format` is available and fails; pass `--format=false` to skip external formatting.
- Multi-field Rust tuple structs now generate valid Python `RootModel[tuple[...]]` models instead of invalid numeric field names.
- Generated Python clients with nested namespaces now import cleanly when models refer to parent or sibling namespaces, including cases like `offer_rules.InsurerCategory`.
- Namespace names containing characters that are not valid in Python identifiers, such as dashes or leading digits, now generate valid Python classes.
- Fixed generated clients silently using the wrong class when a sub-namespace defined a type whose name matched a top-level type (e.g. a top-level `IfConflictOnUpdate` and a `nomatches::IfConflictOnUpdate`). The two were emitted as separate classes sharing one name, so a value built with one was rejected by the other at runtime with a confusingly self-identical pydantic validation error. Each logical type now resolves to a single class: when names would clash, the sub-namespace type is exposed under a namespace-qualified name (e.g. `nomatches.NomatchesIfConflictOnUpdate`) while the short name (`nomatches.IfConflictOnUpdate`) refers to the top-level type.
- Schemas with a root namespace named `sys` no longer conflict with Python's standard `sys` module.
- Re-running `reflectapi codegen` into an existing output directory now removes generated files from older schemas while preserving hand-written files.

## 0.17.4

Python client codegen fixes:

- Doc comments with literal quotes are emitted without source-level escape sequences in generated Python docstrings.
- Referenced child namespaces are imported and rebound before generated model classes are defined, so nested namespace aliases resolve consistently during Pydantic model rebuilds.
- Generated Python namespace/model snapshots now cover deferred rebuild ordering and nested namespace references.

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
