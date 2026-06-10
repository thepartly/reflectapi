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

- **`hidden`** keeps the field in the schema with a `hidden` flag. The type must still implement the relevant trait. At runtime, the axum adapter will still extract and deserialize the field (e.g. for headers). Use this for fields that are functionally required on the server side but should not appear in client interfaces or documentation.

**Example: hidden header field**

```rust,ignore
#[derive(serde::Deserialize, reflectapi::Input)]
pub struct MyHeaders {
    /// Visible to clients — they must provide this
    pub authorization: String,

    /// Server-internal: extracted by axum but not exposed to clients
    #[reflectapi(hidden)]
    #[serde(default)]
    pub x_internal_request_id: String,
}
```

## Enum Variant Level

| Attribute | Description |
|-----------|-------------|
| `#[reflectapi(skip)]` | Exclude the variant from the schema entirely. |
| `#[reflectapi(input_skip)]` | Exclude the variant from the input schema only. |
| `#[reflectapi(output_skip)]` | Exclude the variant from the output schema only. |
