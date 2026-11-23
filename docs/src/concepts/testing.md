# Testing

An API built with reflectapi is plain Rust: async handler functions, serde
types and a [`Builder`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html). Each part can be tested on its own, most of it without
a server or a network:

- the [handlers](#handlers), as functions;
- the [routes](#routes), through the dispatcher or the axum router;
- the API through a [generated client](#generated-clients), in-process or
  against a test server, and code that calls the API, with a fake transport;
- the [schema and the generated clients](#the-schema-and-the-generated-clients),
  by regenerating them in CI;
- the [API definition](#build-time-checks), by building it in a test.

The examples on this page test this API:

```rust
use std::sync::{Arc, Mutex};

use reflectapi::{Builder, Input, Output};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Store {
    names: Mutex<Vec<String>>,
}

#[derive(Serialize, Output)]
pub struct User {
    pub id: u32,
    pub name: String,
}

#[derive(Deserialize, Input)]
pub struct UsersCreateRequest {
    pub name: String,
}

#[derive(Deserialize, Input)]
pub struct UsersGetRequest {
    pub id: u32,
}

#[derive(Debug, Serialize, Output, reflectapi::ResponseError)]
pub enum UsersGetError {
    #[reflectapi(status = 404)]
    NotFound,
}

pub async fn users_create(
    state: Arc<Store>,
    request: UsersCreateRequest,
    _headers: reflectapi::Empty,
) -> User {
    let mut names = state.names.lock().unwrap();
    let id = names.len() as u32;
    names.push(request.name.clone());
    User { id, name: request.name }
}

pub async fn users_get(
    state: Arc<Store>,
    request: UsersGetRequest,
    _headers: reflectapi::Empty,
) -> Result<User, UsersGetError> {
    let names = state.names.lock().unwrap();
    let name = names.get(request.id as usize).ok_or(UsersGetError::NotFound)?;
    Ok(User { id: request.id, name: name.clone() })
}

pub fn api() -> Builder<Arc<Store>> {
    Builder::new()
        .name("Users")
        .route(users_create, |b| b.name("users.create").description("Create a user"))
        .route(users_get, |b| {
            b.name("users.get").description("Get a user by id").readonly(true)
        })
}
# fn main() {}
```

## Handlers

A handler is an async function of the state, the input and the request
headers. Call it directly:

```rust
# // Doctests are not built as tests: run `#[tokio::test]` functions with `#[tokio::main]`.
# mod tokio { pub use ::tokio::*; pub use ::tokio::main as test; }
# use std::sync::{Arc, Mutex};
# use reflectapi::{Builder, Input, Output};
# use serde::{Deserialize, Serialize};
# #[derive(Default)]
# pub struct Store { names: Mutex<Vec<String>> }
# #[derive(Serialize, Output)]
# pub struct User { pub id: u32, pub name: String }
# #[derive(Deserialize, Input)]
# pub struct UsersCreateRequest { pub name: String }
# #[derive(Deserialize, Input)]
# pub struct UsersGetRequest { pub id: u32 }
# #[derive(Debug, Serialize, Output, reflectapi::ResponseError)]
# pub enum UsersGetError {
#     #[reflectapi(status = 404)]
#     NotFound,
# }
# pub async fn users_create(state: Arc<Store>, request: UsersCreateRequest, _headers: reflectapi::Empty) -> User {
#     let mut names = state.names.lock().unwrap();
#     let id = names.len() as u32;
#     names.push(request.name.clone());
#     User { id, name: request.name }
# }
# pub async fn users_get(state: Arc<Store>, request: UsersGetRequest, _headers: reflectapi::Empty) -> Result<User, UsersGetError> {
#     let names = state.names.lock().unwrap();
#     let name = names.get(request.id as usize).ok_or(UsersGetError::NotFound)?;
#     Ok(User { id: request.id, name: name.clone() })
# }
#[tokio::test]
async fn get_finds_a_created_user() {
    let state = Arc::new(Store::default());
    let request = UsersCreateRequest { name: "Ada".into() };
    let created = users_create(state.clone(), request, reflectapi::Empty {}).await;

    let request = UsersGetRequest { id: created.id };
    let found = users_get(state.clone(), request, reflectapi::Empty {}).await;
    assert_eq!(found.unwrap().name, "Ada");

    let missing = users_get(state, UsersGetRequest { id: 7 }, reflectapi::Empty {}).await;
    let error = missing.err().unwrap();
    assert!(matches!(error, UsersGetError::NotFound));
    assert_eq!(
        reflectapi::ResponseError::status(&error),
        http::StatusCode::NOT_FOUND
    );
}
# fn main() { get_finds_a_created_user(); }
```

This tests the logic, and the status each [error](./errors.md) is answered
with. It skips what happens on the [wire](./wire.md): decoding the request,
encoding the response, and the request headers the handler is given.

## Routes

[`reflectapi::dispatch::Dispatcher`](https://docs.rs/reflectapi/latest/reflectapi/dispatch/struct.Dispatcher.html)
routes a request to the handlers of the
built API the same way the axum adapter does, without a server: give it the
method, the path, the headers and the body, and it returns the reply. See
[Integrating Other Servers](../serving/other-servers.md#the-contract) for what
it answers.

```rust
# // Doctests are not built as tests: run `#[tokio::test]` functions with `#[tokio::main]`.
# mod tokio { pub use ::tokio::*; pub use ::tokio::main as test; }
# use std::sync::{Arc, Mutex};
# use reflectapi::{Builder, Input, Output};
# use serde::{Deserialize, Serialize};
# #[derive(Default)]
# pub struct Store { names: Mutex<Vec<String>> }
# #[derive(Serialize, Output)]
# pub struct User { pub id: u32, pub name: String }
# #[derive(Deserialize, Input)]
# pub struct UsersCreateRequest { pub name: String }
# #[derive(Deserialize, Input)]
# pub struct UsersGetRequest { pub id: u32 }
# #[derive(Debug, Serialize, Output, reflectapi::ResponseError)]
# pub enum UsersGetError {
#     #[reflectapi(status = 404)]
#     NotFound,
# }
# pub async fn users_create(state: Arc<Store>, request: UsersCreateRequest, _headers: reflectapi::Empty) -> User {
#     let mut names = state.names.lock().unwrap();
#     let id = names.len() as u32;
#     names.push(request.name.clone());
#     User { id, name: request.name }
# }
# pub async fn users_get(state: Arc<Store>, request: UsersGetRequest, _headers: reflectapi::Empty) -> Result<User, UsersGetError> {
#     let names = state.names.lock().unwrap();
#     let name = names.get(request.id as usize).ok_or(UsersGetError::NotFound)?;
#     Ok(User { id: request.id, name: name.clone() })
# }
# pub fn api() -> Builder<Arc<Store>> {
#     Builder::new()
#         .name("Users")
#         .route(users_create, |b| b.name("users.create").description("Create a user"))
#         .route(users_get, |b| b.name("users.get").description("Get a user by id").readonly(true))
# }
use reflectapi::dispatch::{Dispatcher, Reply};

#[tokio::test]
async fn get_answers_404_for_an_unknown_user() {
    let (_schema, routers) = api().build().unwrap();
    let dispatcher = Dispatcher::new(routers);

    let reply = dispatcher
        .dispatch(
            Arc::new(Store::default()),
            &http::Method::POST,
            "/users.get",
            &http::HeaderMap::new(),
            r#"{"id":7}"#.into(),
        )
        .await
        .expect("a route at /users.get");

    let Reply::Complete { status, body, .. } = reply else {
        panic!("users.get is not a stream route");
    };
    assert_eq!(status, http::StatusCode::NOT_FOUND);
    assert_eq!(body, r#""NotFound""#);
}
# fn main() { get_answers_404_for_an_unknown_user(); }
```

- Each route is at `{path}/{name}`. `dispatch` returns `None` for a path with
  no route, and a `405` reply for a method the route does not accept.
- A body without a `content-type` header is read as JSON. A body or headers
  that don't decode get a `400` reply, with the serde error in the body.
- Pass the request headers the route reads in the `HeaderMap`; the handler is
  given only those.
- A stream route replies with `Reply::Stream`: collect its items, each an
  `Ok` of an `Event` whose `data` is the serialized item.
- The routers of a [`DispatchOnly`](https://docs.rs/reflectapi/latest/reflectapi/struct.DispatchOnly.html) build, which a server may use (see
  [build modes](../serving/other-servers.md#build-modes)), dispatch the same
  way.

### Through axum

Middleware added in the callback of [`reflectapi::axum::into_router`](https://docs.rs/reflectapi/latest/reflectapi/axum/fn.into_router.html), such as
authentication, runs only in the axum router. To test it too, send requests
to the router with `tower::ServiceExt::oneshot`. It needs `tower` 0.5 with
its `util` feature as a dev-dependency:

```rust
# // Doctests are not built as tests: run `#[tokio::test]` functions with `#[tokio::main]`.
# mod tokio { pub use ::tokio::*; pub use ::tokio::main as test; }
# use std::sync::{Arc, Mutex};
# use reflectapi::{Builder, Input, Output};
# use serde::{Deserialize, Serialize};
# #[derive(Default)]
# pub struct Store { names: Mutex<Vec<String>> }
# #[derive(Serialize, Output)]
# pub struct User { pub id: u32, pub name: String }
# #[derive(Deserialize, Input)]
# pub struct UsersCreateRequest { pub name: String }
# #[derive(Deserialize, Input)]
# pub struct UsersGetRequest { pub id: u32 }
# #[derive(Debug, Serialize, Output, reflectapi::ResponseError)]
# pub enum UsersGetError {
#     #[reflectapi(status = 404)]
#     NotFound,
# }
# pub async fn users_create(state: Arc<Store>, request: UsersCreateRequest, _headers: reflectapi::Empty) -> User {
#     let mut names = state.names.lock().unwrap();
#     let id = names.len() as u32;
#     names.push(request.name.clone());
#     User { id, name: request.name }
# }
# pub async fn users_get(state: Arc<Store>, request: UsersGetRequest, _headers: reflectapi::Empty) -> Result<User, UsersGetError> {
#     let names = state.names.lock().unwrap();
#     let name = names.get(request.id as usize).ok_or(UsersGetError::NotFound)?;
#     Ok(User { id: request.id, name: name.clone() })
# }
# pub fn api() -> Builder<Arc<Store>> {
#     Builder::new()
#         .name("Users")
#         .route(users_create, |b| b.name("users.create").description("Create a user"))
#         .route(users_get, |b| b.name("users.get").description("Get a user by id").readonly(true))
# }
use tower::ServiceExt;

#[tokio::test]
async fn get_answers_404_for_an_unknown_user() {
    let (_schema, routers) = api().build().unwrap();
    let app = reflectapi::axum::into_router(
        Arc::new(Store::default()),
        routers,
        reflectapi::axum::RouterSettings::default(),
        |_name, router| router, // add the server's middleware here
    );

    let request = http::Request::post("/users.get")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(r#"{"id":7}"#))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), http::StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(body, r#""NotFound""#);
}
# fn main() { get_answers_404_for_an_unknown_user(); }
```

Build the router in a function that the server and the tests share, so the
tests run the server's middleware.

## Generated clients

### Rust

The generated Rust client sends its requests through a transport, any
`Clone` type that implements
[`reflectapi::rt::Client`](https://docs.rs/reflectapi/latest/reflectapi/rt/trait.Client.html).
A transport that dispatches to the routes calls the API in-process: the
request goes through the client's serialization, the routes and the client's
deserialization, with no server.

```rust
# // Doctests are not built as tests: run `#[tokio::test]` functions with `#[tokio::main]`.
# mod tokio { pub use ::tokio::*; pub use ::tokio::main as test; }
# mod server {
#     use std::sync::{Arc, Mutex};
#     use reflectapi::{Builder, Input, Output};
#     use serde::{Deserialize, Serialize};
#     #[derive(Default)]
#     pub struct Store { names: Mutex<Vec<String>> }
#     #[derive(Serialize, Output)]
#     pub struct User { pub id: u32, pub name: String }
#     #[derive(Deserialize, Input)]
#     pub struct UsersCreateRequest { pub name: String }
#     #[derive(Deserialize, Input)]
#     pub struct UsersGetRequest { pub id: u32 }
#     #[derive(Debug, Serialize, Output, reflectapi::ResponseError)]
#     pub enum UsersGetError {
#         #[reflectapi(status = 404)]
#         NotFound,
#     }
#     pub async fn users_create(state: Arc<Store>, request: UsersCreateRequest, _headers: reflectapi::Empty) -> User {
#         let mut names = state.names.lock().unwrap();
#         let id = names.len() as u32;
#         names.push(request.name.clone());
#         User { id, name: request.name }
#     }
#     pub async fn users_get(state: Arc<Store>, request: UsersGetRequest, _headers: reflectapi::Empty) -> Result<User, UsersGetError> {
#         let names = state.names.lock().unwrap();
#         let name = names.get(request.id as usize).ok_or(UsersGetError::NotFound)?;
#         Ok(User { id: request.id, name: name.clone() })
#     }
#     pub fn api() -> Builder<Arc<Store>> {
#         Builder::new()
#             .name("Users")
#             .route(users_create, |b| b.name("users.create").description("Create a user"))
#             .route(users_get, |b| b.name("users.get").description("Get a user by id").readonly(true))
#     }
# }
# use server::{api, Store};
# /// The output of `reflectapi codegen --language rust` for the API.
# mod users_client {
#     #![allow(dead_code, unexpected_cfgs, unused_imports)]
#     pub use interface::Interface;
#     pub use reflectapi::rt::*;
#     pub mod interface {
#         #[derive(Debug)]
#         pub struct Interface<C: reflectapi::rt::Client + Clone> {
#             pub users: UsersInterface<C>,
#             client: C,
#         }
#         impl<C: reflectapi::rt::Client + Clone> Interface<C> {
#             pub fn new(client: C) -> Self {
#                 Self { users: UsersInterface::new(client.clone()), client }
#             }
#         }
#         #[derive(Debug)]
#         pub struct UsersInterface<C: reflectapi::rt::Client + Clone> {
#             client: C,
#         }
#         impl<C: reflectapi::rt::Client + Clone> UsersInterface<C> {
#             pub fn new(client: C) -> Self {
#                 Self { client }
#             }
#             /// Create a user
#             pub async fn create(
#                 &self,
#                 input: super::types::my_api::UsersCreateRequest,
#                 headers: reflectapi::Empty,
#             ) -> Result<super::types::my_api::User, reflectapi::rt::Error<reflectapi::Empty, C::Error>>
#             {
#                 reflectapi::rt::__request_impl(&self.client, "/users.create", input, headers).await
#             }
#             /// Get a user by id
#             pub async fn get(
#                 &self,
#                 input: super::types::my_api::UsersGetRequest,
#                 headers: reflectapi::Empty,
#             ) -> Result<
#                 super::types::my_api::User,
#                 reflectapi::rt::Error<super::types::my_api::UsersGetError, C::Error>,
#             > {
#                 reflectapi::rt::__request_impl(&self.client, "/users.get", input, headers).await
#             }
#         }
#     }
#     pub mod types {
#         pub mod my_api {
#             #[derive(Debug, serde::Deserialize)]
#             pub struct User {
#                 pub id: u32,
#                 pub name: std::string::String,
#             }
#             #[derive(Debug, serde::Serialize)]
#             pub struct UsersCreateRequest {
#                 pub name: std::string::String,
#             }
#             #[derive(Debug, serde::Deserialize, serde::Serialize)]
#             pub enum UsersGetError {
#                 NotFound,
#             }
#             impl std::fmt::Display for UsersGetError {
#                 fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
#                     write!(f, "{}", reflectapi::rt::error_to_string(self))
#                 }
#             }
#             impl std::error::Error for UsersGetError {}
#             #[derive(Debug, serde::Serialize)]
#             pub struct UsersGetRequest {
#                 pub id: u32,
#             }
#         }
#     }
# }
use std::sync::Arc;

use futures_util::StreamExt;
use reflectapi::dispatch::{sse_frames, Dispatcher, Reply};
use reflectapi::rt::{Request, Response, Url};

/// A transport that dispatches requests to the routes, in-process.
#[derive(Clone)]
struct InProcess<S: Send + 'static> {
    dispatcher: Arc<Dispatcher<S>>,
    state: S,
    base_url: Url,
}

impl<S: Send + 'static> InProcess<S> {
    fn new(routers: Vec<reflectapi::Router<S>>, state: S) -> Self {
        Self {
            dispatcher: Arc::new(Dispatcher::new(routers)),
            state,
            // Required by the trait, and unused.
            base_url: "http://localhost".parse().unwrap(),
        }
    }
}

impl<S: Clone + Send + 'static> reflectapi::rt::Client for InProcess<S> {
    type Error = String;

    fn base_url(&self) -> &Url {
        &self.base_url
    }

    async fn request(&self, request: Request) -> Result<Response<String>, String> {
        let reply = self
            .dispatcher
            .dispatch(
                self.state.clone(),
                &http::Method::POST,
                &request.path,
                &request.headers,
                request.body,
            )
            .await
            .ok_or_else(|| format!("no route at {}", request.path))?;
        Ok(match reply {
            Reply::Complete { status, headers, body } => Response {
                status,
                headers,
                body: Box::pin(futures_util::stream::once(async { Ok(body) })),
            },
            // A stream route: its items as Server-Sent Events, as a server sends them.
            Reply::Stream(items) => Response {
                status: http::StatusCode::OK,
                headers: http::HeaderMap::new(),
                body: Box::pin(sse_frames(items).map(|frame| frame.map(Into::into))),
            },
        })
    }
}

use users_client::types::my_api::{UsersCreateRequest, UsersGetError, UsersGetRequest};

#[tokio::test]
async fn get_finds_a_created_user() {
    let (_schema, routers) = api().build().unwrap();
    let client = users_client::Interface::new(InProcess::new(routers, Arc::new(Store::default())));

    let request = UsersCreateRequest { name: "Ada".into() };
    let created = client.users.create(request, reflectapi::Empty {}).await.unwrap();

    let request = UsersGetRequest { id: created.id };
    let found = client.users.get(request, reflectapi::Empty {}).await.unwrap();
    assert_eq!(found.name, "Ada");

    let missing = client.users.get(UsersGetRequest { id: 7 }, reflectapi::Empty {}).await;
    assert!(matches!(
        missing,
        Err(reflectapi::rt::Error::Application(UsersGetError::NotFound))
    ));
}
# fn main() { get_finds_a_created_user(); }
```

To test against the server as it runs, with its axum middleware, serve it on
a free local port and use the built-in `reqwest` transport:

```rust
# // Doctests are not built as tests: run `#[tokio::test]` functions with `#[tokio::main]`.
# mod tokio { pub use ::tokio::*; pub use ::tokio::main as test; }
# mod server {
#     use std::sync::{Arc, Mutex};
#     use reflectapi::{Builder, Input, Output};
#     use serde::{Deserialize, Serialize};
#     #[derive(Default)]
#     pub struct Store { names: Mutex<Vec<String>> }
#     #[derive(Serialize, Output)]
#     pub struct User { pub id: u32, pub name: String }
#     #[derive(Deserialize, Input)]
#     pub struct UsersCreateRequest { pub name: String }
#     #[derive(Deserialize, Input)]
#     pub struct UsersGetRequest { pub id: u32 }
#     #[derive(Debug, Serialize, Output, reflectapi::ResponseError)]
#     pub enum UsersGetError {
#         #[reflectapi(status = 404)]
#         NotFound,
#     }
#     pub async fn users_create(state: Arc<Store>, request: UsersCreateRequest, _headers: reflectapi::Empty) -> User {
#         let mut names = state.names.lock().unwrap();
#         let id = names.len() as u32;
#         names.push(request.name.clone());
#         User { id, name: request.name }
#     }
#     pub async fn users_get(state: Arc<Store>, request: UsersGetRequest, _headers: reflectapi::Empty) -> Result<User, UsersGetError> {
#         let names = state.names.lock().unwrap();
#         let name = names.get(request.id as usize).ok_or(UsersGetError::NotFound)?;
#         Ok(User { id: request.id, name: name.clone() })
#     }
#     pub fn api() -> Builder<Arc<Store>> {
#         Builder::new()
#             .name("Users")
#             .route(users_create, |b| b.name("users.create").description("Create a user"))
#             .route(users_get, |b| b.name("users.get").description("Get a user by id").readonly(true))
#     }
# }
# use server::{api, Store};
# /// The output of `reflectapi codegen --language rust` for the API.
# mod users_client {
#     #![allow(dead_code, unexpected_cfgs, unused_imports)]
#     pub use interface::Interface;
#     pub use reflectapi::rt::*;
#     pub mod interface {
#         #[derive(Debug)]
#         pub struct Interface<C: reflectapi::rt::Client + Clone> {
#             pub users: UsersInterface<C>,
#             client: C,
#         }
#         impl<C: reflectapi::rt::Client + Clone> Interface<C> {
#             pub fn new(client: C) -> Self {
#                 Self { users: UsersInterface::new(client.clone()), client }
#             }
#         }
#         #[derive(Debug)]
#         pub struct UsersInterface<C: reflectapi::rt::Client + Clone> {
#             client: C,
#         }
#         impl<C: reflectapi::rt::Client + Clone> UsersInterface<C> {
#             pub fn new(client: C) -> Self {
#                 Self { client }
#             }
#             /// Create a user
#             pub async fn create(
#                 &self,
#                 input: super::types::my_api::UsersCreateRequest,
#                 headers: reflectapi::Empty,
#             ) -> Result<super::types::my_api::User, reflectapi::rt::Error<reflectapi::Empty, C::Error>>
#             {
#                 reflectapi::rt::__request_impl(&self.client, "/users.create", input, headers).await
#             }
#             /// Get a user by id
#             pub async fn get(
#                 &self,
#                 input: super::types::my_api::UsersGetRequest,
#                 headers: reflectapi::Empty,
#             ) -> Result<
#                 super::types::my_api::User,
#                 reflectapi::rt::Error<super::types::my_api::UsersGetError, C::Error>,
#             > {
#                 reflectapi::rt::__request_impl(&self.client, "/users.get", input, headers).await
#             }
#         }
#     }
#     pub mod types {
#         pub mod my_api {
#             #[derive(Debug, serde::Deserialize)]
#             pub struct User {
#                 pub id: u32,
#                 pub name: std::string::String,
#             }
#             #[derive(Debug, serde::Serialize)]
#             pub struct UsersCreateRequest {
#                 pub name: std::string::String,
#             }
#             #[derive(Debug, serde::Deserialize, serde::Serialize)]
#             pub enum UsersGetError {
#                 NotFound,
#             }
#             impl std::fmt::Display for UsersGetError {
#                 fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
#                     write!(f, "{}", reflectapi::rt::error_to_string(self))
#                 }
#             }
#             impl std::error::Error for UsersGetError {}
#             #[derive(Debug, serde::Serialize)]
#             pub struct UsersGetRequest {
#                 pub id: u32,
#             }
#         }
#     }
# }
use std::sync::Arc;

use reflectapi::rt::{ReqwestClient, Url};
use users_client::types::my_api::{UsersCreateRequest, UsersGetRequest};

/// Serves the API on a free local port, and returns its URL.
async fn serve(state: Arc<Store>) -> Url {
    let (_schema, routers) = api().build().unwrap();
    let app = reflectapi::axum::into_router(
        state,
        routers,
        reflectapi::axum::RouterSettings::default(),
        |_name, router| router,
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    url.parse().unwrap()
}

#[tokio::test]
async fn get_finds_a_created_user_over_http() {
    let url = serve(Arc::new(Store::default())).await;
    let transport = ReqwestClient::try_new(reqwest::Client::new(), url).unwrap();
    let client = users_client::Interface::new(transport);

    let request = UsersCreateRequest { name: "Ada".into() };
    let created = client.users.create(request, reflectapi::Empty {}).await.unwrap();

    let request = UsersGetRequest { id: created.id };
    let found = client.users.get(request, reflectapi::Empty {}).await.unwrap();
    assert_eq!(found.name, "Ada");
}
# fn main() { get_finds_a_created_user_over_http(); }
```

A generated client crate with its `reqwest` feature also has
`Interface::try_new(reqwest::Client, Url)`; see
[Client Libraries](../clients/index.md#rust).

### TypeScript

`client()` takes a base URL or a transport: any object with the
`request(request): Promise<Response>` method of the `Client` interface in
`generated.transport.ts`. A fake transport tests code that calls the API
without a server. The `Response` returned only needs `status`, `headers.get`
and `body`, so the global `Response` of `fetch` works:

```typescript
import { strict as assert } from "node:assert";
import { test } from "node:test";

import { client } from "./generated";
import type { Client, Request } from "./generated.transport";

/** Answers every request with `status` and `body`, and keeps the requests. */
function fakeServer(status: number, body: unknown) {
  const requests: Request[] = [];
  const transport: Client = {
    async request(request) {
      requests.push(request);
      return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
      });
    },
  };
  return { transport, requests };
}

test("get finds a user", async () => {
  const server = fakeServer(200, { id: 0, name: "Ada" });

  const user = (await client(server.transport).users.get({ id: 0 }, {})).unwrap_ok();

  assert.equal(user.name, "Ada");
  assert.equal(server.requests[0]?.path, "/users.get");
});

test("get reports an unknown user", async () => {
  const server = fakeServer(404, "NotFound");

  const result = await client(server.transport).users.get({ id: 7 }, {});

  assert.equal(result.unwrap_err().err(), "NotFound");
});
```

### Python

The generated clients take a transport in their `client` argument: an
`httpx.Client` (`httpx.AsyncClient` for `AsyncClient`), or any object with a
`request` method that takes a `reflectapi_runtime.transport.Request` and
returns a `Response` (an `async` method for `AsyncClient`):

```python
import json

import pytest
from reflectapi_runtime import ApplicationError
from reflectapi_runtime.transport import Request, Response

from api_client import Client
from api_client.my_api import UsersGetRequest


class FakeServer:
    """Answers every request with `status` and `body`, and keeps the requests."""

    def __init__(self, status: int, body: object) -> None:
        self.status = status
        self.body = json.dumps(body).encode()
        self.requests: list[Request] = []

    def request(self, request: Request) -> Response:
        self.requests.append(request)
        return Response(
            status=self.status,
            headers={"content-type": "application/json"},
            body=self.body,
        )


def test_get_finds_a_user() -> None:
    server = FakeServer(200, {"id": 0, "name": "Ada"})
    client = Client("http://test", client=server)

    user = client.users.get(UsersGetRequest(id=0)).value

    assert user.name == "Ada"
    assert server.requests[0].path == "/users.get"


def test_get_reports_an_unknown_user() -> None:
    client = Client("http://test", client=FakeServer(404, "NotFound"))

    with pytest.raises(ApplicationError) as raised:
        client.users.get(UsersGetRequest(id=7))
    assert raised.value.status_code == 404
```

- Streaming methods send through `httpx` directly, so they need an `httpx`
  client. To fake one, give it an `httpx.MockTransport`:
  `Client("http://test", client=httpx.Client(transport=httpx.MockTransport(handle)))`.
- To stub the client itself, `reflectapi_runtime.testing.create_api_response(value)`
  wraps a model in the `ApiResponse` a method returns, with status `200`
  unless given `status_code`. `reflectapi codegen --python-testing` also
  writes `testing.py` into the package, with a `create_<type>_response`
  function for each type.

## The schema and the generated clients

Commit the schema and the generated clients, and have CI regenerate them and
fail on a difference, so an API change cannot merge without its clients.
[Generate the schema and clients at build time](../serving/workers.md#generate-the-schema-and-clients-at-build-time)
does this from a test; CI then runs the tests and `git diff --exit-code`.
The diff of a pull request then shows what each client gains or loses, next
to the Rust change that caused it.

reflectapi's own tests take snapshots instead, with
[insta](https://insta.rs): `insta::assert_json_snapshot!(schema)` for the
schema, and `insta::assert_snapshot!` for the code each generator writes.
Compile the committed clients in CI too, with the projects that use them.

## Build-time checks

Some mistakes are caught before a test runs:

- **The derives** reject invalid attributes as the crate compiles, such as
  `#[reflectapi(hidden)]` on a tuple field, a malformed deprecation date
  ([Deprecations](../getting-started/deprecations.md#declaring)), or a status
  a route's error must not give
  ([A route's error](../getting-started/attributes.md#a-routes-error)).
- **[`Builder::route`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.route)** requires its handler's input, output, error and
  headers types to implement the reflectapi traits, so a type that doesn't
  is a compile error.
- **[`Builder::build`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.build)** returns
  [`BuildErrors`](https://docs.rs/reflectapi/latest/reflectapi/struct.BuildErrors.html)
  for what it checks at run time: an invalid request header name, malformed
  route deprecation dates, invalid response headers, `sse_keep_alive` on a
  route that isn't a stream route, `#[serde(flatten)]` on a
  [`reflectapi::Option`](https://docs.rs/reflectapi/latest/reflectapi/enum.Option.html) field, a [`rename_types`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.rename_types) pattern that matches no type,
  and the errors of custom validators.

[`Builder::validate`](https://docs.rs/reflectapi/latest/reflectapi/struct.Builder.html#method.validate)
adds a custom validator: a function of the schema that returns the problems
it finds. Use it for the API's own conventions:

```rust
# use std::sync::{Arc, Mutex};
# use reflectapi::{Builder, Input, Output};
# use serde::{Deserialize, Serialize};
# #[derive(Default)]
# pub struct Store { names: Mutex<Vec<String>> }
# #[derive(Serialize, Output)]
# pub struct User { pub id: u32, pub name: String }
# #[derive(Deserialize, Input)]
# pub struct UsersCreateRequest { pub name: String }
# #[derive(Deserialize, Input)]
# pub struct UsersGetRequest { pub id: u32 }
# #[derive(Debug, Serialize, Output, reflectapi::ResponseError)]
# pub enum UsersGetError {
#     #[reflectapi(status = 404)]
#     NotFound,
# }
# pub async fn users_create(state: Arc<Store>, request: UsersCreateRequest, _headers: reflectapi::Empty) -> User {
#     let mut names = state.names.lock().unwrap();
#     let id = names.len() as u32;
#     names.push(request.name.clone());
#     User { id, name: request.name }
# }
# pub async fn users_get(state: Arc<Store>, request: UsersGetRequest, _headers: reflectapi::Empty) -> Result<User, UsersGetError> {
#     let names = state.names.lock().unwrap();
#     let name = names.get(request.id as usize).ok_or(UsersGetError::NotFound)?;
#     Ok(User { id: request.id, name: name.clone() })
# }
use reflectapi::{Schema, ValidationError, ValidationPointer};

/// Every route needs a description, for the generated clients' documentation.
fn every_route_has_a_description(schema: &Schema) -> Vec<ValidationError> {
    schema
        .functions()
        .filter(|function| function.description().is_empty())
        .map(|function| {
            ValidationError::new(
                ValidationPointer::Function(function.name().to_string()),
                "the route has no description".into(),
            )
        })
        .collect()
}

pub fn api() -> Builder<Arc<Store>> {
    Builder::new()
        .name("Users")
        .route(users_create, |b| b.name("users.create").description("Create a user"))
        .route(users_get, |b| b.name("users.get")) // no description
        .validate(every_route_has_a_description)
}

#[test]
fn the_api_builds() {
    if let Err(errors) = api().build() {
        panic!("{errors}");
    }
}
# fn main() {
#     let errors = api().build().err().expect("users.get has no description");
#     assert_eq!(errors.to_string(), "users.get: the route has no description\n");
# }
```

Here `the_api_builds` fails with `users.get: the route has no description`.

Validators and the other schema checks run only when the API is built
[`WithSchema`](https://docs.rs/reflectapi/latest/reflectapi/struct.WithSchema.html). A server that builds it `DispatchOnly` skips them (see
[build modes](../serving/other-servers.md#build-modes)), so keep a test that
builds it `WithSchema`, such as `the_api_builds` above or the test that
generates the clients.
