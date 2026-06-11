# Attributes Reference

ReflectAPI provides `#[reflectapi(...)]` attributes that control how Rust types are reflected into the schema and generated clients.

## Struct / Enum Level

| Attribute | Description |
|-----------|-------------|
| `#[reflectapi(derive(...))]` | Forward additional derive traits to the generated Rust client type. |

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
| `#[reflectapi(transform = "path::to::fn")]` | Apply a type transformation callback for both schemas. |
| `#[reflectapi(input_transform = "path::to::fn")]` | Apply a type transformation callback for input only. |
| `#[reflectapi(output_transform = "path::to::fn")]` | Apply a type transformation callback for output only. |

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

### Restrictions

`#[reflectapi(hidden)]` cannot be used on unnamed (tuple) struct or enum variant fields. Hiding a positional element would shift indices in generated clients, breaking wire compatibility. Use `hidden` only on named fields.

## Enum Variant Level

| Attribute | Description |
|-----------|-------------|
| `#[reflectapi(skip)]` | Exclude the variant from the schema entirely. |
| `#[reflectapi(input_skip)]` | Exclude the variant from the input schema only. |
| `#[reflectapi(output_skip)]` | Exclude the variant from the output schema only. |
