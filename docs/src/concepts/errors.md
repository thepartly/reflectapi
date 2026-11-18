# Errors

A route's errors are part of its type, like its input and output. A handler
returns `Result<O, E>`; `E` is reflected into the schema, and every generated
client gives the caller `E` as a typed value, kept apart from network and
server failures.

## A route's error

The error type derives `Serialize` and `Output`, like any type the server
returns, and implements [`StatusCode`](https://docs.rs/reflectapi/latest/reflectapi/trait.StatusCode.html),
which gives the HTTP status of its response. For an enum, derive it and give
each variant its status:

```rust
use reflectapi::{Builder, Input, Output, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Input)]
struct PetsCreateRequest {
    name: String,
}

#[derive(Serialize, Output)]
struct Pet {
    name: String,
}

#[derive(Serialize, Output, StatusCode)]
enum PetsCreateError {
    /// A pet of that name exists.
    #[reflectapi(status = 409)]
    Conflict { name: String },
    /// The name is empty.
    #[reflectapi(status = 422)]
    InvalidName,
    /// A failure the client can't act on.
    #[reflectapi(output_skip, status = 500)]
    InternalError,
}

async fn pets_create(
    _state: (),
    request: PetsCreateRequest,
    _headers: reflectapi::Empty,
) -> Result<Pet, PetsCreateError> {
    if request.name.is_empty() {
        return Err(PetsCreateError::InvalidName);
    }
    if request.name == "Rex" {
        return Err(PetsCreateError::Conflict { name: request.name });
    }
    Ok(Pet { name: request.name })
}

# fn main() -> Result<(), reflectapi::BuildErrors> {
let (schema, _routers) = Builder::new()
    .route(pets_create, |b| b.name("pets.create"))
    .build()?;

// The route's error type is in the schema, without `InternalError`.
let function = schema.functions().next().unwrap();
let error_type = function.error_type.as_ref().unwrap().name();
let error = schema.output_types().get_type(error_type).unwrap();
let variants: Vec<_> = error.as_enum().unwrap().variants().map(|v| v.name()).collect();
assert_eq!(variants, ["Conflict", "InvalidName"]);
# Ok(())
# }
```

- The response body is the error serialized by serde, as a successful
  output is: `{"Conflict":{"name":"Rex"}}`, or `"InvalidName"`.
- `#[reflectapi(transparent)]` on a variant with one field answers with that
  field's status and headers, for an error type shared by several routes.
- `StatusCode::response_headers` adds headers to the error's response, such
  as `retry-after`.
- A struct error, or any type the derive doesn't fit, implements
  `StatusCode` by hand.

The derive refuses, as it compiles, a status that a client would not read as
the route's error: `1xx` to `3xx`; the `4xx` that reflectapi or the server
send themselves, such as `400`; and every `5xx`, except `500` on a variant
named `InternalError` that is `output_skip`. A client takes any `5xx` as the
server's failure, so `InternalError` is left out of the schema: the server can
still return it, and clients see it as a server failure, not a typed error.
[A route's error](../getting-started/attributes.md#a-routes-error) has the
full rules and an example of `transparent` with `response_headers`. A
hand-written `impl StatusCode` isn't checked; if it gives `200`, reflectapi
answers `500` instead.

## Routes that can't fail

A handler that returns its output directly, not a `Result`, has no error
type. So does one that returns `Result<O, reflectapi::Infallible>`. Neither
has an `error_type` in the schema:

```rust
use reflectapi::{Builder, Empty, Infallible};

async fn health(_: (), _: Empty, _: Empty) -> Empty {
    Empty {}
}

async fn version(_: (), _: Empty, _: Empty) -> Result<String, Infallible> {
    Ok("1.0.0".into())
}

# fn main() -> Result<(), reflectapi::BuildErrors> {
let (schema, _) = Builder::new()
    .route(health, |b| b.name("health"))
    .route(version, |b| b.name("version"))
    .build()?;
assert!(schema.functions().all(|f| f.error_type.is_none()));
# Ok(())
# }
```

Use [`reflectapi::Infallible`](https://docs.rs/reflectapi/latest/reflectapi/enum.Infallible.html),
not `std::convert::Infallible`, which doesn't implement `StatusCode`. The
clients of such a route still report network and server failures, the same way
as for other routes.

## Errors reflectapi returns

Some responses come from reflectapi, not the handler: `400` for a body or
headers that don't deserialize into the route's types, `405` for a method the
route doesn't take, `415` for a content type it doesn't speak, and `500` for
an output or error that fails to serialize. Their body is a plain-text
message, not the route's error type, so clients report them as failures, not
typed errors. [Statuses reflectapi sends](./wire.md#statuses-reflectapi-sends)
lists them with their bodies.

## In the generated clients

Each client keeps the route's typed error apart from other failures: no
response, a `5xx`, or a body that isn't the expected type. The examples use
the error above.

### TypeScript

A call returns a `CallResult`, which doesn't throw. On failure it holds an
`Err`: `err()` is the route's error, or `undefined` when the failure is
something else, which `other_err()` then holds.

```typescript
import { client, match } from "./generated";

const c = client("http://localhost:3000");
const result = await c.pets.create({ name: "Rex" }, {});
if (result.is_ok()) {
  console.log(result.unwrap_ok().name);
} else {
  const err = result.unwrap_err();
  const routeError = err.err();
  if (routeError === undefined) {
    // no response, a 5xx, or a body that isn't JSON
    console.error(err.status_code(), err.other_err());
  } else {
    console.log(
      match(routeError, {
        Conflict: ({ name }) => `${name} exists`,
        InvalidName: () => "give the pet a name",
      }),
    );
  }
}
```

`match` takes one handler per variant of an externally tagged enum (serde's
default). For an internally tagged error, `#[serde(tag = "kind")]`, switch on
`routeError.kind` instead.

Any `4xx` with a JSON body is taken as the route's error, and its shape isn't
checked (except for open enums), so a JSON error from a proxy in front of the
server arrives in `err()` as well; `status_code()` tells which response it
was. See [Client Libraries](../clients/index.md#typescript) for the rest of
`Result` and `Err`, and for reading them in a query library's retry callback.

### Rust

A generated method returns `Result<O, reflectapi::rt::Error<E, C::Error>>`,
where `E` is the generated error type and `C::Error` is the transport's error,
`reqwest::Error` for the built-in transport:

```rust
use reflectapi::rt::Error;
# mod types {
#     // As generated from the server's `PetsCreateError`.
#     #[derive(Debug, serde::Deserialize)]
#     pub enum PetsCreateError {
#         Conflict { name: String },
#         InvalidName,
#     }
# }
use types::PetsCreateError;

fn describe(error: Error<PetsCreateError, reqwest::Error>) -> String {
    match error {
        Error::Application(PetsCreateError::Conflict { name }) => format!("{name} exists"),
        Error::Application(PetsCreateError::InvalidName) => "give the pet a name".into(),
        Error::Network(error) => format!("no response: {error}"),
        Error::Protocol { info, stage } => format!("unexpected response: {info} ({stage})"),
        Error::Server(status, _body) => format!("server failed with {status}"),
    }
}

# fn main() {
assert_eq!(
    describe(Error::Application(PetsCreateError::InvalidName)),
    "give the pet a name",
);
# }
```

- `Application`: a non-`5xx` response whose body deserializes into `E`.
- `Network`: the transport failed; there is no response.
- `Protocol`: the request couldn't be serialized, or a response couldn't be
  decoded, including a `4xx` whose body isn't `E`, such as reflectapi's `400`.
- `Server`: a `5xx`, with its status and body.

### Python

A call returns an `ApiResponse`, whose `value` is the output. Failures raise
exceptions from `reflectapi_runtime`, all subclasses of `ApiError`:

```python
from reflectapi_runtime import ApplicationError, NetworkError, ValidationError

from api_client.my_api import PetsCreateErrorConflictVariant, PetsCreateRequest

try:
    response = await client.pets.create(PetsCreateRequest(name="Rex"))
    print(response.value.name)
except ApplicationError as e:
    error = e.typed_error  # a PetsCreateError, or None
    if error is None:
        print("request failed:", e.status_code, e.error_data)
    elif isinstance(error.root, PetsCreateErrorConflictVariant):
        print(f"{error.root.name} exists")
    elif error.root == "InvalidName":
        print("give the pet a name")
    else:
        print("unexpected error:", error.root)
except NetworkError as e:  # TimeoutError is a NetworkError
    print("no response:", e)
except ValidationError as e:
    print("unexpected response:", e)
```

- `ApplicationError`: any response with status `400` or above, `5xx`
  included. `typed_error` is the body validated into the route's error model,
  or `None` when it doesn't validate; `error_data` is the parsed JSON body, or
  `None` when it isn't JSON.
- `NetworkError` and its subclass `TimeoutError`: no response.
- `ValidationError`: a successful response whose body doesn't match the
  output model.

The error model is a Pydantic `RootModel`: its `root` is the variant, a
string for a unit variant or a generated model for one with fields.

## Adding a variant

Adding a variant to a route's error changes its generated type. In
TypeScript and Rust, code that handles every variant stops compiling when the
client is regenerated, and points at each place that needs the new case. Say
the server adds `#[reflectapi(status = 404)] OwnerNotFound` to
`PetsCreateError`. The TypeScript `match` above fails to type-check:

```text
error TS2345: Argument of type '{ Conflict: ...; InvalidName: ...; }' is not
assignable to parameter of type ...
  Property 'OwnerNotFound' is missing in type '{ Conflict: ...; InvalidName: ...; }'
```

and the Rust `match` fails with `error[E0004]: non-exhaustive patterns`,
naming the `Error::Application(PetsCreateError::OwnerNotFound)` case. Python
has no such check: keep a branch for variants you don't handle.

Clients built before the change don't know the new variant. When the server
returns it, a Rust client reports a `Protocol` error, a Python client an
`ApplicationError` with `typed_error` set to `None`, and a TypeScript `match`
throws. To let clients that are already deployed receive variants added
later, mark the error enum
[`#[reflectapi(open)]`](../getting-started/attributes.md#enum-evolution).
[Evolving an API](./evolving.md) covers which changes are safe for deployed
clients.
