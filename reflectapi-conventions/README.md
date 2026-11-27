# reflectapi-conventions

Types and lints for one opinionated style of API built with
[reflectapi](https://reflectapi.rs/):

- **`pagination`:** `PageRequest` (`cursor`, `limit`), `NonPaginatedList` and
  `ForwardPaginatedList`, each keeping its items under `items`.
- **`identity`:** `Identity` and `IdentityData`, for entities identified in
  more than one way and for bulk-write items.
- **`conflict`:** an `if` clause on a write, `{"if": {"exists": "replace"}, …}`,
  with a catalogue of action sets.
- **`rate_limited`:** `RateLimited`, the case of a call's error that answers a
  request over its rate limit: `429`, with `retry-after`.
- **`units`:** `Unit`, the guidelines' table of units, with each unit's name,
  symbol and field-name suffix, and `Quantity`, a value with its unit.
- **`lints`:** checks of a schema against
  [Partly's HTTP API Guidelines](https://reflectapi.rs/design/guidelines.html), one
  function per rule, for `Builder::validate`.

See [Convention Types](https://reflectapi.rs/design/conventions.html) for the
guide, and [docs.rs](https://docs.rs/reflectapi-conventions) for the API.

Rename the types to `reflectapi::` on the builder, so every API that uses
them gives clients the same names, such as `reflectapi.pagination.PageRequest`:

```rust
builder.rename_types("reflectapi_conventions::", "reflectapi::")
```

Every generated client has its own copy of the types, a Rust client too, so a
client's crate doesn't depend on this one.

Licensed under either of MIT or Apache-2.0, at your option.
