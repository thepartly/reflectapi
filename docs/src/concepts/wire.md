# On the Wire

This page shows the HTTP requests and responses of a reflectapi API: enough to
call it with curl, write a client by hand, configure a proxy in front of it,
or debug what a generated client sends. The examples are captured from the
demo server in `reflectapi-demo`, with the `date` and `content-length`
headers left out.

The axum adapter and [`reflectapi::dispatch::Dispatcher`](https://docs.rs/reflectapi/latest/reflectapi/dispatch/struct.Dispatcher.html) answer the same way.
To serve the API from another HTTP server, see
[the contract](../serving/other-servers.md#the-contract) an integration
implements.

## Requests

### URL

A route is at `{path}/{name}`. The name is the route's full name, dots
included. The path is empty unless one is set: with [`Builder::path`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.path) for every
route of a builder, which [`Builder::nest`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.nest) prepends to the routes of the
builders nested in it, or with `RouteBuilder::path` for one route. So
`pets.create` is at `/pets.create`, and under `Builder::path("/api")` at
`/api/pets.create`.

A query string is ignored: the input comes from the body only.

### Method

Every route accepts `POST`. A route marked `readonly(true)` accepts `GET` as
well, and with axum `HEAD`, which returns the `GET` response's status and
headers without a body. The generated clients always use `POST`.

Another method gets `405 Method Not Allowed` with an empty body and an `allow`
header. A path with no route gets `404` from axum.

```text
$ curl -i -X GET localhost:3000/pets.create
HTTP/1.1 405 Method Not Allowed
allow: POST

$ curl -i -X PUT localhost:3000/pets.list
HTTP/1.1 405 Method Not Allowed
allow: GET,HEAD,POST
```

`Dispatcher` doesn't answer `HEAD`: it gets `405`, with `allow: GET,POST` for
a readonly route. See [the contract](../serving/other-servers.md#the-contract).

### Body

The body is the route's input, serialized by serde. An empty body is read as
an empty object, `{}`, so a `GET` without a body works for a route whose input
is [`reflectapi::Empty`](https://docs.rs/reflectapi/latest/reflectapi/struct.Empty.html) or a struct whose fields all have defaults. A `GET` with
a body is read like a `POST`.

### Content type

The `content-type` header picks the format of the request body, and the
response uses the same one:

| `content-type` | Format |
|---|---|
| `application/json`, or no `content-type` | JSON |
| `application/msgpack` | [MessagePack](https://msgpack.org), with the `msgpack` feature. Structs are maps keyed by field name. |
| anything else | `415 Unsupported Media Type` |

A `charset` parameter other than `utf-8` is also refused with `415`. `accept`
is ignored. Stream routes accept JSON only, and answer MessagePack with `415`.
The generated clients send JSON.

`curl -d` sends `content-type: application/x-www-form-urlencoded` unless told
otherwise, so pass the JSON content type:

```text
$ curl -i localhost:3000/pets.create -H 'authorization: password' \
    -d '{"name":"Bobby","kind":{"type":"dog","breed":"Labrador"}}'
HTTP/1.1 415 Unsupported Media Type

unsupported content-type: application/x-www-form-urlencoded
```

### Headers

A route reads the request headers named by the fields of its headers type,
by their serde names
([Request Headers](../clients/index.md#request-headers)). Name them in
lowercase: HTTP header names are case-insensitive, and reflectapi matches the
lowercase form.

- A field that is not an `Option` is a required header. Without it, the
  request is refused with `400` before the handler runs.
- Each value is passed to the field's type as a string, so a field must be a
  string on the wire: `String`, a unit-variant enum, or a type such as
  `uuid::Uuid` or `chrono::DateTime`. A number or `bool` field fails with
  `400` whenever its header is sent.
- If a header appears more than once, the first value is read.
- The handler is given only the headers it declares. Others, such as
  cookies or a proxy's headers, are dropped before it runs.

## Responses

### Success

A successful call is `200 OK`, with the output serialized in the request's
format. `reflectapi::Empty` is `{}`, and an `Option` output that is `None` is
`null`.

```text
$ curl -i localhost:3000/pets.create -H 'content-type: application/json' \
    -H 'authorization: password' \
    -d '{"name":"Bobby","kind":{"type":"dog","breed":"Labrador"}}'
HTTP/1.1 200 OK
content-type: application/json

{}

$ curl -i localhost:3000/pets.list -H 'authorization: password'
HTTP/1.1 200 OK
content-type: application/json

{"items":[{"name":"Bobby","kind":{"type":"dog","breed":"Labrador"},"updated_at":"2024-01-01T00:00:00Z"}],"cursor":"255"}
```

The second request is a `GET`: `pets.list` is readonly, and its input's
fields are all optional.

### The route's error

When the handler returns `Err`, the response has the status the error's
[`ResponseError`](https://docs.rs/reflectapi/latest/reflectapi/trait.ResponseError.html)
implementation gives, and the error serialized in the request's format, as
serde writes it:

```text
$ curl -i localhost:3000/pets.create -H 'content-type: application/json' \
    -H 'authorization: password' \
    -d '{"name":"Bobby","kind":{"type":"dog","breed":"Labrador"}}'
HTTP/1.1 409 Conflict
content-type: application/json

"Conflict"

$ curl -i localhost:3000/pets.create -H 'content-type: application/json' \
    -H 'authorization: password' -d '{"name":"","kind":{"type":"bird"}}'
HTTP/1.1 422 Unprocessable Entity
content-type: application/json

{"InvalidIdentity":{"message":"Name is required"}}
```

The error can add response headers, such as `retry-after`, with
`ResponseError::headers`. They cannot replace `content-type` or
`traceparent`. An error that says `200` is sent as `500`, since a client would
otherwise read it as the output.

The status tells a client which type the body has: a `2xx` body is the
output, and an error status means the body may be the route's error. It may
also come from reflectapi itself or from a proxy, so clients check that it
decodes. The TypeScript and Rust clients treat any `5xx` as a server failure
rather than the route's error; the Python client tries to decode the route's
error for every status from `400` up, `5xx` included. [Errors](./errors.md) covers choosing statuses
and how each client exposes them.

### Statuses reflectapi sends

Some responses come from reflectapi itself, before or after the handler.
Their body is a plain-text message for people, not the route's error type:

| Status | When | Body |
|---|---|---|
| `400 Bad Request` | The body or the headers could not be deserialized into the route's types | `Failed to parse request body: ...` or `Failed to parse request headers: ...`, with no `content-type` |
| `404 Not Found` | No route has the path (axum) | empty |
| `405 Method Not Allowed` | The route exists but does not accept the method | empty, with an `allow` header |
| `415 Unsupported Media Type` | The content type or charset isn't supported, or a stream route got MessagePack | the reason, with no `content-type`, or `text/plain` for a stream route |
| `500 Internal Server Error` | The output or the error failed to serialize | `text/plain` |

```text
$ curl -i localhost:3000/pets.create -H 'content-type: application/json' \
    -H 'authorization: password' -d '{"name":"x"}'
HTTP/1.1 400 Bad Request

Failed to parse request body: missing field `kind` at line 1 column 12, received: b"{\"name\":\"x\"}"

$ curl -i localhost:3000/pets.list -H 'content-type: application/json' -d '{}'
HTTP/1.1 400 Bad Request

Failed to parse request headers: missing field `authorization`
```

The `400` message includes the request body as received.

These are why a route's error can't use `400`, `405` or `415` when it derives
`ResponseError`: a client couldn't tell the route's answer from reflectapi's (see
[A route's error](../getting-started/attributes.md#a-routes-error)). Servers
and proxies in front of the API add their own, such as `413` for a body that
is too large or `502` from a gateway.

### Response headers

reflectapi sets:

- `content-type`, the request's format, on a success or the route's error;
- `traceparent`, copied from the request when it has one, on a success or
  the route's error (not on a stream that opened);
- the headers of the route's error, if any.

Headers declared with `Builder::response_headers` are a description for
clients, not something reflectapi sends: they come from the error's
`response_headers`, from middleware, or from infrastructure in front of the
server ([Response Headers](../clients/index.md#response-headers)).
`content-length` and other transport headers are the HTTP server's.

## Streams

A stream route is called like any other: `POST` (or `GET` if readonly), a JSON
body and its headers. If the handler returns an error, the response is that
error, as for any route. Otherwise the response is `200 OK` with
`content-type: text/event-stream` and `cache-control: no-cache`, and its body
is a stream of [Server-Sent Events](https://html.spec.whatwg.org/multipage/server-sent-events.html):

```text
$ curl -i -N localhost:3000/pets.cdc-events -H 'authorization: password'
HTTP/1.1 200 OK
content-type: text/event-stream
cache-control: no-cache
transfer-encoding: chunked

data: {"name":"Max","kind":{"type":"cat","lives":9},"updated_at":"2024-01-01T00:00:00Z"}

data: {"name":"Max","kind":{"type":"cat","lives":9},"age":3,"updated_at":"2024-01-01T00:00:00Z"}

```

- Each item is one event: a `data:` line with the item as JSON, then a blank
  line. There is no `event:` or `retry:` field.
- An item the handler wraps in [`reflectapi::Event`](https://docs.rs/reflectapi/latest/reflectapi/struct.Event.html) has an `id:` line before
  its data. A route can declare the `last-event-id` request header to resume
  after the last id a client saw
  ([Streaming Endpoints](../clients/index.md#streaming-endpoints)).
- With keep-alive on, a stream that has sent nothing for the interval gets a
  comment, a line with only `:`, then a blank line
  ([Keep idle streams open](../serving/axum.md#keep-idle-streams-open)).
- There is no end-of-stream event: the stream ends when the response ends. An
  item that fails to serialize ends it too.

`reflectapi::dispatch::sse_event` writes one event:

```rust
use reflectapi::{dispatch::sse_event, Event};

let item = r#"{"name":"Max"}"#.to_string();
assert_eq!(sse_event(&Event { id: None, data: item.clone() }), "data: {\"name\":\"Max\"}\n\n");
assert_eq!(sse_event(&Event::new(7, item)), "id: 7\ndata: {\"name\":\"Max\"}\n\n");
```

A stream can't be served as MessagePack, and the schema describes only the
item type: the event ids and the comments are not part of it.

## Check it in-process

`Dispatcher` answers exactly as the axum adapter does, so a test can check a
route's wire behaviour without a server:

```rust
use http::{HeaderMap, Method, StatusCode};
use reflectapi::dispatch::{Dispatcher, Reply};
use reflectapi::{Builder, Empty, Input, Output};

#[derive(serde::Deserialize, Input)]
struct ListRequest {
    #[serde(default)]
    limit: Option<u8>,
}

#[derive(serde::Serialize, Output)]
struct Page {
    items: Vec<String>,
}

async fn list(_state: (), request: ListRequest, _headers: Empty) -> Page {
    let items = vec!["Bobby".to_string(), "Max".to_string()];
    let limit = request.limit.unwrap_or(u8::MAX) as usize;
    Page { items: items.into_iter().take(limit).collect() }
}

/// Sends one request and returns the status, the `content-type` or `allow`
/// header, and the body.
async fn call(
    dispatcher: &Dispatcher<()>,
    method: Method,
    path: &str,
    content_type: &str,
    body: &str,
) -> (StatusCode, String, String) {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", content_type.parse().unwrap());
    let Some(Reply::Complete { status, headers, body }) = dispatcher
        .dispatch((), &method, path, &headers, body.to_owned().into())
        .await
    else {
        panic!("not a complete reply");
    };
    let header = headers
        .get("content-type")
        .or(headers.get("allow"))
        .map(|value| value.to_str().unwrap().to_owned())
        .unwrap_or_default();
    (status, header, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::main]
async fn main() {
    let (_schema, routers) = Builder::new()
        .path("/api")
        .route(list, |b| b.name("pets.list").readonly(true))
        .build()
        .unwrap();
    let dispatcher = Dispatcher::new(routers);
    let json = "application/json";

    assert_eq!(
        call(&dispatcher, Method::POST, "/api/pets.list", json, r#"{"limit":1}"#).await,
        (StatusCode::OK, json.into(), r#"{"items":["Bobby"]}"#.into()),
    );
    // A readonly route answers GET; no body reads as `{}`.
    assert_eq!(
        call(&dispatcher, Method::GET, "/api/pets.list", json, "").await,
        (StatusCode::OK, json.into(), r#"{"items":["Bobby","Max"]}"#.into()),
    );
    assert_eq!(
        call(&dispatcher, Method::DELETE, "/api/pets.list", json, "").await,
        (StatusCode::METHOD_NOT_ALLOWED, "GET,POST".into(), "".into()),
    );
    let (status, _, _) = call(&dispatcher, Method::POST, "/api/pets.list", json, "[").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _, _) = call(&dispatcher, Method::POST, "/api/pets.list", "text/plain", "").await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    // No route at this path: the server's own 404, or its other routes.
    assert!(dispatcher
        .dispatch((), &Method::POST, "/pets.list", &HeaderMap::new(), Default::default())
        .await
        .is_none());
}
```
