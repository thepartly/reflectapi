# Attributes Reference

ReflectAPI provides `#[reflectapi(...)]` attributes that control how Rust types are reflected into the schema and generated clients.

## Struct / Enum Level

| Attribute | Description |
|-----------|-------------|
| `#[reflectapi(derive(...))]` | Forward additional derive traits to the generated Rust client type. |
| `#[reflectapi(open)]` | Make a tagged enum open in generated responses: retain unfamiliar tags and raw JSON in an explicit unknown case. Request types stay closed. |

### Enum evolution

An enum marked `#[reflectapi(open)]` tells generated clients to accept future
response variants while preserving their content:

```rust,ignore
#[derive(serde::Serialize, reflectapi::Output)]
#[serde(tag = "kind")]
#[reflectapi(open)]
enum PetKind {
    Dog { breed: String },
    Bird,
}
```

This response-only type needs `Serialize` to send values and `Output` to
describe them in the schema. Request-only types use `Deserialize` and `Input`.
Add all four derives only when the same type is used in both directions; an
`open` enum's generated request type still stays closed.

Serde controls the wire representation (`kind` in this example). ReflectAPI's
`open` attribute controls how generated clients handle future response tags.

A response such as `{"kind":"Fox","nocturnal":true}` becomes an explicit
unknown value. A known `Dog` missing `breed` still fails decoding. Rust clients
get an `Unknown { tag, raw }` variant (with a suffix if that variant name is
already taken). Python clients get an unknown model with `.tag` and `.raw`.
TypeScript clients get a branded `UnknownEnum`, detected with `isUnknownEnum`;
its tag and raw value are available under the exported `unknownEnum` symbol.
The same policy applies to nested models, application errors, and SSE items.
See [handling unfamiliar enum variants](../clients/README.md#handling-unfamiliar-enum-variants)
for client-side examples.
Rust retains Serde's surrounding container rules: an optional flattened enum
can become `None` when its contents fail validation. Use a required flattened
field when invalid content must produce a decoding error.

Open enums support external string/single-key-object tags, internal tags, and
adjacent tags. Untagged enums and untagged variants are rejected because an
overlapping future shape cannot be reliably distinguished from a known shape.
Numeric `#[reflectapi(discriminant)]` enums and output type overrides cannot be
combined with `open`.
Directly flattened open enums must use internal or adjacent tags. Codegen
rejects flattened external open enums because a future variant key cannot be
distinguished from a newly added parent field. An external open enum used as a
regular field inside a flattened struct remains supported.
An object with a flattened open or declared-fallback enum cannot flatten
another enum. Use named fields for additional enums so each unknown case has
its own discriminator, payload, and client representation.

This attribute changes client decoding, not server Serde behavior. Requests
use separate closed generated types, and the current wire variants in OpenAPI
are unchanged. Existing SDKs need regeneration to acquire the new policy;
enabling it does not change already distributed SDKs.

Without `open`, a declared `#[serde(other)]` unit variant is preserved in the
generated response clients. An unfamiliar valid tag maps to that unit variant,
discarding its original tag and content. This is supported for internally and
adjacently tagged enums; adjacent unit variants accept only absent or null
content, matching Serde. Missing/non-string tags and malformed known variants
remain errors for ordinary enum fields. A directly flattened internal fallback
also preserves Serde's integer variant identifiers: an index selects the
declared variant, or the fallback when it is out of range. Open enums always
require string tags.

Declaring both `open` and `#[serde(other)]` is rejected. Choose the existing
unit fallback when unfamiliar tags should map to that declared variant, or
`open` when clients should retain the unfamiliar tag and raw JSON. Neither
policy widens generated request types.

## Field Level

### Type Override

| Attribute | Description |
|-----------|-------------|
| `#[reflectapi(type = "T")]` | Override the reflected type for both input and output schemas. |
| `#[reflectapi(input_type = "T")]` | Override the reflected type for the input schema only. |
| `#[reflectapi(output_type = "T")]` | Override the reflected type for the output schema only. |

### Transform

| Attribute | Description |
|-----------|-------------|
| `#[reflectapi(transform = "path::to::fn")]` | Apply a field transformation callback for both schemas. Signature: `fn(&mut Field, &Typespace)`. |
| `#[reflectapi(input_transform = "path::to::fn")]` | Apply a field transformation callback for input only. |
| `#[reflectapi(output_transform = "path::to::fn")]` | Apply a field transformation callback for output only. |

The transform callback receives `&mut Field` and can modify any field metadata — type, `required`, `hidden`, etc. It runs after type references are resolved.

ReflectAPI ships with built-in transforms in the `reflectapi::transforms` module:
- `reflectapi::transforms::fallback_recursively` — unwraps transparent wrappers (e.g. `Arc<T>` → `T`)
- `reflectapi::transforms::make_required` — makes a field required: `Option<T>` → required `T`, `reflectapi::Option<T>` → required `Option<T>` (nullable but not omittable). For non-Option types, marks the field as required without changing the type.
- `reflectapi::transforms::make_nonnullable` — makes a field non-nullable: `Option<T>` → `T`, `reflectapi::Option<T>` → `T` (keeps optionality, removes nullability). No-op for non-Option types.
- `reflectapi::transforms::make_required_and_nonnullable` — makes a field both required and non-nullable: `Option<T>` → required `T`, `reflectapi::Option<T>` → required `T`. For non-Option types, marks the field as required without changing the type.

> **Note:** In reflectapi, "required" and "non-nullable" are distinct concepts. A field can be:
> - Required + non-nullable (`T`, required) — must be present, cannot be null
> - Required + nullable (`Option<T>`, required) — must be present, can be null
> - Optional + nullable (`reflectapi::Option<T>`) — can be omitted or null
> - Optional + non-nullable (`T`, not required) — can be omitted, but if present cannot be null
>
> `make_required` removes the "can be omitted" (undefined) state. `make_nonnullable` removes the "can be null" state. `make_required_and_nonnullable` removes both.

**Example: Controlling requiredness and nullability in the schema**

A field may use an Option wrapper at the Rust level (e.g., because a middleware populates it before your handler runs) but you want generated clients to see a different contract.

```rust,ignore
#[derive(serde::Deserialize, reflectapi::Input)]
struct MyRequest {
    /// In Rust this is `Option<String>` (populated by middleware),
    /// but clients see it as a required `String` field.
    #[serde(default)]
    #[reflectapi(input_transform = "reflectapi::transforms::make_required")]
    pub tenant_id: Option<String>,
}

#[derive(serde::Deserialize, reflectapi::Input)]
struct PatchRequest {
    /// In Rust this is `reflectapi::Option<String>` (populated by middleware),
    /// but clients see it as a required `Option<String>` (nullable, not omittable).
    #[serde(default)]
    #[reflectapi(input_transform = "reflectapi::transforms::make_required")]
    pub correlation_id: reflectapi::Option<String>,

    /// In Rust this is `reflectapi::Option<String>`,
    /// but clients see it as an optional `String` (can be omitted, but not null).
    #[serde(default)]
    #[reflectapi(input_transform = "reflectapi::transforms::make_nonnullable")]
    pub display_name: reflectapi::Option<String>,

    /// In Rust this is `reflectapi::Option<String>`,
    /// but clients see it as a required `String` (must be present, cannot be null).
    #[serde(default)]
    #[reflectapi(input_transform = "reflectapi::transforms::make_required_and_nonnullable")]
    pub user_id: reflectapi::Option<String>,
}
```

### Visibility

| Attribute | Description |
|-----------|-------------|
| `#[reflectapi(skip)]` | Exclude the field entirely from the schema. The field's type does not need to implement `Input`/`Output`. Equivalent to setting both `input_skip` and `output_skip`. |
| `#[reflectapi(input_skip)]` | Exclude the field from the input schema only. |
| `#[reflectapi(output_skip)]` | Exclude the field from the output schema only. |
| `#[reflectapi(hidden)]` | Keep the field in the schema (marked `"hidden": true`) but exclude it from generated clients, documentation, and OpenAPI specs. Useful for header fields that the server needs at runtime but clients should not see. |

### `skip` vs `hidden`

Both attributes remove a field from generated clients. The key difference:

- **`skip`** removes the field from the schema entirely. The field's type is never reflected, so it does not need to implement `Input` or `Output`. Use this for internal bookkeeping fields whose types are not part of your API.

- **`hidden`** keeps the field in the schema JSON (marked with `"hidden": true`) but excludes it from generated clients, documentation, and OpenAPI specs. The type must still implement the relevant trait. Use this for fields that are functionally required by server-side infrastructure — for example, a middleware or a proxy layer that populates the field / header before deserialization — but should not appear in client interfaces.

**When to use `hidden` over `skip`:** The field stays in the schema JSON so that server-side tooling (the axum adapter, middleware, or custom infrastructure) can inspect the full type structure at runtime. If nothing on the server needs the field's schema metadata, prefer `skip`.

Neither `skip` nor `hidden` affects serde serialization. For output types, serde will still serialize a hidden field onto the wire — `hidden` only controls what generated code and documentation show. If a field must never appear in responses, use `#[serde(skip_serializing)]` instead.

Please not that neither `skip` nor `hidden` prevent a malicious client from sending the fields in a request. A middleware may overwrite it or reject or validate as needed. It is up to the specific implementation of your server.

**Example: hidden header field**

```rust,ignore
#[derive(serde::Deserialize, reflectapi::Input)]
pub struct MyHeaders {
    /// Visible to clients — they must provide this
    pub authorization: String,

    /// Not visible to the generated clients and documentation
    /// Expected to be populated by a proxy or server-side middleware.
    #[reflectapi(hidden)]
    #[serde(default)]
    pub x_internal_request_id: String,
}
```

### `#[serde(default)]` on skipped and hidden fields

When a field is excluded from generated clients (via `skip`, `input_skip`, or `hidden`), clients will not send it. Whether you add `#[serde(default)]` is your choice and depends on your deployment:

- **With `#[serde(default)]`:** If the field is absent, serde fills the default value. The request succeeds even if no proxy or middleware populates the field. Use this for optional metadata (trace IDs, correlation IDs) where absence is acceptable.

- **Without `#[serde(default)]`:** If the field is absent, deserialization fails with a protocol error. Use this for fields that a proxy or middleware is expected to inject — a missing value means the infrastructure is misconfigured, and you want to reject the request loudly rather than proceed silently with a zero-value.

### Deprecation

| Attribute | Description |
|-----------|-------------|
| `#[reflectapi(deprecated_since = "YYYY-MM-DD")]` | Date the field was deprecated. Requires `#[deprecated]`. |
| `#[reflectapi(sunset = "YYYY-MM-DD")]` | Date after which the field may be removed. Requires `#[deprecated]`. |

The release in Rust's `#[deprecated(since = "..")]` is kept as well. See [Deprecations](./deprecations.md).

### Restrictions

`#[reflectapi(hidden)]` cannot be used on unnamed (tuple) struct or enum variant fields. Hiding a positional element would shift indices in generated clients, breaking wire compatibility. Use `hidden` only on named fields.

## Enum Variant Level

| Attribute | Description |
|-----------|-------------|
| `#[reflectapi(skip)]` | Exclude the variant from the schema entirely. |
| `#[reflectapi(input_skip)]` | Exclude the variant from the input schema only. |
| `#[reflectapi(output_skip)]` | Exclude the variant from the output schema only. |
| `#[reflectapi(deprecated_since = "YYYY-MM-DD")]` | Date the variant was deprecated. Requires `#[deprecated]`. |
| `#[reflectapi(sunset = "YYYY-MM-DD")]` | Date after which the variant may be removed. Requires `#[deprecated]`. |
