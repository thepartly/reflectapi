# Deprecations

Mark a route, field or enum variant deprecated to tell client authors to move off it. Optionally say when it was deprecated and the date after which it may be removed (its sunset). Generated clients flag uses of deprecated items, and the OpenAPI spec carries the dates for tooling such as breaking-change checkers.

## Declaring

Fields and variants use Rust's `#[deprecated]`, with dates in `#[reflectapi(...)]`:

```rust,ignore
#[derive(serde::Deserialize, reflectapi::Input)]
pub struct DrawRequest {
    #[deprecated(note = "Use `shape` instead.")]
    #[reflectapi(deprecated_since = "2026-10-01", sunset = "2026-11-01")]
    pub radius: Option<f64>,
    pub shape: Shape,
}

#[derive(serde::Deserialize, reflectapi::Input)]
pub enum Shape {
    Circle { radius: f64 },
    #[deprecated(note = "Use `Circle` instead.")]
    #[reflectapi(sunset = "2026-11-01")]
    Ellipse { a: f64, b: f64 },
}
```

Routes are deprecated on the builder:

```rust,ignore
builder.route(draw_v1, |b| {
    b.name("shapes.draw_v1")
        .deprecation_note("Use shapes.draw instead.")
        .deprecated_since("2026-10-01")
        .sunset("2026-11-01")
})
```

Dates are `YYYY-MM-DD`, and the sunset can't be before the deprecation date. The derive macros reject a bad date, or a date on an item without `#[deprecated]`, at compile time; `Builder::build` rejects bad route dates.

A date in `#[deprecated(since = "2026-10-01")]` is read as well, but clippy denies dates there by default (`clippy::deprecated_semver`), so prefer `deprecated_since`. A `since` that isn't a date, such as a crate version, is left out of the schema.

## What clients see

Each target renders one message built from the dates and the note, for example `Deprecated since 2026-10-01; removal after 2026-11-01. Use shapes.draw instead.` Without dates, the message is the note.

| Target | Routes | Fields | Variants |
|--------|--------|--------|----------|
| OpenAPI | `deprecated: true`, `x-deprecated-since`, `x-sunset` on the operation | the same on the property, header parameter or response header | the same on the variant's schema |
| Rust | `#[deprecated(note = "...")]` on the method | on the field | on the variant |
| TypeScript | `@deprecated ...` on the method | on the property | on the union member, and on the tag key of an externally tagged variant with fields |
| Python | `@typing_extensions.deprecated(...)` on the method, plus a `DeprecationWarning` when called | `Field(deprecated=...)`, so Pydantic warns when the field is read | not marked |

A marker doesn't always produce an editor or type-checker warning:

- **TypeScript:** editors flag a deprecated method or property. A variant is flagged only when it is externally tagged and has fields, through its tag key: `value.Ellipse` is struck through. On unit, numeric and internally tagged variants, and on adjacently tagged and untagged ones, the `@deprecated` comment sits before a union member, which TypeScript doesn't treat as a deprecated symbol, so it's documentation only.
- **Fields:** only named fields are marked outside Rust. In OpenAPI, TypeScript and Python, a deprecated field of a tuple struct or newtype, or a deprecated `#[serde(flatten)]` field, is not marked (TypeScript's comment on a tuple element is documentation only). Deprecate the inner named fields instead.

The Python decorator uses `category=None`, so it only informs type checkers: pyright (with `reportDeprecated`) and mypy (with `--enable-error-code deprecated`) report call sites, and the method body emits the runtime warning.

`x-sunset` follows the convention oasdiff uses for a deprecated item's earliest removal date.

## What a sunset means

A sunset is the date you announce, not a check that removal is safe. reflectapi doesn't know which clients are deployed or whether they still use the item, and clients generated before the deprecation carry no warning at all. Before removing an item, confirm from your own traffic or client inventory that nothing still depends on it.
