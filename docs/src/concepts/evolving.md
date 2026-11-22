# Evolving an API

A generated client is compiled against one version of the schema, and keeps
running after the server changes: in a browser tab left open, an app on a
phone, or a service deployed on its own schedule. This page lists which server
changes such clients survive, and how to make the others safely.

Two directions matter:

- **Old client, new server:** the server is deployed first, and clients
  generated before the change keep calling it. This is the usual case.
- **New client, old server:** a regenerated client reaches a server that
  doesn't have the change yet, during a rollout or a rollback.

## What clients tolerate

Generated clients ignore response fields they don't know. Python models are
generated with `extra="ignore"`, the Rust client's serde types ignore unknown
fields, and the TypeScript client doesn't check a response's fields at runtime.
A server built with reflectapi ignores unknown request fields too, unless the
type has `#[serde(deny_unknown_fields)]`.

A missing field is another matter. A Python model rejects a response without a
field it requires, and so does Rust for a field that isn't an `Option`.
TypeScript checks a response only on routes that can return an open enum or a
`#[serde(other)]` fallback; elsewhere the field is `undefined` where its type
says it's present. The server answers a request missing a required field with
`400`.

Type names are not on the wire: the JSON holds field names, variant tags and
values. Moving a type to another module or renaming it only changes the
generated code. A route is called at `{path}/{name}`, so its name and its
builder's path are on the wire.

## Changes at a glance

| Change | Old client, new server | New client, old server |
|--------|------------------------|------------------------|
| Add a response field | Safe | Breaks where the new client requires the field |
| Add an optional request field | Safe | Safe, but the old server ignores the value |
| Add a required request field | Breaks: `400` | Safe, but the old server ignores the value |
| Remove a response field | Breaks where the old client requires the field | Safe |
| Remove a request field | Safe: the server ignores it | Breaks if the old server requires it |
| Rename a field, keeping its serde name | Safe | Safe |
| Add a response enum variant | Breaks when sent, unless the enum is open or has a fallback | Safe |
| Add a request enum variant | Safe | Breaks when sent: `400` |
| Add a route | Safe | `404` when called |
| Remove or rename a route, or change its path | `404` when called | Safe |
| Rename or move a type | Safe | Safe |

"Breaks" for a response means a Python client raises a validation error (for
an error response, an `ApplicationError` whose `typed_error` is `None`), a
Rust client returns a protocol error, and a TypeScript client usually hands
the caller a value its types don't describe. Changing a field's type breaks
whenever a value of one type doesn't decode as the other.

## Adding fields

A new response field is safe for deployed clients. Deploy the server before
clients that rely on it.

A new request field must be optional for the clients that don't send it: an
`Option<T>`, which a missing key leaves `None`, or a field with
`#[serde(default)]`. Generated clients then let callers leave it out, and a
request from a client generated before the change still decodes:

```rust
use reflectapi::dispatch::{Dispatcher, Reply};

#[derive(serde::Deserialize, reflectapi::Input)]
struct CreatePetRequest {
    name: String,
    // Added after clients shipped.
    #[serde(default)]
    tags: Vec<String>,
    nickname: Option<String>,
}

async fn create(_: (), request: CreatePetRequest, _: reflectapi::Empty) -> String {
    request.name
}

# #[tokio::main]
# async fn main() {
let (schema, routers) = reflectapi::Builder::<()>::new()
    .route(create, |b| b.name("pets.create"))
    .build()
    .unwrap();

// Clients generated from the new schema may leave the new fields out...
let request = schema
    .input_types()
    .types()
    .find(|t| t.name().ends_with("CreatePetRequest"))
    .and_then(|t| t.as_struct())
    .unwrap();
let optional: Vec<&str> = request.fields().filter(|f| !f.required).map(|f| f.name()).collect();
assert_eq!(optional, ["tags", "nickname"]);

// ...and a request from a client generated before them still succeeds.
let reply = Dispatcher::new(routers)
    .dispatch(
        (),
        &http::Method::POST,
        "/pets.create",
        &http::HeaderMap::new(),
        r#"{"name":"Rex"}"#.into(),
    )
    .await;
assert!(matches!(reply, Some(Reply::Complete { status, .. }) if status == 200));
# }
```

Without `#[serde(default)]`, the same request gets `400` for the missing
`tags`. See [Missing fields and null](../getting-started/attributes.md#missing-fields-and-null)
for how defaults, `Option` and [`reflectapi::Option`](https://docs.rs/reflectapi/latest/reflectapi/enum.Option.html) map to required and
nullable fields in each direction.

Generated Rust types don't derive `Default` unless the type asks for it with
`#[reflectapi(derive(Default))]`, so Rust code that builds a request with a
struct literal must add the new field after regenerating.

## Removing and renaming fields

To rename a field in Rust without changing the wire, keep its old name with
`#[serde(rename = "...")]`. Neither deployed nor regenerated clients notice:
clients name a field after its serde name, and use the Rust identifier only
when the serde name can't be one.

To remove a response field, deprecate it first:

```rust
#[derive(serde::Serialize, reflectapi::Output)]
struct Pet {
    name: String,
    #[deprecated(note = "Use `name`.")]
    #[reflectapi(sunset = "2027-01-01")]
    title: String,
}

#[allow(deprecated)]
async fn get(_: (), _: reflectapi::Empty, _: reflectapi::Empty) -> Pet {
    Pet { name: "Rex".into(), title: "Rex".into() }
}

let (schema, _) = reflectapi::Builder::<()>::new()
    .route(get, |b| b.name("pets.get"))
    .build()
    .unwrap();
let files = reflectapi::codegen::typescript::generate(schema, &Default::default()).unwrap();
assert!(files["generated.ts"].contains("title?: string"));
```

The server still sends `title`, but clients generated from now on treat it as
optional: `title?: string` in TypeScript, an `Option` in Rust, and `None`
when missing in Python. Once the clients generated before the deprecation are
gone, removing the field breaks nothing. This applies to named fields of types
used only in responses. A type also used in requests keeps the field required,
and so does a type a client decodes by trying alternatives: an untagged enum,
an enum with an untagged variant, or the type of a flattened `Option` field.

To remove a request field, first make it optional, so new clients can stop
sending it, then remove it once the clients that require it are gone. The
server ignores the field if an old client still sends it.

[Deprecations](../getting-started/deprecations.md) covers declaring the
release, the dates and the note, and what each client shows. A sunset date is
an announcement: reflectapi doesn't know which clients are still deployed.

## Adding enum variants

A client generated from a closed response enum can't decode a variant it
doesn't know. Mark
the enum `#[reflectapi(open)]`, or give it a `#[serde(other)]` unit variant,
before you need to add variants: clients generated after that accept
unknown tags as an explicit unknown case or as the fallback. Clients generated
earlier don't. See [Enum evolution](../getting-started/attributes.md#enum-evolution)
for the attribute and
[Handling unfamiliar enum variants](../clients/index.md#handling-unfamiliar-enum-variants)
for client code.

The server decodes a request enum with serde, so it rejects a variant it
doesn't know with `400`, unless the enum has a `#[serde(other)]` variant.
Generated request enums stay closed, so a client sends only the variants it
was generated with: a new request variant is safe once every server has it.

A route's error is a response type, so the same applies to a new error
variant. See [Errors](./errors.md) for what a client does with an error it
can't decode.

## Adding, removing and renaming routes

A new route is safe for deployed clients. A removed or renamed route answers
`404` to the clients that still call it, so deprecate it first and keep it
until they are gone. To rename one, register the same handler under both
names, with the old one deprecated; the demo does this for `pets.delete`:

```rust
# async fn pets_remove(_: (), _: reflectapi::Empty, _: reflectapi::Empty) {}
let builder = reflectapi::Builder::<()>::new()
    .route(pets_remove, |b| b.name("pets.remove"))
    .route(pets_remove, |b| {
        b.name("pets.delete")
            .deprecation_note("Use pets.remove instead.")
            .sunset("2027-01-01")
    });
# let (schema, _) = builder.build().unwrap();
# assert_eq!(schema.functions().count(), 2);
```

Changing a builder's path or a route's name moves the route in the same way.
See [Organising a Larger API](./organising.md#route-names-become-client-namespaces).

## Keeping clients in sync

Generate the schema and the clients from the server's code, commit them, and
have CI regenerate them and fail on a difference.
[Generate the schema and clients at build time](../serving/workers.md#generate-the-schema-and-clients-at-build-time)
shows a test that does it. Every API change then shows up in review as a diff
of the committed schema and clients, next to the code that caused it. Use the
table above to check that diff.

A TypeScript or Rust client compiled in the same repository fails to build
when a change breaks the code that calls it. That catches breaks in the code
you have, not in clients already deployed.
