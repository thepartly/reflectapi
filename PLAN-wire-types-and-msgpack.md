# Plan: wire types and MessagePack

Goal: give reflectapi a few wire types that downstream services keep
re-implementing (`Bounded`, `Raw`, `Encoded`), and make the
existing opt-in MessagePack support work end to end, including for `Raw`.

Status: the first part is implemented, uncommitted, on
`andrey/core-wire-types` (see [Done on this branch](#done-on-this-branch)
and [Remaining](#remaining)). The rest is proposed. The measurements behind the decisions are in
[Benchmarks](#benchmarks), with the code in [`bench/formats`](bench/formats).

## Summary of decisions

| Topic | Decision |
|---|---|
| `Bounded<T, MIN, MAX>` | In core, no feature flag. Done |
| `reflectapi::Empty` | Also derives `Copy` and `Hash`. Done |
| `Raw<T>` | Under `json`, stored as JSON text by definition; converted at the edge for MessagePack. Done |
| `Encoded<T>` (first called `Opaque`) | Under `json`, JSON then base64url without padding. Done |
| MessagePack | Keep, opt-in (`msgpack` feature), JSON stays the default. Make it complete: `Raw`, `accept`, client opt-in |
| Proxies | A separate `RawBody<T>`: the whole body as bytes in whichever format arrived |
| CBOR, BSON, other binary JSON | Not adding. MessagePack is as fast and small, and better supported |
| `sonic-rs` | Optional later, for `Raw::peek` only, with a serde_json fallback on wasm without `simd128` |

## Done on this branch

### `Bounded<T, MIN, MAX>`, `BoundedMax<T, MAX>`, `Length`

`reflectapi/src/bounded.rs`. A string or collection whose length is within
`MIN..=MAX`, inclusive. On the wire it is the `T` it wraps. Deserializing
another length fails, so the request is rejected with HTTP 400 before the
handler runs. Strings count characters, collections count entries
(`Vec`, `VecDeque`, `HashSet`, `BTreeSet`, `HashMap`, `BTreeMap`).
`Bounded::new`, `TryFrom`, `from_array` (length checked at compile time),
`chunks` (split a long slice into bounded batches), `into_inner`, `Deref`.

Why no feature flag: it needs only serde and reflectapi's own traits, it is
small, and the codegens recognise it by name, which they can't do if it
depends on a flag set in the user's crate (the CLI is built separately).
`Option`, `Empty` and `Infallible` are flag-free for the same reason.

Schema: a primitive `reflectapi::Bounded` with parameters `T`, `MIN`, `MAX`
and fallback `T`, the same pattern as fixed-size arrays
(`std::array::Array<T, N>`, fallback `Vec<T>`). The bounds are literal type
arguments: `Bounded<Vec<Item>, 0, 500>`.

How each generated client sees it:

| Client | Type | Checks the bounds? |
|---|---|---|
| TypeScript | `Bounded<Array<Item>, 0, 500>`, a branded type (`codegen/lib.ts`) made with `bounded(value, MIN, MAX)` | Yes, when building a request ([design](#branded-bounded-in-typescript)) |
| Rust | `reflectapi::Bounded<Vec<Item>, 0, 500>`, the real type | Yes, when building a request and when decoding |
| Python | `Annotated[list[Item], Field(max_length=500)]` (`min_length` when `MIN > 0`) | Yes, when building a model and when parsing |
| OpenAPI | `allOf` of the `T` and `minLength`/`maxLength`, `minItems`/`maxItems` or `minProperties`/`maxProperties` | Tools generated from it can |

### Codegen fix: parameters filled in one pass

TypeScript and Rust filled a built-in type's template parameters one at a
time with `replacen`, so a parameter's name could be found inside an argument
already filled in (`N` inside a type named `Node` in `FixedSizeArray<T, N>`,
or `MIN` inside `ADMIN` in `Bounded<T, MIN, MAX>`). Both now use
`codegen::substitute_type_parameters`, one pass over the template's own
identifiers, with a unit test.

### `Raw<T>` (feature `json`)

`reflectapi/src/raw.rs`. A `T` kept as the JSON text it arrived as; the
schema and clients see `T`. `new(String)`, `from_value`, `json()`,
`peek::<V>()` (reads a few fields into a view that may borrow from the text),
`decode()`. `json` now enables serde_json's `raw_value` feature.

Use cases it serves:

- **Returning stored JSON**: a service streams documents read from a
  database as JSON text. With `Raw` it sends the text as is instead of
  decoding each into its struct and encoding it again, and clients still get
  typed values.
- **Forwarding**: a proxy takes a request as `Raw`, reads only what it needs
  with `peek` (for an access check), and forwards the text. Fields and enum
  variants its own copy of the types doesn't know survive, so the upstream
  can change without redeploying the proxy.

So `Raw` buys speed (no decode and re-encode), fidelity (unknown fields
survive) and typed clients.

**Fixed on this branch (option 1 below):** under MessagePack, a request with a `Raw` field
failed (`invalid type: newtype struct, expected any valid JSON value`), and a
response was silently wrong: serde_json's private marker was written as data,
so the client got `{"$serde_json::private::RawValue": "{\"a\":1}"}` instead
of `{"a":1}`. See [Raw under MessagePack](#raw-under-messagepack).

### `Empty` derives `Copy` and `Hash`

So an enum holding it can derive both. A downstream copy existed only for
this.

### Tests and docs

Unit tests for `Bounded`, `Raw` and the substitution; a snapshot test across
every codegen (`reflectapi-demo/src/tests/basic.rs`,
`test_reflectapi_struct_with_bounded`); "Lengths" and "Raw JSON" sections in
`docs/src/concepts/types.md`; CHANGELOG entries. Workspace tests pass with
`CI=1` (generated TypeScript and Rust type-checked), clippy with
`-D warnings`, doctests, the book's doctests, and builds with no default
features and with only `json`.

## Encoded

`Encoded<T>`: a value clients store and send back without reading it, such as
a page cursor. The server sees a typed `T`; the wire and schema see a
`string`. The downstream version encodes MessagePack then base64.

**Name.** First called `Opaque`, as downstream. Renamed before release:
"opaque" suggests a value the client can't see into (in OAuth, an opaque
token is a random handle), while this one is base64 of JSON that anyone can
decode and change; Rust also uses "opaque type" for `impl Trait`; and the
name is read by server developers (clients see a `string`), for whom
`Encoded` says what it does. `Sealed<T>` or `Signed<T>` stay free for a
future tamper-proof version. Downstream code can migrate through a type
alias.

Dropping MessagePack leaves JSON then base64url, which needs serde_json and
base64 (the crate, or about 40 lines). Options:

1. **Under `json`, next to `Raw`. Chosen and implemented** (`reflectapi/src/encoded.rs`). Both are wire types built on
   serde_json; no new flag. `json` would also pull the small `base64` crate.
2. **Its own `encoded` flag.** Most precise, a flag for one type.
3. **In a future conventions module.** No: it is useful for any token, not
   just cursors, and pagination types there would use it.

It must not go under `msgpack`: that flag turns on server-side MessagePack
bodies and `builder`, which a client wanting encoded cursors doesn't need.

Whatever the choice: document that it is encoded, not secret. Clients can
decode it and send back a forged value, so the server must validate what it
decodes. Signing would be a different type. Settle the format before
publishing: cursors clients hold depend on it.

## MessagePack today

- Opt-in: the `msgpack` feature (default features are just `glob`), which
  also enables `builder`.
- Request: `content-type: application/msgpack` is decoded with rmp-serde
  (`rmp_serde::from_read`); anything else, or none, is JSON. Without the
  feature, `application/msgpack` is rejected.
- Response: always in the request's format
  (`reflectapi/src/builder/handler.rs`). The `accept` header is ignored, so a
  client can't send JSON and ask for MessagePack back.
- Clients: none of the generated clients send MessagePack. The TypeScript
  client sends `application/json`; the Rust runtime and the Python runtime
  have no MessagePack support. Only hand-written callers use it.

### Should we support it?

Yes, opt-in:

- It exists; removing it would break its users.
- Rust servers decode it 1.4–3.7× faster and encode it 1.6–8× faster than
  serde_json; the same holds on wasm. Node clients using `msgpackr` decode it
  faster than `JSON.parse`, especially numbers and text. It is 11–45%
  smaller before compression.

But not the default:

- In browsers, without msgpackr's native addon, standard MessagePack decodes
  typical mixed records 40% slower than `JSON.parse`.
- After gzip or brotli the size advantage mostly disappears, except on
  numeric data.
- JSON is easier to debug.

Making it complete is separate pieces of work: `Raw` under MessagePack,
honouring `accept`, client opt-in, and `RawBody` for proxies
([Work plan](#work-plan)).

## Raw under MessagePack

### The constraint

serde gives a type no format-neutral way to capture the raw bytes of the next
value, or to write bytes as they are. serde_json does it through a private
marker name its own reader and writer recognise. rmp-serde has nothing like
it: its `rmp_serde::Raw` is for strings that aren't valid UTF-8, not values
(checked in 1.3.1). So under MessagePack, a `Raw` field can only be
re-encoded, unless reflectapi adds its own machinery.

The building blocks exist in rmp-serde, just not reachable from a generic
`Deserialize`/`Serialize` impl: a slice-based deserializer exposes the unread
input (`get_ref().remaining_slice()`), and the serializer exposes its writer
(`get_mut()`). Finding where the next MessagePack value ends is cheap and
allocation-free: every string, binary and container states its length.

### Options

1. **Convert at the edge. Chosen and implemented** (`reflectapi/src/transcode.rs`, our own converter rather than `serde_transcode`, so it can refuse what JSON can't hold). `Raw` keeps JSON text. Use serde's
   `is_human_readable()` (serde_json: true, rmp-serde: false, both checked):
   - JSON: exact passthrough, as now.
   - MessagePack request: stream the value into JSON text (key order and
     every field kept).
   - MessagePack response: stream the stored JSON into MessagePack. Fixes the
     marker bug.

   Correct in both formats, keeps unknown fields, works anywhere serde works
   (not only inside reflectapi's handlers). About 150 lines, or the small
   `serde_transcode` crate. Cost: the MessagePack path pays one conversion,
   about the cost of decoding. Values JSON can't represent (MessagePack
   binary data, NaN) return a clear error.
2. **Store either format, pass bytes through.** `Raw` holds JSON text or
   MessagePack bytes, tagged. reflectapi wraps rmp-serde's reader and writer
   in adapters that recognise `Raw`: reading slices the value's bytes off by
   skipping, writing puts them straight in. Different formats convert as in
   option 1. Zero decoding when formats match, but the adapters are roughly
   800–1,000 lines of boilerplate (pass every serde method through and wrap
   every nested struct, sequence and map, like `serde_path_to_error`), work
   only inside reflectapi's handlers, and `peek`/`decode` need two paths.
   Not worth it: the sources `Raw` serves are JSON (database text, upstream
   calls), rmp-serde decodes fast, and `peek` is fastest on JSON
   ([peek benchmark](#peek-reading-a-few-fields)).
3. **`RawBody<T>` for whole bodies.** See [Proxies](#proxies-rawbody).
4. **JSON only, failing loudly.** The minimum: an error under MessagePack
   instead of the marker object.

### Design (option 1)

- `Raw` is "a `T` held as JSON text"; keep `json() -> &RawValue`.
- `Deserialize`: `is_human_readable()` → `Box<RawValue>` as now; otherwise
  transcode the incoming value into a `serde_json::Serializer` over a
  `Vec<u8>`, then `RawValue::from_string`.
- `Serialize`: `is_human_readable()` → the `RawValue` as now; otherwise
  transcode from a `serde_json::Deserializer` over the text into the given
  serializer.
- Lives under `json`; doesn't need `msgpack` (it works with any serde
  format).
- Tests: JSON and MessagePack, request and response; unknown fields survive;
  key order kept; binary data and NaN give errors; a response no longer
  contains the marker.

Later, optional: `peek` through sonic-rs's unchecked lookup behind a feature.
`Raw`'s text is always valid JSON (`RawValue` checks on creation; database
JSON columns guarantee it), so skipping validation is safe; 19 µs against
273 µs today on the record list. Fall back to serde_json on wasm builds
without `simd128`, where sonic-rs is slower.

## Proxies: RawBody

A proxy forwarding JSON or MessagePack to a MessagePack upstream would, with
`Raw`, convert a MessagePack request twice (to JSON on the way in, back on
the way out). `RawBody<T>` is a type for the whole request or response:

- Holds the body bytes and their format (JSON or MessagePack) as received;
  the schema and clients see `T`.
- `format()`, `bytes()`, `peek::<V>()` and `decode()` (serde_json or
  rmp-serde by format), `to_msgpack()` and `to_json()` (unchanged when the
  format matches, converted otherwise).
- Needs a small builder hook so a handler taking `RawBody` gets the body
  before any decoding ([The builder hook](#the-builder-hook)). A response
  `RawBody` goes out unchanged when the client accepts its format, converted
  otherwise.

Forwarding to a MessagePack upstream:

| Client sends | Conversions |
|---|---|
| MessagePack | none: forwarded as is; `peek` reads only what it needs |
| JSON | one: JSON to MessagePack |

Responses mirror it, given `accept` support.

The two types have separate jobs: `Raw<T>` is a JSON value inside a typed
message; `RawBody<T>` is the whole message in whichever format arrived.

### The builder hook

Today `Handler::new` and `new_stream` (`reflectapi/src/builder/handler.rs`)
and `Builder::route` require `I: Input + DeserializeOwned`, and
`parse_input` always decodes the body with serde, choosing the format from
`content-type`:

```rust
let input_parsed = match content_type {
    ContentType::Json => serde_json::from_slice::<I>(&body),
    #[cfg(feature = "msgpack")]
    ContentType::MessagePack => rmp_serde::from_read::<_, I>(body.as_ref()),
};
```

A handler only ever receives what went through serde, which can't hand a
type the raw bytes of a MessagePack value. The hook replaces that call with a
trait that gets the bytes and the format:

```rust
/// How a route's input is made from the request body. Sealed: implemented for
/// serde types and for `RawBody`.
pub trait FromBody: Sized + sealed::Sealed {
    fn from_body(body: bytes::Bytes, format: ContentType) -> Result<Self, String>;
}

// Every serde type: exactly what parse_input does today.
impl<T: serde::de::DeserializeOwned> FromBody for T {
    fn from_body(body: Bytes, format: ContentType) -> Result<Self, String> {
        match format {
            ContentType::Json => serde_json::from_slice(&body).map_err(|e| e.to_string()),
            #[cfg(feature = "msgpack")]
            ContentType::MessagePack => rmp_serde::from_slice(&body).map_err(|e| e.to_string()),
        }
    }
}

// RawBody: keep the bytes, decode nothing.
impl<T> FromBody for RawBody<T> {
    fn from_body(body: Bytes, format: ContentType) -> Result<Self, String> {
        Ok(RawBody::new(body, format))
    }
}
```

- The bounds become `I: Input + FromBody`, and `parse_input` calls
  `I::from_body(body, content_type)` (keeping today's empty-body rule: an
  empty body reads as `{}`).
- Existing handlers are unchanged: every `DeserializeOwned` type gets
  `FromBody` through the blanket impl, and code generic over
  `DeserializeOwned` still satisfies the bound.
- The two impls don't overlap because `RawBody` is a local type that never
  implements `Deserialize`. The trait is sealed so users can't add impls
  that would.
- It lives in `handler.rs`, so the axum integration and `dispatch` both get
  it.
- `ContentType` (today `#[doc(hidden)]`) becomes public API, or the trait
  takes a public format enum of its own.

Responses mirror it: `handler_wrap` serializes the result with
`serde_json::to_vec` / `rmp_serde::to_vec_named`. An `IntoBody` trait, with a
blanket impl for serde types (serialized as today) and an impl for `RawBody`
(its bytes unchanged when already in the wanted format, converted
otherwise), is applied to the success value; route errors stay serde. This is
also where [honouring `accept`](#honouring-accept) fits: the hook gets the
format the client asked for, not just the request's.

Not covered: request headers (`H`, still built through serde from the header
map) and stream items (SSE events, always JSON).

## Honouring `accept`

Respond in the format `accept` asks for (`application/json` or
`application/msgpack`); without `accept`, in the request's format, as now.
Needed for `RawBody` responses and for clients that send small JSON requests
but want large MessagePack responses.

## Client opt-in

- **Rust** (`reflectapi::rt`): rmp-serde, already a dependency of reflectapi.
  Matters most for a proxy calling its upstream through the generated client.
- **TypeScript**, for Node: `msgpackr` as an optional dependency; off in
  browsers.
- **Python**: the `msgpack` package (C extension).

## Storage note: database JSON

Postgres hands `jsonb` to clients as JSON text, even over the binary protocol
(a version byte, then text), rendering its internal tree on every read. It
has no MessagePack output. For bodies only passed through:

| Column | Writes | Reads | Querying inside | MessagePack response |
|---|---|---|---|---|
| `jsonb` | parsed into a tree | rendered to text every time | fast, indexable | convert in the application |
| `json` | checked as valid JSON, stored as text | the stored text, unchanged: cheapest for passthrough | possible, re-parsed per query | convert in the application |
| `bytea` with MessagePack | stored unchanged | bytes unchanged | not in SQL | passthrough only with option 2's adapters; JSON clients need conversion |

`jsonb` also drops whitespace, reorders keys and collapses duplicates, so
"as it arrived" means "as Postgres renders it". None of this changes the
`Raw` design.

## Alternatives considered

- **CBOR** (RFC 8949): same data model and, measured, the same size and speed
  as MessagePack (cbor-x in Node, minicbor-serde in Rust); stricter
  libraries are much slower (cborg sorts keys: 3–4× slower decoding, 7–19×
  slower encoding; ciborium decodes 2.4× slower than rmp-serde). Its
  advantages are standardisation and tags, not speed.
- **Binary JSON formats**: Postgres `jsonb` and SQLite JSONB are internal
  random-access formats (offset tables, sorted keys): larger and slower to
  write, and serde decoding reads every field in order anyway. Postgres's
  isn't a wire format at all. BSON stores byte lengths so whole documents can
  be skipped, but is larger, has no unsigned 64-bit integers, and stores
  arrays as maps keyed `"0"`, `"1"`, …. UBJSON, Smile and Amazon Ion don't
  beat MessagePack either. MessagePack is the binary JSON.
- **Schema-based formats** (Protobuf, FlatBuffers, Avro, bincode, postcard):
  much smaller and faster, but bincode and postcard can't express untagged,
  internally tagged or flattened types (no `deserialize_any`), and Protobuf
  would mean reflectapi generating its own encoders with different evolution
  rules. Only for a measured hot path.
- **Faster JSON** (sonic-rs, simd-json): a drop-in on the server. sonic-rs
  closes most of the gap to MessagePack on records and text, not on numbers.
- **Compression**: the biggest size win, but costly on the server. gzip 6
  takes about 35× as long as encoding the JSON; zstd 3 matches its size in a
  tenth of the time. See [Compression](#compression-rust).

## Benchmarks

Synthetic payloads shaped like reflectapi responses: snake_case keys,
internally tagged enums, decimals as strings, nulls for absent optionals,
`items` plus `pagination`. Four shapes: 1,000 mixed records (657 KB JSON),
one record (0.8 KB), 5,000 numeric points (8 floats each), 200 text documents
of about 2 KB. Times are medians of 15 batches after a warm-up, on one thread
of a 24-core x86_64 Linux machine; every format's round trip is checked
equal. One machine and synthetic data: treat ratios as indicative.

Reproduce: `bench/formats/node` (`npm install && npm run bench`, then
`npm run dump` for the Rust payloads) and `bench/formats/rust`
(`cargo run --release`, `cargo run --release --bin peek`). For wasm:
`cargo build --release --target wasm32-wasip1 --bins` (add
`RUSTFLAGS="-C target-feature=+simd128"` for SIMD), then
`node run-wasi.mjs target/wasm32-wasip1/release/rsbench.wasm`.

Versions: Node 24.14.1, msgpackr 1.12.1 (native addon on unless stated),
@msgpack/msgpack 3.1.3, cbor-x 1.6.9, cborg 4.5.8; Rust 1.92, serde_json
1.0.154, sonic-rs 0.5.12, simd-json 0.15.1, rmp-serde 1.3.1, ciborium 0.2.2,
minicbor-serde 0.6.2, flate2 1.1.11, brotli 8.0.4, zstd 0.13.5.

### Node

Size in KB (raw / gzip 6 / brotli 5), decode and encode time:

**1,000 records**

| Format | Size | Decode | Encode |
|---|---|---:|---:|
| JSON (`JSON.parse` of a UTF-8 buffer) | 656.6 / 160.3 / 163.5 | 1.15 ms | 0.95 ms |
| MessagePack, msgpackr | 582.3 / 166.3 / 166.7 | 1.06 ms | 1.00 ms |
| MessagePack, msgpackr, no native addon | same | 1.60 ms | 1.11 ms |
| MessagePack, @msgpack/msgpack | 572.5 / 165.9 / 166.4 | 1.70 ms | 1.69 ms |
| msgpackr records extension (non-standard) | 398.2 / 152.8 / 156.5 | 0.36 ms | 0.99 ms |
| CBOR, cbor-x | 584.7 / 166.3 / 166.8 | 1.10 ms | 1.27 ms |
| CBOR, cborg (sorts keys) | 574.9 / 166.1 / 166.6 | 3.27 ms | 6.94 ms |
| cbor-x records extension (non-standard) | 415.1 / 154.0 / 157.5 | 0.40 ms | 1.14 ms |

**5,000 numeric points**

| Format | Size | Decode | Encode |
|---|---|---:|---:|
| JSON | 933.6 / 396.8 / 406.8 | 1.70 ms | 3.02 ms |
| MessagePack, msgpackr | 512.0 / 291.5 / 259.4 | 0.92 ms | 0.73 ms |
| MessagePack, msgpackr, no native addon | same | 0.69 ms | 0.71 ms |
| MessagePack, @msgpack/msgpack | 502.2 / 292.7 / 261.0 | 0.65 ms | 1.07 ms |
| msgpackr records (non-standard) | 429.0 / 288.4 / 258.8 | 0.39 ms | 0.74 ms |
| CBOR, cbor-x | 512.0 / 291.5 / 259.3 | 0.92 ms | 0.80 ms |
| CBOR, cborg | 502.2 / 292.7 / 261.2 | 3.06 ms | 13.82 ms |

**200 text documents**

| Format | Size | Decode | Encode |
|---|---|---:|---:|
| JSON | 434.1 / 81.6 / 95.7 | 0.20 ms | 0.33 ms |
| MessagePack, msgpackr | 432.0 / 82.2 / 96.3 | 94 µs | 83 µs |
| MessagePack, msgpackr, no native addon | same | 0.16 ms | 88 µs |
| MessagePack, @msgpack/msgpack | 431.6 / 82.2 / 96.2 | 0.15 ms | 0.65 ms |
| CBOR, cbor-x | 432.2 / 82.2 / 96.3 | 94 µs | 95 µs |

**One record**: everything is 1–3 µs; JSON is fastest (1.1 µs decode).

Takeaways: in Node, msgpackr beats `JSON.parse` 2–2.4× on numbers and
text, and slightly on mixed records, the latter only with its native addon
(on by default in Node, absent in browsers, where it is 40% slower). The
records extensions are 3× faster on lists but unreadable by rmp-serde.
CBOR via cbor-x equals msgpackr.

### Rust (serde, typed structs)

Decoding into typed structs, as a reflectapi server does (including the
internally tagged enum). Built with `-C target-cpu=native`; without it,
sonic-rs's record decode is 0.69 ms instead of 0.63.

| Payload | serde_json | sonic-rs | simd-json* | rmp-serde | minicbor-serde | ciborium |
|---|---:|---:|---:|---:|---:|---:|
| 1,000 records, decode | 1.01 ms | 0.63 ms | 0.88 ms | 0.71 ms | 0.82 ms | 1.69 ms |
| 1,000 records, encode | 0.34 ms | 0.26 ms | 0.34 ms | 0.21 ms | 0.15 ms | 0.21 ms |
| numeric, decode | 1.15 ms | 0.96 ms | 1.19 ms | 0.31 ms | 0.36 ms | 1.09 ms |
| numeric, encode | 0.57 ms | 0.59 ms | 0.91 ms | 0.16 ms | 85 µs | 0.27 ms |
| text, decode | 81 µs | 41 µs | 75 µs | 38 µs | 42 µs | 91 µs |
| text, encode | 100 µs | 16 µs | 23 µs | 12 µs | 8 µs | 12 µs |
| one record, decode | 0.8 µs | 0.5 µs | 0.8 µs | 0.4 µs | 0.5 µs | 1.7 µs |

\* simd-json needs a mutable buffer; the time includes copying it.

Sizes: MessagePack 572.6 KB for the records (JSON 656.6), 502.2 KB for the
numeric data (JSON 931.5).

### Compression (Rust)

| Input | Method | Size | Compress | Decompress |
|---|---|---:|---:|---:|
| records as JSON (656.6 KB) | gzip 1 (flate2, miniz_oxide) | 229.8 KB | 1.95 ms | 1.13 ms |
| | gzip 6 | 163.9 KB | 11.98 ms | 0.93 ms |
| | brotli 1 | 264.3 KB | 3.30 ms | 2.51 ms |
| | brotli 4 | 175.8 KB | 4.86 ms | 1.22 ms |
| | brotli 5 | 163.5 KB | 10.30 ms | 1.19 ms |
| | zstd 1 | 170.4 KB | 0.96 ms | 0.23 ms |
| | zstd 3 | 165.6 KB | 1.20 ms | 0.23 ms |
| records as MessagePack (572.6 KB) | gzip 6 | 168.7 KB | 11.17 ms | 0.92 ms |
| | zstd 3 | 168.8 KB | 1.13 ms | 0.22 ms |
| numeric as JSON (931.5 KB) | gzip 6 | 402.5 KB | 24.92 ms | 1.85 ms |
| | zstd 1 | 363.9 KB | 1.29 ms | 0.28 ms |

Takeaways: gzip 6 costs about 35× the JSON encode (flate2's default backend;
zlib-ng or zlib-rs would be 2–3× faster). gzip 6 pays off below about
330 Mbit/s; zstd 3 below about 3 Gbit/s. MessagePack compresses no smaller
than JSON. The brotli library's default quality is 11, far slower than any
row here: always set the level.

Guidance: browsers, JSON plus gzip 1–4, brotli 4, or zstd where supported;
service to service, MessagePack, no compression on fast networks, zstd 1–3
where bandwidth costs money.

### `peek`: reading a few fields

| Reading | serde_json view (today's `peek`) | sonic-rs `get_from_slice` | sonic-rs unchecked | rmp-serde view | MessagePack, skipping by lengths |
|---|---:|---:|---:|---:|---:|
| `pagination.next_cursor`, after all 1,000 records (full decode 1.03 ms) | 0.27 ms | 0.21 ms | 19 µs | 0.33 ms | 97 µs |
| `items[500].id` | 0.44 ms* | 0.11 ms | | 0.33 ms* | 48 µs |
| one record, two fields | 0.38 µs | 0.20 µs | | 0.23 µs | 0.10 µs |

\* A serde view can't jump to an index, so it reads every item's `id`.

Takeaways: today's `peek` is already 4× cheaper than a full decode.
MessagePack via a serde view is no faster than JSON; skipping by lengths
(hand-written over `rmp`) is about 3× faster. sonic-rs's unchecked lookup on
already-valid JSON is the fastest.

Why sonic-rs is fast: SIMD (16–32 bytes per instruction) to skip whitespace,
find string ends and escapes, and validate UTF-8; skipping unwanted values by
SIMD bracket and quote matching instead of walking tokens; SIMD escape checks
when writing. The SIMD path is chosen at compile time (`sonic-simd`): SSE2 or
AVX2 on x86_64, NEON on aarch64, wasm SIMD128 only with
`-C target-feature=+simd128`, otherwise scalar emulation.

### wasm (`wasm32-wasip1`, run on V8 through Node's WASI)

wasmtime wasn't available here; V8 is the engine of Cloudflare Workers, which
reflectapi supports. wasmtime enables SIMD128 by default too; expect the same
shape, with different absolute numbers.

| | native | wasm | wasm + `simd128` |
|---|---:|---:|---:|
| serde_json, records decode | 1.01 ms | 1.08 ms | 1.04 ms |
| sonic-rs, records decode | 0.69 ms (no CPU flags) | 3.61 ms | 0.79 ms |
| rmp-serde, records decode | 0.68–0.71 ms | 0.65 ms | 0.65 ms |
| sonic-rs, numeric decode | 0.96 ms | 2.18 ms | 1.27 ms |
| sonic-rs, text decode | 41 µs | 0.95 ms | 52 µs |
| sonic-rs unchecked `peek`, cursor | 19–44 µs | 1.83 ms | 48 µs |
| MessagePack, skipping by lengths, cursor | 91–97 µs | 128 µs | 155 µs |

Takeaways: without `simd128`, sonic-rs is 2–10× slower than serde_json on
wasm; with it, 15–40% faster. serde_json and rmp-serde barely change on
wasm, and rmp-serde stays the fastest full decoder everywhere.

## Further candidates

Types downstream services keep re-implementing, which could follow as an
opt-in conventions module, separately from this plan:

- Pagination: a request with optional `cursor` and `limit`; responses with
  `items` and optional `pagination: {next_cursor}` (and a variant with
  `total_count`), lists wrapped in `items` so pagination can be added later.
- Conflict resolution on writes: an `if` field saying what happens when the
  target exists or is missing (`abort`, `ignore`, `replace`, `update`), with
  per-endpoint enums of the allowed choices and their defaults.
- Schema lints through `Builder::validate`. Some catch codegen-breaking
  shapes and belong in core: unit variants in untagged enums (serialize to
  `null`), two types renamed onto one name (the codegen silently drops one),
  repeated module segments (`a::a::T`, breaks the TypeScript codegen).
  Others are style, configurable: tagged functions, `resource.verb` names
  from a verb list, snake_case variants, no `Enum`/`Struct` suffixes, path
  depth.

## Work plan

1. **This branch**: `Bounded`, `Empty`, `Raw` (JSON), the codegen fix. Then
   `Raw` under MessagePack (option 1) and `Encoded`.
2. **`accept`**: respond in the requested format.
3. **`RawBody<T>`** with the builder hook.
4. **Client opt-in**: Rust runtime first, then TypeScript (Node, msgpackr),
   then Python.
5. **`Bounded` in every client**: Python, TypeScript and OpenAPI
   ([Checking `Bounded` in every client](#checking-bounded-in-every-client)).
6. **Optional**: sonic-rs `peek` behind a feature with a wasm fallback.

Progress is tracked in [Remaining](#remaining).

## Checking `Bounded` in every client

Today only the server and the Rust client check the bounds. Every client
should check them before sending, so a bad request fails locally with a clear
error instead of a 400.

- **Rust**: done; the client uses `reflectapi::Bounded` itself.
- **Python**: generate `Annotated[T, Field(min_length=MIN, max_length=MAX)]`.
  Pydantic checks when a request model is built and when a response is
  parsed; it covers strings, lists, sets and dicts, nested too. `len(str)`
  counts code points, matching Rust's character count. Work: a Python codegen
  mapping for `reflectapi::Bounded` that writes `MIN` and `MAX` as literals.
- **TypeScript**: the type system can't express a length range, so it is a
  runtime check. Options:
  1. **Reuse the descriptor decoder. Recommended.** The client already ships
     `__decode` (`codegen/lib_decode.ts`), which walks a value against
     generated descriptors; today it runs on responses of endpoints whose
     output reaches an open enum or a `#[serde(other)]` variant, and its
     `integer` descriptor already enforces `min`/`max`. Add
     `{k: "len", min, max, v}`, and a request-side hook in `__request` that
     checks the input before sending, generated only for endpoints whose
     input reaches a `Bounded`. Other endpoints get no extra code.
  2. **A branded type with a checking constructor**, like the
     `#[reflectapi(distinct)]` brand: errors at compile time, but every place
     that builds such a field must call the constructor. See
     [Branded `Bounded` in TypeScript](#branded-bounded-in-typescript).
     `Bounded` is unreleased, so choosing this now breaks nobody; only code
     migrating from a downstream copy that passes plain values needs
     `bounded()` calls.
  3. **A separate validator export** callers run themselves: easy to forget.

  Strings must be measured in code points (`[...s].length` or a loop), not
  `string.length`, which counts UTF-16 units (🦀 is 2).
- **OpenAPI**: `minLength`/`maxLength` for strings, `minItems`/`maxItems` for
  arrays, `minProperties`/`maxProperties` for maps.

Check requests only by default: the server already enforces bounds on what it
reads, and checking every response would cost every call.

### Branded `Bounded` in TypeScript

Chosen and implemented. The descriptor check (option 1) and the
per-request `check()` remain possible additions.

**What branding means.** TypeScript compares types by structure, so today's
`type Bounded<T, MIN, MAX> = T` is exactly `T`: any `string[]` fits a
`Bounded<string[], 0, 50>` field. A brand intersects the type with a property
that exists only for the compiler:

```ts
declare const __bounded: unique symbol;
export type Bounded<T, MIN extends number, MAX extends number> =
  T & { readonly [__bounded]: readonly [MIN, MAX] };
```

At runtime the value is still the plain `T` (`declare const` emits nothing,
the wire is unchanged). At compile time a plain `T` lacks the property, so it
isn't assignable; the only way to get a `Bounded` is a cast, which
`bounded()` makes after checking. `MIN` and `MAX` are part of the brand, so
different bounds are different types. `#[reflectapi(distinct)]` uses the same
technique (`__Distinct`), with an unchecked constructor.

**Per-value API.** For a field `tags: Bounded<Array<Bounded<string, 1, 32>>, 0, 50>`:

```ts
import { bounded, BoundsError } from "./generated";

const tag = bounded("red", 1, 32);                     // Bounded<string, 1, 32>, or throws BoundsError
await client.pets.tag({ tags: bounded([tag], 0, 50) }, {});

const name = bounded.try(input.value, 1, 32);          // Result<Bounded<string, 1, 32>, BoundsError>
if (name.is_err()) show(name.err()!.message);          // "length 0 out of bounds (min 1, max 32)"

client.pets.tag({ tags: ["red"] }, {});                // compile error: string[] isn't a Bounded
client.pets.tag({ tags: bounded([tag], 0, 100) }, {}); // compile error: bounds differ from the schema's
```

Runtime, in `codegen/lib.ts`:

```ts
export class BoundsError extends Error {
  constructor(readonly length: number, readonly min: number, readonly max: number) {
    super(`length ${length} out of bounds (min ${min}, max ${max})`);
  }
}

function __length(value: unknown): number {
  if (typeof value === "string") { let n = 0; for (const _ of value) n++; return n; } // code points
  if (Array.isArray(value)) return value.length;                                      // Vec, sets
  if (value !== null && typeof value === "object") return Object.keys(value).length;  // maps
  throw new TypeError("bounded() takes a string, an array or an object");
}

export const bounded = Object.assign(
  <T, MIN extends number, MAX extends number>(value: T, min: MIN, max: MAX): Bounded<T, MIN, MAX> => {
    const length = __length(value);
    if (length < min || length > max) throw new BoundsError(length, min, max);
    return value as Bounded<T, MIN, MAX>;
  },
  { try: /* the same check, returning Result<Bounded<T, MIN, MAX>, BoundsError> */ },
);
```

- The caller passes the bounds, because types are erased at runtime.
  TypeScript infers them as literal types, so the compiler rejects bounds
  that differ from the schema's; when the server changes a bound, call sites
  stop compiling until updated.
- Responses: decoded values are typed as branded but stay plain at runtime,
  with no check (the server guarantees them). A value from a response can be
  sent back without checking again.
- `__Wire<T>`, which strips distinct brands to get the plain JSON shape, must
  strip this brand too.
- Cost: nesting is verbose,
  `bounded(names.map((n) => bounded(n, 1, 32)), 0, 50)`.

**Per-request alternative.** A generated check per input type containing a
`Bounded`, `TagPet.check({ tags: ["red"] })`, which validates every nested
`Bounded` through the descriptor decoder and returns a branded request the
endpoint requires. One call per request, bounds from the schema, nothing
written at call sites; but values can't be checked once and reused, and every
such input type gets a generated function.

**Python and Rust need no new API**: constructing the value checks. Python:
`TagPet(tags=[...])` raises `pydantic.ValidationError`;
`TagPet.model_construct(...)` skips it. Rust: `Bounded::new(v)?`,
`Bounded::try_from(v)?`, or `Bounded::from_array([...])` (checked at compile
time).

## Remaining

Tracked here until done. Sections above have the details.

### This branch

- [x] `Bounded`, `BoundedMax`, `Length`, `BoundsError` in core
- [x] `Empty` derives `Copy` and `Hash`
- [x] `Raw<T>` under `json`, JSON passthrough
- [x] Codegen: template parameters filled in one pass
- [x] `Raw` under MessagePack: convert at the edge
  ([design](#design-option-1)), with tests in both formats
- [x] `Encoded<T>`: under `json`, JSON then base64url without padding,
  documented as encoded, not secret

### MessagePack

- [ ] Respond in the format `accept` asks for
- [ ] `RawBody<T>` and the builder hook: sealed `FromBody` and `IntoBody` in place of the hard-coded serde calls ([design](#the-builder-hook))
- [ ] Rust client: opt-in MessagePack in `reflectapi::rt`
- [ ] TypeScript client: opt-in MessagePack for Node via `msgpackr`
- [ ] Python client: opt-in MessagePack via `msgpack`
- [ ] Optional: `Raw::peek` through sonic-rs's unchecked lookup behind a
  feature, falling back to serde_json on wasm without `simd128`

### `Bounded` in clients

- [x] Python: Pydantic length constraints
- [x] TypeScript: the branded `Bounded` with `bounded()` and `bounded.try()`
  ([design](#branded-bounded-in-typescript))
- [x] OpenAPI: length keywords, through `allOf`
- [x] One recognizer for length ranges (`codegen/schema/length.rs`,
  `length_bounds`): bounds parsed and checked once, and a `LengthKind`
  (characters, items, entries) that decides the keywords. OpenAPI and Python
  render from it. Schema names stay string literals, as for every other
  built-in type (`"reflectapi::Option"`, `"std::vec::Vec"`). A user type implementing `Length` gets no
  client checks (Pydantic would reject a length constraint on it)
- [x] Fixed-size arrays through the same recognizer: Python
  `Field(min_length=N, max_length=N)`, OpenAPI `minItems`/`maxItems`, and
  TypeScript `fixedSizeArray(items, N)` for arrays built at runtime

### Types and lints from downstream

Proven in a downstream extension crate; ported in their own PRs, after
agreeing the conventions module's scope.

| Piece | Plan |
|---|---|
| Pagination: page request (`cursor`, `limit`), forward-paginated list, non-paginated list, list with `total_count` | Conventions module; add `previous_cursor` |
| Authorization header inputs (required and optional), `Identity`, `IdentityData` | Conventions module |
| Conflict resolution: `if` field enums (`IfExists*`, `IfNotExists*`), `InsertOrElse` and the other wrappers, match conditions | Conventions module. Two parallel versions exist (with and without match conditions): merge them first, and replace the `subenum`/`duplicate` macros with `macro_rules!` |
| Lints that prevent codegen bugs: unit variants in untagged enums, two types renamed onto one name, repeated module segments | Core, as warnings first, then errors |
| Style lints: tagged functions, `resource.verb` names from a verb list, snake_case variants, no `Enum`/`Struct` suffixes, path depth, error variant naming, internal endpoints tagged, description indentation | Conventions module, with the verb list and exceptions as settings |
| An internal-error type that returns only a trace ID to clients and logs the rest | Keep downstream: it depends on OpenTelemetry, whose frequent breaking releases would force breaking releases of reflectapi |

Downstream migration, once ported: the downstream crate re-exports
reflectapi's types; the schema name of its bounded type changes to
`reflectapi::Bounded`, so generated clients change and its Rust codegen
`shared_modules` entry goes away.
