# Types

A type can cross the API when it implements
[`Input`](https://docs.rs/reflectapi/latest/reflectapi/trait.Input.html), for
values the server reads, or
[`Output`](https://docs.rs/reflectapi/latest/reflectapi/trait.Output.html), for
values it writes. Your own types derive them next to serde's derives.
reflectapi implements them for the standard types below and, behind crate
features, for common library types. This page covers what can be used, how it
appears in the generated clients, and what to do with a type reflectapi
doesn't know.

## Your own types

Structs and enums derive `Input`, `Output` or both. The derives read serde's
attributes, so the schema describes the JSON serde reads and writes.

```rust
mod model {
    use serde::Serialize;

    /// One page of a list.
    #[derive(Serialize, reflectapi::Output)]
    pub struct Page<T> {
        pub items: Vec<T>,
        pub next_cursor: Option<String>,
    }

    #[derive(Serialize, reflectapi::Output)]
    pub struct Pet {
        pub name: String,
    }
}

async fn pets_list(
    _: (),
    _: reflectapi::Empty,
    _: reflectapi::Empty,
) -> model::Page<model::Pet> {
    model::Page { items: vec![], next_cursor: None }
}

# fn main() -> Result<(), reflectapi::BuildErrors> {
let (schema, _) = reflectapi::Builder::new()
    .route(pets_list, |b| b.name("pets.list"))
    .build()?;

// Types are named by their Rust module path.
let page = schema
    .output_types()
    .types()
    .find(|t| t.name().ends_with("::model::Page"))
    .unwrap();
assert_eq!(page.parameters().map(|p| p.name()).collect::<Vec<_>>(), ["T"]);
# Ok(())
# }
```

- Generic types stay generic: the clients get `Page<T>` (`Page[T]` in
  Python), used as `Page<Pet>`. The derive bounds each type parameter by
  `Input` or `Output`. It doesn't support lifetime or const generic
  parameters.
- A type's name in the schema is its Rust module path and name, such as
  `my_api::model::Pet`. The module path becomes a namespace in each client:
  `my_api.model.Pet` in TypeScript, `types::my_api::model::Pet` in Rust and the
  `api_client.my_api.model` package in Python.
  [Organising a Larger API](./organising.md) shows how to rename them.
- Doc comments on types, fields and variants are carried into the clients.

## Standard types

| Rust | TypeScript |
|------|------------|
| `u8` to `u128`, `i8` to `i128`, `usize`, `isize`, `NonZeroU8` to `NonZeroU128` | `number` |
| `f32`, `f64` | `number` |
| `bool` | `boolean` |
| `String`, `char`, `PathBuf`; `&'static str` as output only | `string` |
| `Option<T>` | `T \| null` |
| `Vec<T>`, `HashSet<T>`, `BTreeSet<T>` | `Array<T>` |
| `[T; N]` | `FixedSizeArray<T, N>`, an array of length `N`; see [Lengths](#lengths) |
| `HashMap<K, V>`, `BTreeMap<K, V>` | `Record<K, V>` |
| tuples of 1 to 12 elements | `[A, B, ...]` |
| `()` | `null` |
| `Box<T>`, `Arc<T>` (with serde's `rc` feature), `Cell<T>`, `RefCell<T>`, `Mutex<T>`, `RwLock<T>` | `T` |
| `std::time::Duration` | `{ secs: number, nanos: number }`, as serde writes it |

The Rust client uses the same Rust types. The Python client uses the matching
built-in types (`int`, `float`, `str`, `list`, `dict`, `tuple`, `None`),
`Path` for paths and `timedelta` for `Duration`.

A JavaScript `number` holds integers exactly only up to 2<sup>53</sup>. A
`u64` or `i64` field whose values can go beyond that loses precision in a
TypeScript client. Send such values as strings: serialize the field as a
string and describe it as one with a [type override](#types-reflectapi-doesnt-know).

## Library types

Enable a feature of the `reflectapi` crate to use the types of another crate:

| Feature | Types | TypeScript |
|---------|-------|------------|
| `chrono` | `DateTime<Utc>`, `DateTime<Local>`, `DateTime<FixedOffset>`, `NaiveDateTime`, `NaiveDate`, `NaiveTime` | `string` |
| `chrono-tz` | `chrono_tz::Tz` | `string` |
| `uuid` | `uuid::Uuid` | `string` |
| `url` | `url::Url` | `string` |
| `rust_decimal` | `rust_decimal::Decimal` | `string` |
| `json` | `serde_json::Value`; `serde_json::Map` as a map; [`reflectapi::Raw<T>`](#raw-json) as the `T`; [`reflectapi::Encoded<T>`](#encoded-values) as a string | `any`; the `T`'s type; `string` |
| `indexmap` | `IndexMap<K, V>`, `IndexSet<T>`, as maps and sets | `Record<K, V>`, `Array<T>` |
| `conventions` | [`reflectapi::conventions`](../design/conventions.md): pagination, identity envelopes, conflict clauses | their own types |

```bash
cargo add reflectapi --features builder,chrono,uuid
```

The Rust client uses the same types, so a crate that includes it needs the
same libraries. The Python client uses `datetime` and `date` for chrono's
dates and `UUID` for `uuid::Uuid`.

## Enums

Serde's [enum representations](https://serde.rs/enum-representations.html)
are reflected, and each client's type matches the JSON. For this enum:

```rust
enum Shape {
    Circle { radius: f64 },
    Empty,
}
# use serde_json::to_string as json;
# #[derive(serde::Serialize)] enum External { Circle { radius: f64 }, Empty }
# #[derive(serde::Serialize)] #[serde(tag = "type")] enum Internal { Circle { radius: f64 }, Empty }
# #[derive(serde::Serialize)] #[serde(tag = "t", content = "c")] enum Adjacent { Circle { radius: f64 }, Empty }
# #[derive(serde::Serialize)] #[serde(untagged)] enum Untagged { Circle { radius: f64 }, Empty }
# fn main() {
# let _ = (Shape::Circle { radius: 1.0 }, Shape::Empty);
# assert_eq!(json(&External::Circle { radius: 1.0 }).unwrap(), r#"{"Circle":{"radius":1.0}}"#);
# assert_eq!(json(&External::Empty).unwrap(), r#""Empty""#);
# assert_eq!(json(&Internal::Circle { radius: 1.0 }).unwrap(), r#"{"type":"Circle","radius":1.0}"#);
# assert_eq!(json(&Internal::Empty).unwrap(), r#"{"type":"Empty"}"#);
# assert_eq!(json(&Adjacent::Circle { radius: 1.0 }).unwrap(), r#"{"t":"Circle","c":{"radius":1.0}}"#);
# assert_eq!(json(&Adjacent::Empty).unwrap(), r#"{"t":"Empty"}"#);
# assert_eq!(json(&Untagged::Circle { radius: 1.0 }).unwrap(), r#"{"radius":1.0}"#);
# assert_eq!(json(&Untagged::Empty).unwrap(), "null");
# }
```

| Serde attribute | JSON | TypeScript |
|-----------------|------|------------|
| none (externally tagged) | `{"Circle":{"radius":1.0}}`, `"Empty"` | `{ Circle: { radius: number } } \| "Empty"` |
| `#[serde(tag = "type")]` | `{"type":"Circle","radius":1.0}`, `{"type":"Empty"}` | `{ type: "Circle", radius: number } \| { type: "Empty" }` |
| `#[serde(tag = "t", content = "c")]` | `{"t":"Circle","c":{"radius":1.0}}`, `{"t":"Empty"}` | `{ t: "Circle", c: { radius: number } } \| { t: "Empty" }` |
| `#[serde(untagged)]` | `{"radius":1.0}`, `null` | `{ radius: number } \| null` |

The Rust client's enum has the same serde attributes. Python gets a model per
variant and a union of them.

- `rename`, `rename_all` and the other serde attributes on variants and
  fields are reflected too.
- An untagged unit variant, such as `Empty` above, is written as `null`.
  `build()` refuses two of them in one enum, since `null` reads back as the
  first, and one inside an `Option` or `reflectapi::Option`, since `null`
  reads back as `None`.
- An enum whose responses may gain variants can be marked
  `#[reflectapi(open)]`, so that clients keep values with unfamiliar tags; see
  [Enum evolution](../getting-started/attributes.md#enum-evolution).
- A unit enum that serde writes as numbers, for example with the
  `serde_repr` crate, needs `#[reflectapi(discriminant)]`, so that clients use
  its discriminants (`1 | 2` in TypeScript) instead of its variant names.

## Newtypes

A tuple struct with one field, `struct UserId(u64)`, is written by serde as
its field, and so is a struct marked `#[serde(transparent)]`. The clients
alias it: `type UserId = number` in TypeScript and `pub type UserId = u64` in
Rust; Python uses `int` in its place. The server keeps the distinct type; the
clients see the inner one.
[Inline newtypes](./organising.md#inline-newtypes) removes the alias names from
the clients too.

## One type, two shapes

A type is reflected once as input and once as output, and the two can
differ:

- a field can be optional in a request (`#[serde(default)]`) but always present
  in a response;
- `#[serde(skip_deserializing)]` or `#[reflectapi(input_skip)]` leaves a field
  out of the input, and `#[serde(skip_serializing)]` or
  `#[reflectapi(output_skip)]` out of the output;
- `#[reflectapi(input_type = "...")]` and `#[reflectapi(output_type = "...")]`
  describe a field as a different type in each direction.

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, reflectapi::Input, reflectapi::Output)]
struct User {
    /// Set by the server.
    #[serde(skip_deserializing)]
    id: u64,
    name: String,
}

async fn users_create(_: (), user: User, _: reflectapi::Empty) -> User {
    User { id: 1, ..user }
}

# fn main() -> Result<(), reflectapi::BuildErrors> {
let (schema, _) = reflectapi::Builder::new()
    .route(users_create, |b| b.name("users.create"))
    .build()?;

let fields = |types: &reflectapi::Typespace| {
    let user = types.types().find(|t| t.name().ends_with("::User")).unwrap();
    user.as_struct().unwrap().fields().map(|f| f.name().to_owned()).collect::<Vec<_>>()
};
assert_eq!(fields(schema.input_types()), ["name"]);
assert_eq!(fields(schema.output_types()), ["id", "name"]);
# Ok(())
# }
```

When the two shapes differ, the clients have two types, in `input` and
`output` namespaces: `User` becomes `input.User`, which the client sends,
and `output.User`, which it receives. A type whose shapes are the same stays
one type. [Missing fields and null](../getting-started/attributes.md#missing-fields-and-null)
has the rules for which fields are required in each direction.

## Option and reflectapi::Option

`std::option::Option<T>` has two states, a value or null. In a request, a
missing field also reads as `None`, so the server can't tell "not sent" from
`null`. For a partial update, where leaving a field out means "keep it" and
`null` means "clear it", use
[`reflectapi::Option<T>`](https://docs.rs/reflectapi/latest/reflectapi/enum.Option.html),
which has three: `Undefined`, `None` and `Some(T)`.

```rust
#[derive(serde::Serialize, serde::Deserialize, reflectapi::Input)]
struct PetPatch {
    #[serde(default, skip_serializing_if = "reflectapi::Option::is_undefined")]
    age: reflectapi::Option<u8>,
}

let patch: PetPatch = serde_json::from_str("{}").unwrap();
assert!(patch.age.is_undefined());
let patch: PetPatch = serde_json::from_str(r#"{"age":null}"#).unwrap();
assert!(patch.age.is_none());
let patch: PetPatch = serde_json::from_str(r#"{"age":3}"#).unwrap();
assert_eq!(patch.age, reflectapi::Option::Some(3));

let undefined = PetPatch { age: reflectapi::Option::Undefined };
assert_eq!(serde_json::to_string(&undefined).unwrap(), "{}");
```

Reading needs `#[serde(default)]`: without it a missing field reads as
`None`. Writing needs `skip_serializing_if`: without it `Undefined` is written
as `null`. A request type the server only reads needs just
`#[serde(default)]`; this example, read and written, needs both. The clients keep the three states: the field is
`age?: number | null | undefined` in TypeScript, a `reflectapi::Option<u8>` in
Rust, and a Python model sends only the fields that were set. See
[Missing fields and explicit null](../clients/index.md#missing-fields-and-explicit-null).

## Empty and Infallible

[`reflectapi::Empty`](https://docs.rs/reflectapi/latest/reflectapi/struct.Empty.html)
is a struct with no fields, `{}` on the wire. Use it for a route with no
input, no output or no request headers. A request with an empty body is read
as `{}`, so an `Empty` input accepts it. `()` is `null` on the wire instead.

[`reflectapi::Infallible`](https://docs.rs/reflectapi/latest/reflectapi/enum.Infallible.html)
is the error type of a route that can't fail. See
[Errors](./errors.md#routes-that-cant-fail).

```rust
assert_eq!(serde_json::to_string(&reflectapi::Empty {}).unwrap(), "{}");
assert_eq!(serde_json::to_string(&()).unwrap(), "null");
```

## Lengths

[`reflectapi::Bounded<T, MIN, MAX>`](https://docs.rs/reflectapi/latest/reflectapi/struct.Bounded.html)
is a string or collection whose length is within `MIN..=MAX`, inclusive.
`BoundedMax<T, MAX>` sets only the upper bound. On the wire it is the `T` it
wraps. A request with another length fails to deserialize, so it's rejected
with HTTP 400 before the handler runs. Strings count characters (Unicode code
points, not bytes); collections count entries.

```rust
use reflectapi::{Bounded, BoundedMax};

#[derive(serde::Deserialize, reflectapi::Input)]
struct TagPet {
    /// Up to 50 tags, each 1 to 32 characters long.
    tags: BoundedMax<Vec<Bounded<String, 1, 32>>, 50>,
}

assert!(serde_json::from_str::<TagPet>(r#"{"tags":["friendly"]}"#).is_ok());
assert!(serde_json::from_str::<TagPet>(r#"{"tags":[""]}"#).is_err());
```

The generated clients check the bounds too, so a request with another length
fails before it is sent:

- **TypeScript**: the field's type is `Bounded<Array<Bounded<string, 1, 32>>, 0, 50>`,
  a type of its own that a plain array or string doesn't fit. Make one with
  `bounded(value, MIN, MAX)`, which checks the length and throws a
  `BoundsError`, or `bounded.try(value, MIN, MAX)`, which returns a `Result`.
  The bounds must be the field's: other bounds don't compile. Values from a
  response already have the type.

  ```ts
  import { bounded } from "./generated";

  const tag = bounded("friendly", 1, 32);
  await client.pets.tag({ tags: bounded([tag], 0, 50) }, {});
  ```

- **Python**: the field is `Annotated[list[Annotated[str, Field(min_length=1, max_length=32)]], Field(max_length=50)]`,
  so building the model raises `pydantic.ValidationError`.
- **Rust**: the field is `reflectapi::Bounded` itself: build it with
  `Bounded::new(value)?` or `try_from`, or `Bounded::from_array` for a length
  checked at compile time.
- **OpenAPI**: `minLength`/`maxLength` for strings, `minItems`/`maxItems` for
  arrays and sets, `minProperties`/`maxProperties` for maps.

A fixed-size array, `[T; N]`, has a length too: exactly `N`. The server and
the Rust client enforce it. In TypeScript, an array literal of the right
length fits `FixedSizeArray<T, N>` as it is, and an array built at runtime
goes through `fixedSizeArray(items, N)` (or `fixedSizeArray.try`), which checks
it. Python gets `Field(min_length=N, max_length=N)`, OpenAPI
`minItems`/`maxItems`.

## Raw JSON

[`reflectapi::Raw<T>`](https://docs.rs/reflectapi/latest/reflectapi/struct.Raw.html),
with the `json` feature, is a `T` kept as the JSON it arrived as. The schema
and the clients see a `T`; the server keeps the text, so it can store or
forward the value unchanged, read a few fields with `peek` or decode it with
`decode`. Fields and enum variants the server's own types don't know survive.

With a MessagePack request or response the value is converted to and from
JSON text: key order and every field survive, but the text isn't kept byte
for byte, and a value JSON can't represent, such as binary data or NaN, is an
error.

## Encoded values

[`reflectapi::Encoded<T>`](https://docs.rs/reflectapi/latest/reflectapi/struct.Encoded.html),
with the `json` feature, is a typed value encoded into a string, for clients
to store and send back without reading it, such as a page cursor. The server works with the typed `T`; the
wire, the schema and the clients see a `string`: `T` as JSON, then base64url
without padding.

```rust
use reflectapi::Encoded;

#[derive(serde::Serialize, serde::Deserialize)]
struct Cursor {
    after_id: u64,
}

let cursor = Encoded(Cursor { after_id: 42 });
assert_eq!(serde_json::to_string(&cursor).unwrap(), r#""eyJhZnRlcl9pZCI6NDJ9""#);
```

In a generic type, a parameter used only inside `Encoded`, such as `C` in a
field `Option<Encoded<C>>`, is left out of the schema with its arguments, so
clients never see the type it holds.

It is encoded, not secret: anyone can decode the string and send back a
changed one. Check what you decode as you would any other input, and don't
put in it anything the client mustn't see.

## Conventions

[`reflectapi::conventions`](https://docs.rs/reflectapi/latest/reflectapi/conventions/index.html),
with the `conventions` feature, has the types behind the
[API Design Standard](../design/standard.md): cursor pagination, identity
envelopes and conflict resolution. See [Convention Types](../design/conventions.md)
for how to use them.

## Types reflectapi doesn't know

A type that implements neither trait, such as `std::net::IpAddr`, can't be
used as it is. Tell reflectapi which known type has the same JSON.

For a field, override its type. Serde writes an `IpAddr` as a string:

```rust
#[derive(serde::Serialize, reflectapi::Output)]
struct Server {
    #[reflectapi(type = "String")]
    address: std::net::IpAddr,
    #[reflectapi(type = "Vec::<String>")]
    fallbacks: Vec<std::net::IpAddr>,
}
# fn main() {
#     let _ = Server { address: [127, 0, 0, 1].into(), fallbacks: vec![] };
# }
```

- `type` sets both directions; `input_type` and `output_type` set one.
- The string is a Rust type path. A generic type needs the turbofish:
  `"Vec::<String>"`, not `"Vec<String>"`.
- reflectapi doesn't check that serde writes what the override says. If they
  differ, the clients are wrong.

For a type of your own whose serde implementation is written by hand, put the
override on the type: it is reflected as the given type, under its own name.
Serde's `#[serde(into = "...")]`, `from` and `try_from` are followed the same
way, with no override needed:

```rust
use serde::{Deserialize, Serialize};

/// A version, written as "1.2".
#[derive(Clone, Serialize, Deserialize, reflectapi::Input, reflectapi::Output)]
#[serde(into = "String", try_from = "String")]
struct Version {
    major: u32,
    minor: u32,
}

impl From<Version> for String {
    fn from(v: Version) -> String {
        format!("{}.{}", v.major, v.minor)
    }
}

impl TryFrom<String> for Version {
    type Error = String;

    fn try_from(s: String) -> Result<Self, String> {
        let (major, minor) = s.split_once('.').ok_or("expected major.minor")?;
        let number = |s: &str| s.parse().map_err(|_| format!("not a number: {s}"));
        Ok(Version { major: number(major)?, minor: number(minor)? })
    }
}

async fn version(_: (), v: Version, _: reflectapi::Empty) -> Version {
    v
}

# fn main() -> Result<(), reflectapi::BuildErrors> {
let (schema, _) = reflectapi::Builder::new()
    .route(version, |b| b.name("version"))
    .build()?;
let version = schema
    .output_types()
    .types()
    .find(|t| t.name().ends_with("::Version"))
    .unwrap();
let field = version.as_struct().unwrap().fields().next().unwrap();
assert_eq!(field.type_ref().name(), "std::string::String");
# Ok(())
# }
```

The TypeScript client gets `type Version = string`. With a hand-written
`Serialize`, `#[reflectapi(type = "String")]` on the struct does the same.

`Input` and `Output` can also be implemented by hand. The simplest
implementation returns another type's reflection:

```rust
/// A temperature, written as a number.
struct Celsius(f64);

impl serde::Serialize for Celsius {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl reflectapi::Output for Celsius {
    fn reflectapi_output_type(schema: &mut reflectapi::Typespace) -> reflectapi::TypeReference {
        f64::reflectapi_output_type(schema)
    }
}
```

Unlike an override, this gives the type no name of its own: clients see an
`f64`. The orphan rule applies: a trait from `reflectapi` can be implemented
only for a type of your own crate, so use an override for a field of a type
from another crate.

## TypeScript types chosen by the client

A TypeScript client can write a schema type as a type of its own choosing,
such as a UUID type from a package it already uses. The server's schema stays
as it is, so two clients of one API can choose differently.

```rust
use reflectapi::codegen::typescript::{generate, Config};

#[derive(serde::Serialize, reflectapi::Output)]
struct Order {
    id: uuid::Uuid,
}

async fn order(_: (), _: reflectapi::Empty, _: reflectapi::Empty) -> Order {
    Order { id: uuid::Uuid::nil() }
}

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let (schema, _) = reflectapi::Builder::new()
    .route(order, |b| b.name("order"))
    .build()?;
let files = generate(
    schema,
    Config::default().type_override("uuid::Uuid", r#"import("@acme/ids").Uuid"#),
)?;
assert!(files["generated.ts"].contains(r#"id: import("@acme/ids").Uuid"#));
# Ok(())
# }
```

The CLI takes the same as `--typescript-type`, once per type:

```bash
reflectapi codegen --language typescript \
  --typescript-type 'uuid::Uuid=import("@acme/ids").Uuid'
```

- **The type** is any TypeScript type. An import type, `import("…")`, needs
  no import statement. A bare name is looked up inside the generated
  namespaces, so it can be shadowed by a generated type of the same name.
- **It must be assignable to the schema type.** `generated.ts` checks each
  override against the type it replaces, as that type is on the wire, so
  `tsc` refuses an override that a request would send as something else. A
  `Date` for a `chrono::DateTime` fails with
  `Type 'Date' does not satisfy the constraint 'string'`; a branded string
  passes.
- **Responses aren't converted.** A value from a response is the JSON
  value, typed as the override, so the override should describe that value:
  a narrower string type, a branded string or a readonly view, not a class.
- **The name is the Rust type's.** A type the builder splits into `input` and
  `output` types, because requests and responses differ, is replaced in both,
  and must be assignable to both. A type with type parameters can't be
  overridden, except one written as a string, such as `chrono::DateTime`.
  Naming a type the schema doesn't have is an error.
