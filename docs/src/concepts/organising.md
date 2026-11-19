# Organising a Larger API

A small API fits in one [`Builder`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html). A larger one is easier to keep as a builder
per module, combined into one API. This page covers how names in your Rust code
become names in the generated clients, and the builder methods that shape them.

## A builder per module

Each module defines its types, its handlers and a function returning its
builder. The root builder combines them:

```rust
use reflectapi::Builder;

#[derive(Clone, Default)]
pub struct AppState;

mod pets {
    use super::AppState;
    use reflectapi::{Builder, Empty};

    #[derive(serde::Serialize, reflectapi::Output)]
    pub struct Pet {
        pub name: String,
    }

    async fn list(_: AppState, _: Empty, _: Empty) -> Vec<Pet> {
        Vec::new()
    }

    pub fn api() -> Builder<AppState> {
        Builder::new()
            .name("pets")
            .route(list, |b| b.name("pets.list").readonly(true))
    }
}

mod admin {
    use super::AppState;
    use reflectapi::{Builder, Empty};

    #[derive(serde::Serialize, reflectapi::Output)]
    pub struct Stats {
        pub pets: u64,
    }

    async fn stats(_: AppState, _: Empty, _: Empty) -> Stats {
        Stats { pets: 0 }
    }

    pub fn api() -> Builder<AppState> {
        Builder::new()
            .name("admin")
            .path("/admin")
            .tag("internal")
            .route(stats, |b| b.name("admin.stats"))
    }
}

pub fn api() -> Builder<AppState> {
    Builder::new()
        .name("Pet store")
        .path("/api")
        .nest(pets::api())
        .nest(admin::api())
#       .rename_types("rust_out::", "petstore::")
        .rename_types("petstore::", "myapi::")
}

# fn main() {
let (schema, routers) = api().build().unwrap();

let urls: Vec<String> = schema
    .functions()
    .map(|f| format!("{}/{}", f.path(), f.name()))
    .collect();
assert_eq!(urls, ["/api/pets.list", "/api/admin/admin.stats"]);

let types: Vec<&str> = schema.output_types().types().map(|t| t.name()).collect();
assert!(types.contains(&"myapi::pets::Pet"));
assert!(types.contains(&"myapi::admin::Stats"));

let names: Vec<&str> = routers.iter().map(|r| r.name.as_str()).collect();
assert_eq!(names, ["Pet store", "pets", "admin"]);

let app = reflectapi::axum::into_router(AppState, routers, Default::default(), |name, router| {
    match name.as_str() {
        // A smaller request body limit for the admin module only.
        "admin" => router.layer(axum::extract::DefaultBodyLimit::max(64 * 1024)),
        _ => router,
    }
});
# let _: axum::Router = app;
# }
```

- [`Builder::path`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.path)
  sets the base path of the routes added after it, so call it before
  `route`. A route's own
  [`RouteBuilder::path`](https://docs.rs/reflectapi/latest/reflectapi/struct.RouteBuilder.html#method.path)
  replaces its builder's path; it is not appended to it.
- [`nest`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.nest)
  merges another builder and prepends this builder's path to its routes.
  [`extend`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.extend)
  merges it without changing paths. Both take its types, its validators and
  any errors it recorded.
- Each merged builder becomes its own router, named by its `name`. `build`
  returns the root's router first, then one per merged builder.
  [`into_router`](../serving/axum.md#mount-the-routes) passes each name to its
  callback, so middleware can differ per module.
- A builder's name doesn't prefix its routes' names. Give each route its full
  dotted name, as `admin.stats` above.
- Combine the module builders in the root directly. Today `nest` and `extend`
  take only the other builder's own routes into the routers: the routes it had
  merged from a third builder stay in the schema but are not served.

## Route names become client namespaces

A route's name is the name of its client method, and the dots group methods
into namespaces. With the API above, a TypeScript client calls
`client.pets.list(...)` and `client.admin.stats(...)`, and the Rust and Python
clients use the same paths. TypeScript and Rust nest every segment: `a.b.c`
is `client.a.b.c(...)`. Python makes only the first segment a namespace and
joins the rest with underscores: `client.a.b_c(...)`. A `-` in a name becomes
`_`, so `pets.get-first` is `get_first`.

The name is also part of the URL: a route is served at `{path}/{name}`, here
`/api/pets.list`. Renaming a route or changing a builder's path moves it, and
deployed clients still call the old URL. See [Evolving an API](./evolving.md).

## Type names come from module paths

A reflected type is named by its Rust path, crate included:
`petstore::pets::Pet`. The generated clients turn the path into namespaces:

| Client | `myapi::pets::Pet` becomes |
|--------|---------------------------|
| TypeScript | `myapi.pets.Pet`, in nested `namespace` blocks |
| Rust | `types::myapi::pets::Pet`, in nested modules |
| Python | `Pet` in the subpackage `myapi.pets` of the client package |

So the module layout of your server is the layout of your clients. Keep types
in the module they belong to, and rename the prefix you don't want clients to
see. [`rename_types`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.rename_types)
takes either a module prefix ending in `::`, which it replaces (an empty
replacement removes it), or a full type name, which it renames. The demo
renames its crate, `reflectapi_demo::`, to `myapi::`. With the default `glob`
feature,
[`glob_rename_types`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.glob_rename_types)
matches type names against a glob instead and replaces their whole module path.

- A rename applies to the types already in the schema. Call it after the
  routes and `nest` calls that bring the types in, and after
  `response_headers`.
- A rename that matches no type is a build error, which catches a stale
  pattern after a module moves. `allow_redundant_renames(true)`, called before
  the rename, turns it off.
- Renames don't check for collisions. Two types renamed to the same name
  produce a schema with both under one name, so keep the result unique.
- Type names are not on the wire. Moving or renaming a type changes the
  generated code, not what deployed clients send or receive.

A type that is sent and received with different shapes gets separate `input`
and `output` modules in the clients, such as `myapi.pets.input.Pet`. See
[Types](./types.md).

## Inline newtypes

A `#[serde(transparent)]` struct is sent as its one field, but clients still
get a named type for it, for example `type PetId = string` in TypeScript.
[`fold_transparent_types`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.fold_transparent_types)
replaces each of them with the type it wraps:

```rust
use reflectapi::{Builder, Empty};

#[derive(serde::Deserialize, reflectapi::Input)]
#[serde(transparent)]
pub struct PetId(String);

async fn get(_: (), _: PetId, _: Empty) -> Option<String> {
    None
}

let (schema, _) = Builder::<()>::new()
    .route(get, |b| b.name("pets.get"))
    .fold_transparent_types()
    .build()
    .unwrap();

let input = schema.functions().next().unwrap().input_type.as_ref().unwrap();
assert_eq!(input.name, "std::string::String");
assert!(!schema.input_types().types().any(|t| t.name().ends_with("PetId")));
```

Like `rename_types`, it applies to the types already in the schema.

## Project rules

[`validate`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.validate)
adds a check that `build` runs on the finished schema. `build` fails with
every error the checks return, alongside its own. Validators of nested
builders run too.

```rust
use reflectapi::{Builder, Empty, Schema, ValidationError, ValidationPointer};

fn routes_are_described(schema: &Schema) -> Vec<ValidationError> {
    schema
        .functions()
        .filter(|f| f.description().is_empty())
        .map(|f| {
            ValidationError::new(
                ValidationPointer::Function(f.name().into()),
                "add a description".into(),
            )
        })
        .collect()
}

async fn check(_: (), _: Empty, _: Empty) -> Empty {
    Empty {}
}

let errors = Builder::<()>::new()
    .route(check, |b| b.name("health.check"))
    .validate(routes_are_described)
    .build()
    .err()
    .unwrap();
assert_eq!(errors.to_string(), "health.check: add a description\n");
```

Validators run only where the schema is built: a [`DispatchOnly`](https://docs.rs/reflectapi/latest/reflectapi/struct.DispatchOnly.html) build skips
them. See [build modes](../serving/other-servers.md#build-modes).

## Tags and partial clients

[`Builder::tag`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.tag)
tags the routes added after it, and
[`RouteBuilder::tag`](https://docs.rs/reflectapi/latest/reflectapi/struct.RouteBuilder.html#method.tag)
tags one route; `untag` removes a tag. A parent's tags don't apply to the
builders it nests. Tags are in the schema, and the OpenAPI document lists
them on each operation.

`reflectapi codegen` takes `--include-tags` and `--exclude-tags`
(comma-separated). A route is kept when it has any included tag (or no
`--include-tags` is given) and none of the excluded ones. Only
`--language openapi` applies them today: the TypeScript, Rust and Python
generators accept the flags and generate every route. The demo uses them to
leave its `internal` routes out of the published OpenAPI document:

```bash
reflectapi codegen --language openapi --schema reflectapi.json \
  --output openapi/ --exclude-tags internal
```

For a client that covers only part of the API, build a second root builder
that nests only those modules, and write its schema next to the full one.
Each module's `api()` function can be called as often as needed.

## Sharing types with Rust clients

A Rust client may already depend on a crate that defines some of the API's
types, such as a crate of shared models. `--shared-modules` (Rust only,
comma-separated) leaves the types whose names start with one of the given
prefixes out of the generated code, and the generated code refers to them by
their schema name:

```bash
reflectapi codegen --language rust --schema reflectapi.json \
  --output clients/rust/ --shared-modules petstore_models::
```

- The match is a plain prefix of the type name. End the prefix with `::` so
  `petstore_models::` doesn't also match `petstore_models_v2::`.
- The schema name must resolve in the client crate, so don't rename those
  types with `rename_types`: `petstore_models::Pet` needs the client crate to
  depend on `petstore_models`.
- The shared types are used as they are, so they need the serde traits the
  client uses: `Serialize` for request types, `Deserialize` for response
  types.
