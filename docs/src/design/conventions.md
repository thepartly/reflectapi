# Convention Types

[`reflectapi-conventions`](https://docs.rs/reflectapi-conventions/latest/reflectapi_conventions/index.html)
holds the types behind [Partly's HTTP API Guidelines](./guidelines.md): cursor
pagination, identity envelopes, the `if` clause on writes, the answer to a
request over a rate limit, and units of measurement. Services that
use them share the same types in every generated client.

Add the crate beside `reflectapi`:

```toml
reflectapi = { version = "0.26", features = ["builder"] }
reflectapi-conventions = "0.26"
```

Their Rust paths start with the crate's, `reflectapi_conventions::`. Rename
them to `reflectapi::` on the builder, so they have the same names in every
API that uses them
([`api-type-shared-same-name`](./guidelines.md#api-type-shared-same-name)):

```rust,ignore
let builder = reflectapi::Builder::new()
    // routes …
    .rename_types("reflectapi_conventions::", "reflectapi::");
```

Clients then see `reflectapi.pagination.PageRequest` and so on. Every
generated client has its own copy of the types, a Rust client too, so a
client's crate doesn't depend on `reflectapi-conventions`.

## Pagination

A list request has a `page` field, a `PageRequest`:
`{"page": {"cursor": …, "limit": …}}`, both optional. The response is a
`ForwardPaginatedList`: its items under `items`, and a `next_cursor` while
more follow.

A cursor is a `String` by default. A server that keeps something structured in
it names its own type, such as `Encoded<Cursor>`: clients still see a string,
and the server gets the decoded value.

```rust
use reflectapi_conventions::{ForwardPaginatedList, ForwardPagination, PageRequest};
use reflectapi::Encoded;
use std::sync::Arc;

#[derive(Clone, serde::Serialize, reflectapi::Output)]
struct Pet {
    id: u64,
    name: String,
}

/// What a cursor holds. Only the server reads it.
#[derive(serde::Serialize, serde::Deserialize)]
struct PetsCursor {
    after_id: u64,
}

type Pets = Arc<Vec<Pet>>;

#[derive(serde::Deserialize, reflectapi::Input)]
struct PetsListRequest {
    #[serde(default)]
    page: PageRequest<Encoded<PetsCursor>>,
}

async fn pets_list(
    pets: Pets,
    request: PetsListRequest,
    _: reflectapi::Empty,
) -> ForwardPaginatedList<Pet, Encoded<PetsCursor>> {
    let page = request.page;
    let after = page.cursor.map_or(0, |cursor| cursor.after_id);
    let limit = page.limit.map_or(20, |limit| limit.get() as usize);
    let mut rest = pets.iter().filter(|pet| pet.id > after);
    let items: Vec<Pet> = rest.by_ref().take(limit).cloned().collect();
    let next_cursor = match (rest.next(), items.last()) {
        (Some(_), Some(last)) => Some(Encoded(PetsCursor { after_id: last.id })),
        _ => None,
    };
    ForwardPaginatedList {
        items,
        pagination: Some(ForwardPagination { next_cursor }),
    }
}
# #[tokio::main]
# async fn main() {
# let (_schema, _routers) = reflectapi::Builder::<Pets>::new()
#     .route(pets_list, |b| b.name("pets.list"))
#     .build()
#     .unwrap();
# let pets: Pets = Arc::new(
#     ["Rex", "Max", "Bella"]
#         .iter()
#         .zip(1..)
#         .map(|(name, id)| Pet { id, name: name.to_string() })
#         .collect(),
# );
# let call = |json: &str| {
#     let request = serde_json::from_str(json).unwrap();
#     let pets = pets.clone();
#     async move { serde_json::to_value(pets_list(pets, request, reflectapi::Empty {}).await).unwrap() }
# };
# assert_eq!(
#     call(r#"{"page": {"limit": 2}}"#).await,
#     serde_json::json!({
#         "items": [{"id": 1, "name": "Rex"}, {"id": 2, "name": "Max"}],
#         "pagination": {"next_cursor": "eyJhZnRlcl9pZCI6Mn0"}
#     })
# );
# assert_eq!(
#     call(r#"{"page": {"cursor": "eyJhZnRlcl9pZCI6Mn0", "limit": 2}}"#).await,
#     serde_json::json!({
#         "items": [{"id": 3, "name": "Bella"}],
#         "pagination": {"next_cursor": null}
#     })
# );
# }
```

A client reads the pages until `next_cursor` is `null`:

```text
→ pets.list  {"page": {"limit": 2}}
← {"items": [{"id": 1, "name": "Rex"}, {"id": 2, "name": "Max"}],
   "pagination": {"next_cursor": "eyJhZnRlcl9pZCI6Mn0"}}

→ pets.list  {"page": {"cursor": "eyJhZnRlcl9pZCI6Mn0", "limit": 2}}
← {"items": [{"id": 3, "name": "Bella"}],
   "pagination": {"next_cursor": null}}
```

A page may hold fewer items than `limit` even when more follow, so a client
checks `next_cursor`, never the count. A list that isn't paginated yet returns
a `NonPaginatedList`, `{"items": […]}`: adding `pagination` to it later doesn't
break clients, while changing a bare array would.

## Identity

`Identity<I>` is `{"identity": …}`, for an entity that can be identified in
more than one way, its value saying which:

```json
{"identity": {"sku": "A-1"}}
```

`IdentityData<I, D>` adds the data's fields beside it, as in an insert or an
`ingest` item: `{"identity": {"sku": "A-1"}, "name": "Bolt"}`.

## Conflict resolution

A write can say what to do when what it writes already exists, or what it
changes doesn't: an `if` clause beside the request's own fields.

```json
{"if": {"exists": "replace"}, "name": "Rex", "kind": "cat"}
```

A route's input is one of the aliases in `conflict`, such as `InsertOrElse`,
over its data and the set of actions it offers, from
[`conflict::actions`](https://docs.rs/reflectapi-conventions/latest/reflectapi_conventions/conflict/actions/index.html).
The schema then offers clients only those actions, and the handler matches on
the one the request chose:

```rust
use reflectapi_conventions::conflict::{actions, InsertOrElse, RemoveOrElse};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq, serde::Deserialize, reflectapi::Input)]
struct Pet {
    name: String,
    kind: String,
}

#[derive(serde::Deserialize, reflectapi::Input)]
struct PetName {
    name: String,
}

#[derive(Debug, serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PetsInsertError {
    #[reflectapi(status = 409)]
    Exists,
}

#[derive(Debug, serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PetsRemoveError {
    #[reflectapi(status = 404)]
    NotFound,
}

type Store = Arc<Mutex<HashMap<String, Pet>>>;

/// If it exists: replace it, or abort, the default.
async fn pets_insert(
    store: Store,
    input: InsertOrElse<Pet, actions::ReplaceOrAbort>,
    _: reflectapi::Empty,
) -> Result<reflectapi::Empty, PetsInsertError> {
    let mut pets = store.lock().unwrap();
    if pets.contains_key(&input.data.name) {
        match input.if_.exists {
            actions::ReplaceOrAbort::Replace => {}
            actions::ReplaceOrAbort::Abort => return Err(PetsInsertError::Exists),
        }
    }
    pets.insert(input.data.name.clone(), input.data);
    Ok(reflectapi::Empty {})
}

/// If it doesn't exist: abort, the default, or ignore.
async fn pets_remove(
    store: Store,
    input: RemoveOrElse<PetName, actions::AbortOrIgnore>,
    _: reflectapi::Empty,
) -> Result<reflectapi::Empty, PetsRemoveError> {
    let removed = store.lock().unwrap().remove(&input.data.name);
    match (removed, input.if_.not_exists) {
        (None, actions::AbortOrIgnore::Abort) => Err(PetsRemoveError::NotFound),
        _ => Ok(reflectapi::Empty {}),
    }
}
# #[tokio::main]
# async fn main() {
# let (_schema, _routers) = reflectapi::Builder::<Store>::new()
#     .route(pets_insert, |b| b.name("pets.insert"))
#     .route(pets_remove, |b| b.name("pets.remove"))
#     .build()
#     .unwrap();
# let store = Store::default();
# let insert = |json: &str| {
#     let input = serde_json::from_str(json);
#     let store = store.clone();
#     async move { pets_insert(store, input.unwrap(), reflectapi::Empty {}).await }
# };
# let remove = |json: &str| {
#     let input = serde_json::from_str(json).unwrap();
#     let store = store.clone();
#     async move { pets_remove(store, input, reflectapi::Empty {}).await }
# };
# assert!(insert(r#"{"name": "Rex", "kind": "dog"}"#).await.is_ok());
# assert!(matches!(insert(r#"{"name": "Rex", "kind": "cat"}"#).await, Err(PetsInsertError::Exists)));
# assert!(insert(r#"{"if": {"exists": "replace"}, "name": "Rex", "kind": "cat"}"#).await.is_ok());
# assert_eq!(store.lock().unwrap()["Rex"].kind, "cat");
# assert!(serde_json::from_str::<InsertOrElse<Pet, actions::ReplaceOrAbort>>(
#     r#"{"if": {"exists": "ignore"}, "name": "Rex", "kind": "cat"}"#
# ).is_err());
# assert!(remove(r#"{"if": {"not_exists": "ignore"}, "name": "Max"}"#).await.is_ok());
# assert!(matches!(remove(r#"{"name": "Max"}"#).await, Err(PetsRemoveError::NotFound)));
# }
```

On the wire, with Rex not yet stored:

| Request | Result |
|---|---|
| `pets.insert {"name": "Rex", "kind": "dog"}` | inserted |
| `pets.insert {"name": "Rex", "kind": "cat"}` | `409`, `{"kind": "exists"}`: no `if`, so the default, `abort` |
| `pets.insert {"if": {"exists": "replace"}, "name": "Rex", "kind": "cat"}` | Rex is now a cat |
| `pets.insert {"if": {"exists": "ignore"}, …}` | `400`: `ignore` isn't in the set |
| `pets.remove {"if": {"not_exists": "ignore"}, "name": "Max"}` | succeeds, nothing removed |
| `pets.remove {"name": "Max"}` | `404`, `{"kind": "not_found"}`: the default, `abort` |

### Choosing a set

A set with `abort` defaults to it. A set without `abort`, such as
`ReplaceOrIgnore`, has no default, so the client always chooses: its route
takes the `…OrElseRequired` alias, whose `if` and action are required.

```rust
use reflectapi_conventions::conflict::{actions, InsertOrElseRequired};

#[derive(serde::Deserialize, reflectapi::Input)]
struct Pet {
    name: String,
}

type PetsImport = InsertOrElseRequired<Pet, actions::ReplaceOrIgnore>;

let insert: PetsImport =
    serde_json::from_str(r#"{"if": {"exists": "ignore"}, "name": "Rex"}"#).unwrap();
assert_eq!(insert.if_.exists, actions::ReplaceOrIgnore::Ignore);

// Without `if`, refused.
assert!(serde_json::from_str::<PetsImport>(r#"{"name": "Rex"}"#).is_err());
```

- `replace` overwrites what is stored: fields the request leaves out are reset
  to their defaults. `update` patches it: they keep their stored values. Both
  insert when nothing is stored, and insert with `update` is the upsert.
- The sets without `abort` implement `actions::Idempotent`: processing the
  same write again ends the same way, so a write from a queue that may deliver
  twice offers only those.
- The `conflict::matching` clauses also check, before writing, what the entity
  must be: `{"if": {"matches": {"edition": 7}}, …}` writes only if it is still
  at edition 7. An insert of several checks its parent instead, with
  `parent_matches`.

The full catalogue of sets, with their defaults, is in the
[`actions` documentation](https://docs.rs/reflectapi-conventions/latest/reflectapi_conventions/conflict/actions/index.html).

## Rate limiting

A call that limits its rate answers a request over the limit with `429`, a
`retry-after` header, and a `rate_limited` case of its error type that says
how long to wait, in seconds
([`api-rate-limit-answered-429`](./guidelines.md#api-rate-limit-answered-429)).
`rate_limited::RateLimited` is that case: a case of the call's error enum
marked `#[reflectapi(transparent)]` takes its status and header, made from
its field, so the two never disagree.

```rust
use reflectapi::ResponseError;
use reflectapi_conventions::rate_limited::RateLimited;

#[derive(serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[reflectapi(open)]
enum OrdersListError {
    /// Takes the status and the headers of `RateLimited`.
    #[reflectapi(transparent)]
    RateLimited(RateLimited),
    #[reflectapi(status = 403)]
    Forbidden,
}

let error = OrdersListError::RateLimited(RateLimited { retry_after_seconds: 30 });
assert_eq!(error.status(), 429);
assert_eq!(error.headers()["retry-after"], "30");
assert_eq!(
    serde_json::to_string(&error).unwrap(),
    r#"{"kind":"rate_limited","retry_after_seconds":30}"#
);
```

On the wire:

```http
429 Too Many Requests
retry-after: 30
content-type: application/json

{"kind": "rate_limited", "retry_after_seconds": 30}
```

Every generated client has the case as a typed value, for an app that shows
"try again in 30 seconds". A client's retry reads the header, which a `429`
from a gateway in front of the server has too.

## Units

`units::Unit` is the guidelines' [table of units](./guidelines.md#units) as
an enum, so every API spells a unit the same way
([`api-unit-spelled-as-table`](./guidelines.md#api-unit-spelled-as-table)). Each unit has three
spellings:

| | `Unit::Millimeter` | `Unit::Celsius` | `Unit::Kilobyte` |
|---|---|---|---|
| `name()`, its value on the wire | `millimeter` | `celsius` | `kilobyte` |
| `symbol()`, for showing it to a person | `mm` | `°C` | `kB` |
| `suffix()`, for a field's name | `Some("_mm")` | `Some("_celsius")` | `None`: a size is `_bytes` |

A quantity whose unit is fixed is a number named with the unit's suffix,
`width_mm` ([`api-quantity-fixed-unit-suffix`](./guidelines.md#api-quantity-fixed-unit-suffix)).
One whose unit varies with the value, such as a measurement one source gives
in inches and another in millimetres, is a `units::Quantity`, which keeps the
unit it was given in
([`api-quantity-varying-unit-object`](./guidelines.md#api-quantity-varying-unit-object)):

```rust
use reflectapi_conventions::units::{Quantity, Unit};

#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input, reflectapi::Output)]
struct Part {
    weight_grams: u32,
    width: Quantity,
}

let part = Part {
    weight_grams: 120,
    width: Quantity { value: 12.5, unit: Unit::Millimeter },
};
assert_eq!(
    serde_json::to_string(&part).unwrap(),
    r#"{"weight_grams":120,"width":{"value":12.5,"unit":"millimeter"}}"#
);

// The server reads a symbol as well as a name, and sends the name.
let width: Quantity = serde_json::from_str(r#"{"value": 0.5, "unit": "in"}"#).unwrap();
assert_eq!(width.unit, Unit::Inch);
// It refuses a unit it doesn't know.
assert!(serde_json::from_str::<Unit>(r#""furlong""#).is_err());
```

`Unit` is open in a response
([`api-enum-output-open`](./guidelines.md#api-enum-output-open)): a client
generated before a unit was added keeps it as an unknown value. In a request
it is closed, as every enum is
([`api-enum-request-closed`](./guidelines.md#api-enum-request-closed)), so a
server refuses a unit it doesn't know.
Clients get the names alone; the symbols and suffixes stay in the Rust crate.

## Lints

[`conventions::lints`](https://docs.rs/reflectapi-conventions/latest/reflectapi_conventions/lints/index.html)
checks a schema against the rules of the [guidelines](./guidelines.md) that its
shape can show. Each lint is a function for `Builder::validate` and checks
one rule; an error names its rule in `ValidationError::rule`, and its message
links to it.

```rust,ignore
use reflectapi_conventions::lints;

let mut builder = api();
for lint in lints::ALL {
    builder = builder.validate(*lint);
}
let (schema, routers) = builder.build()?;
```

| Lint | Rule |
|---|---|
| `call_name_form` | [`api-call-name-form`](./guidelines.md#api-call-name-form) |
| `call_names_kebab_case` | [`api-call-name-kebab-case`](./guidelines.md#api-call-name-kebab-case) |
| `call_resource_levels` | [`api-resource-two-levels`](./guidelines.md#api-resource-two-levels) |
| `call_version_suffix`: reports `groups.v2.insert`; use `groups.insert-v2` | [`api-call-version-suffix`](./guidelines.md#api-call-version-suffix) |
| `call_actions_standard`: `create`, `delete`, `fetch`, `upsert` and other words a standard action replaces | [`api-insert-not-create`](./guidelines.md#api-insert-not-create), [`api-remove-not-delete`](./guidelines.md#api-remove-not-delete), [`api-action-standard`](./guidelines.md#api-action-standard) |
| `call_actions_not_parameters`: `search-by-name`, `update-password` | [`api-action-not-parameters`](./guidelines.md#api-action-not-parameters) |
| `request_is_object` | [`api-arguments-in-body-object`](./guidelines.md#api-arguments-in-body-object) |
| `response_is_object`: the output, and the error, whose cases without data are strings unless written in `kind` | [`api-response-is-object`](./guidelines.md#api-response-is-object) |
| `names_snake_case`: an enum value in capitals, such as `NZ`, passes | [`api-name-snake-case`](./guidelines.md#api-name-snake-case), [`api-enum-outside-codes-keep-form`](./guidelines.md#api-enum-outside-codes-keep-form) |
| `enums_kind_tag`: an enum with fields, and an error type, written in `kind` | [`api-enum-kind-tag`](./guidelines.md#api-enum-kind-tag), [`api-enum-one-value-cases`](./guidelines.md#api-enum-one-value-cases) |
| `header_names_kebab_case` | [`api-header-name-case`](./guidelines.md#api-header-name-case) |
| `header_names_no_x_prefix` | [`api-header-own-prefix`](./guidelines.md#api-header-own-prefix) |
| `call_type_names`: a type named as a call's request, response or error, `…Request`, `…Response` or `…Error`, names its call | [`api-type-named-after-call`](./guidelines.md#api-type-named-after-call) |
| `type_names_no_kind_suffix` | [`api-type-no-kind-suffix`](./guidelines.md#api-type-no-kind-suffix) |
| `type_path_depth`: two modules at most, a version not counted | [`api-type-path-depth`](./guidelines.md#api-type-path-depth) |
| `type_modules_not_implementation`: no `models`, `types`, `errors` and the like | [`api-type-modules-not-implementation`](./guidelines.md#api-type-modules-not-implementation) |
| `error_cases_no_error_suffix` | [`api-error-case-no-error-suffix`](./guidelines.md#api-error-case-no-error-suffix) |
| `conventions_types_renamed`: this crate's types renamed from `reflectapi_conventions::` to `reflectapi::` | [`api-type-shared-same-name`](./guidelines.md#api-type-shared-same-name) |
| `unit_suffixes`: `_secs`, `_s`, `_millis`, `_kb`, `_mb` and other spellings of a unit the table of units spells another way, and `_cents` | [`api-unit-suffix-one-spelling`](./guidelines.md#api-unit-suffix-one-spelling) |
| `output_enums_open`: an enum reachable from a call's output, or its stream's items, and one reachable from its error type, is `#[reflectapi(open)]` | [`api-enum-output-open`](./guidelines.md#api-enum-output-open), [`api-error-open`](./guidelines.md#api-error-open) |

A lint reports what the guidelines rule out, and leaves what it can't judge:

- Actions are checked against the words a standard action replaces, not a
  list of allowed ones, so an application's own action, such as
  `orders.confirm`, passes.
- `call_type_names` checks only a type that one call alone uses and that is
  named as a call's, with the suffix `Request`, `Response` or `Error`. Such a
  name ends with the call's action and uses the call's words in their order:
  `ProductsSearchRequest`, or `SearchRequest` in a module of its own, for
  `products.search`; a version may be left out. A type the call takes as it
  is, such as a `Product`, isn't checked.
- `output_enums_open` can't tell an enum whose cases are fixed by what they
  mean, such as `asc | desc`, from one that grows: such an enum in an output
  passes through a wrapper that filters its error, below. An error type has
  no such exception. An enum `open` doesn't apply to, untagged or written as
  numbers, isn't checked, nor is an output's enum with a `#[serde(other)]`
  case, which reads unknown cases too.

To relax a lint, wrap it in a function of your own that drops the errors you
accept, such as those of calls not yet migrated:

```rust,ignore
fn call_actions_standard(schema: &reflectapi::Schema) -> Vec<reflectapi::ValidationError> {
    lints::call_actions_standard(schema)
        .into_iter()
        .filter(|error| !error.to_string().starts_with("uploads.delete:"))
        .collect()
}
```

