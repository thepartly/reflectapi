# Integrating Other Servers

Any HTTP server can serve a reflectapi API through
`reflectapi::dispatch::Dispatcher`. It needs only the `builder` feature, and it
is what the [axum](./axum.md) and [Worker](./workers.md) integrations are built
on. This page is the contract an integration implements.

## Build modes

A server needs the routes; the schema is for generating clients and
documentation. `Builder<S, M>` takes a build mode:

| Mode | `build()` returns | Use |
|---|---|---|
| `WithSchema` (default) | `(Schema, Vec<Router<S>>)` | Build step: write the schema, generate clients, check them in CI. A server that also serves its schema |
| `DispatchOnly` | `Vec<Router<S>>` | Serving. Only request-headers types are reflected, for the header names, so startup does almost no schema work and the reflection code of every other type is not linked |

Write the definition once, generic over the mode, starting from
`Builder::default()` (`Builder::new()` is always `WithSchema`):

```rust,ignore
pub fn api<M: reflectapi::BuildMode>() -> reflectapi::Builder<AppState, M> {
    reflectapi::Builder::default()
        .name("Pets")
        .route(pets_list, |b| b.name("pets.list").readonly(true))
        .stream_route(pets_events, |b| b.name("pets.events"))
}

let (schema, _) = api::<reflectapi::WithSchema>().build()?; // build step
let routers = api::<reflectapi::DispatchOnly>().build()?;   // server
```

Schema validation (custom validators, response headers, redundant renames)
runs only in the `WithSchema` build, so run that build in CI. Errors that
affect serving, such as an invalid request header name, fail both builds.

## Dispatch a request

```rust,ignore
use reflectapi::dispatch::{sse_event, Dispatcher, Reply};

let dispatcher = Dispatcher::new(routers);

// per request
match dispatcher.dispatch(state.clone(), &method, path, &headers, body).await {
    None => { /* no such route */ }
    Some(Reply::Complete { status, headers, body }) => { /* send as is */ }
    Some(Reply::Stream(items)) => { /* send as Server-Sent Events */ }
}
```

`dispatch` takes the request method, its path without the query string, all
its headers, and the complete body.

## The contract

**Routing.** Every route is at `POST {path}/{name}`; a readonly route also
accepts `GET`. This is what the axum adapter serves, and `dispatch` answers
the same way:

- `None` when no route has the path. Respond `404`, or pass the request on to
  the server's other routes.
- A `405 Method Not Allowed` reply when the route exists but not for this
  method, with an `allow` header: `POST`, or `GET,POST` for a readonly
  route. This includes `HEAD`: axum answers it for readonly routes by running
  the `GET` handler and dropping the body, and says `GET,HEAD,POST`;
  `dispatch` does not.
- For `HEAD`, the `GET` reply's status and headers, without a body.

**Requests.**

- Pass all request headers; the dispatcher forwards to each handler only the
  headers it declares, plus `content-type` and `traceparent`.
- Pass the complete body. Enforce a body size limit before dispatching.
- The state is cloned into every call: make it cheap to clone (`Arc`, handles).

**Complete replies.** Send `status`, `headers` and `body` unchanged. The
reply has no `content-length`: the length is transport metadata, which the
server sets when it writes the body (axum does, and so do Worker runtimes).

**Stream replies.**

- Respond `200` with `content-type: text/event-stream` and
  `cache-control: no-cache`.
- Frame each `Ok` item with `sse_event`, and send it as soon as it arrives:
  do not buffer the response. An item is an `Event<String>`: the serialized
  value, and the event id if the route gave the item one by wrapping it in
  `reflectapi::Event`; `sse_event` writes the id as the standard SSE `id:`
  field.
- An `Err` item is a value that failed to serialize: log it and end the
  response.
- reflectapi sends no heartbeats and no end-of-stream marker; the stream ends
  when the response ends. Add SSE comment lines (`:\n\n`) as heartbeats if a
  proxy or runtime closes quiet connections.
- An error that the handler returns before the stream opens arrives as a
  complete reply, not as a stream.

**`Send`.** Handlers, their futures and their streams must be `Send`. On a
single-threaded runtime, wrap values that are not, as the
[Worker integration](./workers.md#define-the-api) does.

## Partitioned servers

A server can be partitioned by a request's partition key, the fields marked
`#[reflectapi(partition_key = N)]` (see
[Attributes](../getting-started/attributes.md#partition-key)): each partition
serves the requests whose key is its own, and a router in front sends each
request to its partition. Two calls of the dispatcher serve this, in either
build mode:

```rust,ignore
// The router: find the request's partition.
match dispatcher.partition_key(&method, path, &headers, body.clone()) {
    Ok(Some(parts)) => { /* send the request to the partition named by parts */ }
    Ok(None) => { /* no key: serve it anywhere, with `dispatch` */ }
    Err(reply) => { /* the input does not parse: send the reply */ }
}

// The partition: serve only its own requests.
dispatcher.dispatch_in(&["acme", "o-42"], state, &method, path, &headers, body).await
```

- `partition_key` parses the body and the headers as the handler would, and
  gives the key's parts in position order, without calling the handler. It
  is `Ok(None)` for a route without a key, and wherever `dispatch` would not
  call a handler: no route has the path, or the route does not accept the
  method. It is `Err` with the reply the handler would give for input that
  does not parse.
- `dispatch_in` is `dispatch`, except that after parsing the request and
  before calling the handler it compares the request's key with the
  partition's, and answers `400` without calling the handler when they
  differ. Routes without a key are dispatched as by `dispatch`. The request
  is parsed once.

The parts are strings, compared as they are. How they become one name, such
as a host or a shard, is the server's: if it joins them, make the join
unambiguous, so that two different keys never name one partition.

A client that knows the key from the schema may pick the partition itself;
`dispatch_in` is what keeps a partition's handlers to its own requests when
a client picks the wrong one.

## Test the integration

The responses must be identical to the axum adapter's. The reflectapi demo
checks this by replaying recorded requests through both and comparing status,
headers and body (`reflectapi-demo/tests/run`), and does the same for unknown
paths and unaccepted methods (`reflectapi-demo/tests/dispatch.rs`); do the
same with your adapter. The expected differences are `content-length`, which
axum sets itself, and `HEAD` on readonly routes, described above.
