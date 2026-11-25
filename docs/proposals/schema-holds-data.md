# Proposal: the schema holds data, reflectapi holds meaning

Goal: `reflectapi-schema` is the minimal data that describes an API's types and
endpoints. Knowing what those types mean, which is what diff, the codegens and
the builder need, lives in `reflectapi` under feature flags.

Status: what the next PR does. It touches #241 (released in 0.26), #263
(merged, unreleased) and #256 (open). Breaking changes are acceptable for now.
`reflectapi::Bounded` was taken out of #265 and comes back in its own PR; its
design is below, in [`Bounded`](#bounded-a-separate-pr), for review.

## The principle

A type's name and its arguments say everything about the type. `uuid::Uuid`
is a UUID. `std::array::Array<u8, 3>` is an array of exactly 3 bytes. Each
tool recognises the names it knows and falls back for the rest. The codegens already work this way, with about 55
entries keyed by name.

The schema stores data only where the name can't give it. Those are serde's
attributes, which change the JSON without changing the Rust types:

| Rust | The schema without the field | The field |
|---|---|---|
| `#[serde(rename = "petName")] pet_name: String` | a field `pet_name` of type `String` | `serde_name`: the key is `"petName"` |
| `#[serde(flatten)] address: Address` | a field `address` of type `Address` | `flattened`: `Address`'s keys sit in the parent |
| `#[serde(default)] limit: u32` | a field `limit` of type `u32` | `required: false`: the key may be absent |
| `#[serde(tag = "kind")] enum Event` | an enum and its variants | `representation`: `{"kind": "created", …}` |
| `#[serde(other)] Unknown` | a variant with no data | `other`: an unknown tag reads as this variant |

`Primitive.format` (#241) is the one exception: it restates what the name
already says.

## The problem: a type's wire form depends on the format

A function can accept and return JSON or MessagePack (`Function.serialization`).
Some types are written differently in each, because their `Serialize` checks
`is_human_readable()`. serde_json is human-readable, and rmp-serde, which the
builder uses with its default config, is not:

| Type | JSON | MessagePack |
|---|---|---|
| `uuid::Uuid` | `"e7fe9a2c-1b7f-4a43-9d63-2c1f0b6f5a10"` | 16 bytes (`bin 8`) |
| `reflectapi::Raw<T>` | the JSON text as it is | the value the text holds |

serde's own `std::net` types do the same, but reflectapi doesn't reflect them
today.

`wire` knows only the JSON forms. `wire::format` says `uuid::Uuid` is a
`uuid` string whatever the format, and `Primitive.format` can only hold that
one answer. So a MessagePack client written from the schema sends a UUID as a
string, and the server rejects it:

```
invalid type: string "e7fe9a2c-1b7f-4a43-9d63-2c1f0b6f5a10", expected a 16 byte array
```

Nothing hits this yet, because no generated client speaks MessagePack and
OpenAPI leaves MessagePack bodies out (`// TODO msgpack` in `openapi.rs`).
Compatibility isn't affected. It's judged on types: `uuid::Uuid` narrows
`String` whatever the format, as #256 already treats it. How a format writes
a value is the codec's concern, and a receiver is expected to read a type in
either form. A server reads what its types' `Deserialize` accepts, and in
MessagePack the `uuid` crate's accepts only bytes, as the error above shows.
The guide advises servers how to accept both (change 4).

## Changes

### 1. Diff moves to `reflectapi::diff`, behind a `diff` feature

To judge compatibility, diff has to know what types mean, just as a codegen
does. Today it lives in `reflectapi-schema` (#256), and that placement is
the reason #263 moved serde's rules down into the data crate. Instead, diff
joins the codegens in `reflectapi`, behind its own feature:

```toml
# reflectapi/Cargo.toml
diff = []

# reflectapi-cli/Cargo.toml
reflectapi = { …, features = ["codegen", "diff"] }
```

#256 is still open, so it can move before it ships.

### 2. `wire` moves back to `reflectapi`

`reflectapi_schema::wire` holds routines that read the schema and work out
what it means:

| Module | Decides |
|---|---|
| `format` | a primitive's string format, and integer widths and ranges, by name |
| `layout` | the keys a flattened field adds to its parent |
| `shape` | unit, newtype, tuple or struct, and so whether a newtype is written as its field |
| `key`, `presence`, `nullability` | whether a key may be absent, and whether `null` is accepted |
| `headers` | the headers a request-headers type reads |

These routines are the layer between the data and the tools. All of the
tools that use them are in `reflectapi`:

```
reflectapi-schema    data: types, endpoints; structural helpers (subst, visit, rename)
        ▲
reflectapi
  ├─ wire            meaning of names and serde attributes,
  │                  built when any of builder, codegen or diff is on
  ├─ builder         uses wire
  ├─ codegen         uses wire
  └─ diff            uses wire
```

This reverses #263, which is unreleased and kept `wire` `#[doc(hidden)]`, so
no published API changes. `wire` would only need a crate of its own if
something outside `reflectapi` came to need it.

### 3. `Primitive.format` is removed

The format follows from the name. `wire::format` already has the table
(`uuid::Uuid` → `uuid`, `chrono::DateTime` → `date-time`, …) and uses it
whenever the field is absent. That table becomes the only source.

- **JSON:** compatible both ways. A missing `format` already means "derive it
  from the name", and readers ignore unknown keys. No reader changes
  behaviour, so the format version stays at 1. The changelog notes this as an
  exception to the "removing a field bumps the version" rule.
- **Rust API:** `Primitive::with_format` and `Primitive::format()` go away.
  That's a breaking change, for 0.27.
- **A user's own primitive** can no longer declare a format in the schema.
  Its format is mapped in the tool instead: OpenAPI config, or a TypeScript
  type override (#262).
- **Diff** reads the format from the name table instead of `p.format`.

### 4. `wire` answers per format

The routines that describe a value's form take the serialization format, and
the tables keyed by name record each form:

```rust
pub fn wire_format(name: &str, mode: SerializationMode) -> Option<WireForm>;

// "uuid::Uuid", Json    → a string, format "uuid"
// "uuid::Uuid", Msgpack → 16 bytes
```

- A tool describing a MessagePack body asks for `Msgpack`: OpenAPI's
  `application/msgpack` content, and a future MessagePack client.
- Diff doesn't use it. Compatibility is judged on types, whatever the format:
  `String` → `uuid::Uuid` narrows, so it breaks a request and not a response.
- A generated MessagePack decoder reads a type in either form, string or
  bytes.
- The names already hold this information. A single `format` field couldn't,
  which is one more reason to remove it (change 3).

A server's tolerance is up to its types, so the guide advises rather than
reflectapi enforcing it. [On the Wire](../src/concepts/wire.md) gains a
note for servers that take MessagePack: read a type in either form. For a
UUID that means a `deserialize_with` function that accepts a string or 16
bytes:

```rust
#[derive(serde::Deserialize, reflectapi::Input)]
struct PetsGetRequest {
    #[serde(deserialize_with = "uuid_from_str_or_bytes")]
    id: uuid::Uuid,
}

// `deserialize_any` with a visitor: `visit_str` parses the text,
// `visit_bytes` takes 16 bytes.
fn uuid_from_str_or_bytes<'de, D: serde::Deserializer<'de>>(d: D) -> Result<uuid::Uuid, D::Error>
```

The schema still names the field `uuid::Uuid`.

Not chosen: running rmp-serde as human-readable, so that MessagePack matches
JSON. MessagePack is there for performance, and writing strings in place of
bytes would undo that. Also not chosen: reflectapi making servers tolerant
automatically. That's left for later.

A user's own type whose `Serialize` checks `is_human_readable()` remains
outside the schema's knowledge, as any custom `Serialize` is today. Its author
would describe it through a mapping declared by the type (see "Left for
later").

### 5. Const arguments are documented

`TypeReference::arguments` says its entries are "references to actual types".
But `[T; N]` is written as `std::array::Array<T, "N">`, as the
`test_reflectapi_struct_with_fixed_size_array` snapshot shows:

```json
{"name": "std::array::Array", "arguments": [{"name": "u8"}, {"name": "3"}]}
```

The doc comment says that an argument can be a const value. A reader that
doesn't know `std::array::Array` follows its fallback to `Vec<T>`.

## Order

1. `wire` back to `reflectapi` (change 2). The codegens' and builder's
   imports change; nothing else does.
2. #256 lands as `reflectapi::diff` (change 1).
3. `Primitive.format` removed, and `wire` answers per format (changes 3
   and 4).
4. Const arguments documented (change 5). This can happen at any point.

`Bounded` comes after, in its own PR.

## `Bounded`: a separate PR

`reflectapi::Bounded` was implemented on #265 and taken out for review. It
comes back in its own PR, after the changes above, with the decisions below.
Length checks for fixed-size arrays came out with it, since they share its
code, and come back with it too.

### The type

`Bounded<T, MIN, MAX>` is a string or collection whose length is within
`MIN..=MAX`, inclusive. `BoundedMax<T, MAX>` sets only the upper bound. Its
source, as implemented on #265, is in [`bounded.rs`](bounded.rs).

```rust
use reflectapi::{Bounded, BoundedMax};

#[derive(serde::Deserialize, reflectapi::Input)]
struct PetsTagRequest {
    /// Up to 50 tags, each 1 to 32 characters long.
    tags: BoundedMax<Vec<Bounded<String, 1, 32>>, 50>,
}
```

- **On the wire** it is the `T` it wraps, in every format.
- **The server** fails to deserialize a value of another length, so the
  request is rejected with HTTP 400 before the handler runs.
- **Lengths** are characters for strings (code points, not bytes), and
  entries for `Vec`, `VecDeque`, `HashSet`, `BTreeSet`, `HashMap` and
  `BTreeMap`. Another type takes part by implementing `reflectapi::Length`.
- **Building one:** `Bounded::new(value)?`, `TryFrom`, `from_array([…])` with
  the length checked at compile time, and `chunks(&items)` to split a long
  slice into bounded batches. Out of bounds is a `BoundsError { length, min,
  max }`. `Deref`, `AsRef` and `into_inner` give the value back.
- **No feature flag.** It needs only serde and reflectapi's own traits, and
  the codegens recognise it by name, which they couldn't do if it depended on
  a flag set in the user's crate. `Option`, `Empty` and `Infallible` are
  flag-free for the same reason.

### In the schema

A primitive `reflectapi::Bounded`, with parameters `T`, `MIN` and `MAX` and
fallback `T`. The limits are const arguments, as `[T; N]`'s `N` is
(change 5):

```json
{"name": "reflectapi::Bounded", "arguments": [
  {"name": "std::string::String"}, {"name": "1"}, {"name": "32"}
]}
```

A reader that doesn't know `reflectapi::Bounded` follows its fallback to `T`.

### In each client

Every client checks the limits before sending, so a bad request fails locally
with a clear error instead of a 400:

| Client | Type | Checks |
|---|---|---|
| TypeScript | `Bounded<Array<Item>, 0, 500>`, a branded type made with `bounded(value, MIN, MAX)` | when a request is built ([branding](#branding-in-typescript)) |
| Rust | `reflectapi::Bounded<Vec<Item>, 0, 500>`, the real type | when a request is built, and when a response is decoded |
| Python | `Annotated[list[Item], Field(max_length=500)]`, with `min_length` when `MIN > 0` | when a model is built or parsed; `len(str)` counts code points, as Rust does |
| OpenAPI | `allOf` of the `T` and `minLength`/`maxLength`, `minItems`/`maxItems` or `minProperties`/`maxProperties` | for tools generated from it |

Fixed-size arrays get the same checks: Python `Field(min_length=N,
max_length=N)`, OpenAPI `minItems`/`maxItems`, and in TypeScript
`fixedSizeArray(items, N)` for an array built at runtime. An array literal of
the right length fits `FixedSizeArray<T, N>` as it is.

A user type implementing `Length` gets no client checks, since a client can't
measure it.

### Where the length rules live

One routine, `length_bounds`, reads the limits of a `Bounded` or a `[T; N]`
from its arguments, and `length_kind` says what a length counts: characters,
items or entries. It goes into `wire` as `wire::length` (change 2), shared by
the codegens and diff.

Diff then judges `Bounded` by its limits. Reading #256's code, here is what
it reports without them:

| Change on an input | #256 without `wire::length` | Correct |
|---|---|---|
| `Bounded<String, 1, 32>` → `Bounded<String, 1, 64>` | breaking: the arguments `"32"` and `"64"` resolve to no type, so it reports a type change | compatible; breaking on an output |
| `String` → `Bounded<String, 1, 32>` | compatible: `Bounded` falls back to `String`, so it reports a rename | breaking; compatible on an output |

### Not chosen: limits as data in the schema

Review suggested storing the limits as schema fields, as `format` stores a
format. Each of its concerns is met without new data:

- **Diff can't read the limits.** It reads them through `wire::length` once it
  shares a crate with that routine (changes 1 and 2).
- **Readers see types named `"1"` and `"500"`.** That's the documented const
  argument encoding, already used by arrays (change 5).
- **It doesn't extend to patterns or numeric ranges.** Const generics take
  every integer type, signed ones included, so an integer range works the same
  way. A pattern is better as a named type, as `uuid::Uuid` is. Only float
  ranges have no form.

Limit fields would restate what the name says, as `format` does, and every
tool would then need rules for both.

### Branding in TypeScript

TypeScript compares types by structure, so `type Bounded<T, MIN, MAX> = T`
would be exactly `T`, and any `string[]` would fit a `Bounded<string[], 0, 50>`
field. A brand intersects the type with a property that exists only for the
compiler. At runtime the value is still the plain `T`, and the wire is
unchanged. A plain `T` lacks the property, so the only way to get a `Bounded`
is the cast `bounded()` makes after checking:

```ts
import { bounded, BoundsError } from "./generated";

const tag = bounded("red", 1, 32);                     // Bounded<string, 1, 32>, or throws BoundsError
await client.pets.tag({ tags: bounded([tag], 0, 50) }, {});

const name = bounded.try(input.value, 1, 32);          // Result<Bounded<string, 1, 32>, BoundsError>

client.pets.tag({ tags: ["red"] }, {});                // compile error: string[] isn't a Bounded
client.pets.tag({ tags: bounded([tag], 0, 100) }, {}); // compile error: other limits
```

- The caller passes the limits, because types are erased at runtime.
  TypeScript infers them as literal types, so limits that differ from the
  schema's don't compile, and a call site stops compiling when the server
  changes a limit.
- A value from a response is typed as branded with no check, since the server
  guarantees it, and can be sent back as it is.
- `__Wire<T>`, the plain JSON shape that type overrides (#262) are checked
  against, strips the brand.
- Nesting is verbose: `bounded(names.map((n) => bounded(n, 1, 32)), 0, 50)`.

#### Which brand

`#[reflectapi(distinct)]` already brands, through `__Distinct`. A second
brand for `Bounded` makes three mechanisms, counting #262's overrides, and
review asked for one model. The overrides aren't a brand reflectapi defines:
they replace the type with the client's own, checked against `__Wire`. So
the choice is between two brands and one:

```ts
// A. Its own brand, as implemented on #265.
declare const __bounded: unique symbol;
type Bounded<T, MIN extends number, MAX extends number> =
  T & { readonly [__bounded]: { readonly base: T; readonly min: MIN; readonly max: MAX } };

// B. Folded into today's __Distinct: the limits go into the name.
type Bounded<T, MIN extends number, MAX extends number> =
  __Distinct<T, `reflectapi::Bounded<${MIN}, ${MAX}>`>;

// C. Folded into a __Distinct keyed by name, with __Wire reading the key's value.
type __Distinct<T, Name extends string> = T extends null | undefined
  ? T
  : T & { readonly [__distinct]: { readonly [K in Name]: __Wire<T> } };
type __Wire<T> = IsAny<T> extends true ? T
  : T extends { readonly [__distinct]: infer M } ? M[keyof M]
  : T extends object ? { [K in keyof T]: __Wire<T[K]> } : T;
type Bounded<T, MIN extends number, MAX extends number> =
  __Distinct<T, `reflectapi::Bounded<${MIN}, ${MAX}>`>;
```

Checked with `tsc` 5.9 in strict mode, where `Tag` and `Label` are distinct
types over one `Bounded<String, 1, 32>`, and `OrderId` and `ShopId` are
distinct types over a distinct `ProductId`:

| Check | A | B | C |
|---|---|---|---|
| a plain `string` isn't a `Bounded`, and other limits don't fit | ✓ | ✓ | ✓ |
| `Tag` and `Label` aren't assignable to each other | ✓ | ✗ | ✓ |
| `OrderId` and `ShopId` aren't assignable to each other | ✗ | ✗ | ✓ |
| `__Wire<Tag>` is `string` | ✓ | ✗ `unknown` | ✓ |
| `__Wire<OrderId>` is `string` | ✗ `unknown` | ✗ `unknown` | ✓ |

Chosen: **C**, one brand keyed by name. A nested brand adds a key instead of
clashing with the one it wraps. Generated code doesn't change: it still writes
`__Distinct<wrapped, "name">` and `Bounded<string, 1, 32>`. Only the `min` and
`max` in the brand go, and nothing reads them.

#### A bug in `__Distinct` today

Column A's ✗s aren't about `Bounded`: they are today's `__Distinct` on main.
Every brand sits on the one symbol `__distinct`, so a distinct type over
another distinct type, such as `struct OrderId(ProductId)`, carries two
`name`s that intersect into `never`:

- two such types over the same inner type are assignable to each other, so
  the brand doesn't keep them apart;
- `__Wire` of such a type is `unknown`, so a type override that reaches it is
  checked against nothing.

Folding `Bounded` in as it is (B) spreads the bug to every distinct type over
a `Bounded`. C fixes it, and can land on its own, before `Bounded`.

#### Other ways to check, not chosen

- **A per-request check**, `PetsTagRequest.check({ tags: ["red"] })`, through
  the descriptor decoder the client already ships for open enums: one call
  per request with the schema's limits, but a value can't be checked once and
  reused, and each such request type gets a generated function.
- **A validator export** that callers run themselves: easy to forget.

Python and Rust need no new API, since building the value checks it.

### Migrating from a copy of the type

A service that has its own bounded type re-exports reflectapi's. Its schema
name becomes `reflectapi::Bounded`, so its generated clients change, and a
Rust codegen `shared_modules` entry for the old type goes away.

## Left for later

- **Mappings declared by the type.** Every type has a `codegen_config`, which
  today holds only `rust.additional_derives`. A Python section for type hints
  and imports was tried in March 2026 and moved to the Python backend before
  it merged; the reason wasn't recorded. A mapping declared by the type would
  let a crate outside reflectapi choose its client types without changes to
  reflectapi. It would sit between a client's override (#262) and the
  codegen's own table, and it would have to be in the JSON for the CLI to see
  it.
- **How `wire` is exposed.** `pub(crate)` is enough while every tool using it
  is in `reflectapi`.
