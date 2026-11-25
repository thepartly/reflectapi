# Convention Types

[`reflectapi::conventions`](https://docs.rs/reflectapi/latest/reflectapi/conventions/index.html)
holds the types behind the [API Design Standard](./standard.md): cursor
pagination, identity envelopes and the `if` clause on writes. Services that
use them share the same types in every generated client.

Enable the `conventions` feature. A Rust client uses these types from
`reflectapi` too, so its crate enables the feature as well:

```toml
reflectapi = { version = "0.26", features = ["builder", "conventions"] }
```

## Pagination

A list request has a `page` field, a `PageRequest`:
`{"page": {"cursor": …, "limit": …}}`, both optional. The response is a
`ForwardPaginatedList`: its items under `items`, and a `next_cursor` while
more follow.

A cursor is a `String` by default. A server that keeps something structured in
it names its own type, such as `Encoded<Cursor>`: clients still see a string,
and the server gets the decoded value.

```rust
use reflectapi::conventions::{ForwardPaginatedList, ForwardPagination, PageRequest};
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
[`conflict::actions`](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/index.html).
The schema then offers clients only those actions, and the handler matches on
the one the request chose:

```rust
use reflectapi::conventions::conflict::{actions, InsertOrElse, RemoveOrElse};
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
enum PetsInsertError {
    #[reflectapi(status = 409)]
    Exists,
}

#[derive(Debug, serde::Serialize, reflectapi::Output, reflectapi::ResponseError)]
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
| `pets.insert {"name": "Rex", "kind": "cat"}` | `409`: no `if`, so the default, `abort` |
| `pets.insert {"if": {"exists": "replace"}, "name": "Rex", "kind": "cat"}` | Rex is now a cat |
| `pets.insert {"if": {"exists": "ignore"}, …}` | `400`: `ignore` isn't in the set |
| `pets.remove {"if": {"not_exists": "ignore"}, "name": "Max"}` | succeeds, nothing removed |
| `pets.remove {"name": "Max"}` | `404`: the default, `abort` |

### Choosing a set

A set with `abort` defaults to it. A set without `abort`, such as
`ReplaceOrIgnore`, has no default, so the client always chooses: its route
takes the `…OrElseRequired` alias, whose `if` and action are required.

```rust
use reflectapi::conventions::conflict::{actions, InsertOrElseRequired};

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
[`actions` documentation](https://docs.rs/reflectapi/latest/reflectapi/conventions/conflict/actions/index.html).
