# Serving with celld and Cloudflare Workers

A reflectapi API can run in a Cloudflare-compatible Worker, on
[Cloudflare Workers](https://developers.cloudflare.com/workers/) or on
[celld](https://celld.dev), written in Rust with
[workers-rs](https://github.com/cloudflare/workers-rs) and compiled to
WebAssembly. The Worker runtime is the HTTP server, so axum is not used: the
`reflectapi::dispatch::Dispatcher` routes requests, and the adapter on this
page connects it to workers-rs.

## Dependencies

```toml
[lib]
crate-type = ["cdylib", "rlib"] # rlib: the native build step below links the crate

[dependencies]
reflectapi = { version = "...", default-features = false, features = ["builder", "json"] }
worker = "0.8"
http = "1"
futures-util = "0.3"

# Generating the schema and clients happens natively, at build time.
[target.'cfg(not(target_arch = "wasm32"))'.dev-dependencies]
reflectapi = { version = "...", features = ["builder", "json", "codegen"] }

[profile.release]
lto = true
codegen-units = 1
# Not `strip = true` (the same as "symbols"): wasm-bindgen then fails with
# "externref table required for catch wrappers". `strip = "debuginfo"` is fine.
```

Build with `worker-build --release`, as for any workers-rs Worker.

## Define the API

Write the definition once, generic over the [build mode](./other-servers.md#build-modes):
the Worker builds it with `DispatchOnly`, so no schema is built or linked into
the wasm, and the build step builds it with `WithSchema`.

```rust,ignore
pub fn api<M: reflectapi::BuildMode>() -> reflectapi::Builder<Ctx, M> {
    reflectapi::Builder::default()
        .name("Routing agent")
        .stream_route(prompt, |b| b.name("agent.prompt"))
}
```

Handlers, their futures and their streams must be `Send`, but values backed by
JavaScript objects (storage, bindings, fetch responses) are not. A Worker runs
on one thread, so marking them `Send` is sound: wrap state in
`worker::send::SendWrapper`, futures in `worker::send::SendFuture`, and streams
in the `SendStream` from the adapter below.

```rust,ignore
pub type Ctx = worker::send::SendWrapper<std::rc::Rc<SessionState>>;

fn prompt(
    ctx: Ctx,
    request: PromptRequest,
    headers: SessionHeaders,
) -> Result<SendStream<impl Stream<Item = AgentEvent>>, PromptError> {
    let session = ctx.0.clone();
    Ok(SendStream::new(async_stream::stream! {
        // read and write `session`, await fetches and timers, yield events
    }))
}
```

## The adapter

Copy this module into your Worker crate.

```rust,ignore
//! Serves a reflectapi `Dispatcher` from a Cloudflare-compatible Worker or Durable Object.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::{Stream, StreamExt};
use reflectapi::dispatch::{sse_event, Dispatcher, Reply};
use worker::{send::SendWrapper, Error, Headers, Request, Response, Result};

/// Dispatches one Worker request and converts the reply to a Worker response.
pub async fn serve<S>(dispatcher: &Dispatcher<S>, state: S, mut req: Request) -> Result<Response>
where
    S: Send + Clone + 'static,
{
    let method = http::Method::from_bytes(req.method().as_ref().as_bytes())
        .map_err(|err| Error::RustError(err.to_string()))?;
    let mut headers = http::HeaderMap::new();
    for (name, value) in req.headers().entries() {
        let name = http::HeaderName::from_bytes(name.as_bytes())
            .map_err(|err| Error::RustError(err.to_string()))?;
        let value =
            http::HeaderValue::from_str(&value).map_err(|err| Error::RustError(err.to_string()))?;
        headers.append(name, value);
    }
    let body = req.bytes().await?;

    match dispatcher
        .dispatch(state, &method, &req.path(), &headers, body.into())
        .await
    {
        None => Response::error("Not Found", 404),
        Some(Reply::Complete {
            status,
            headers,
            body,
        }) => Ok(Response::from_bytes(body.to_vec())?
            .with_status(status.as_u16())
            .with_headers(worker_headers(&headers)?)),
        Some(Reply::Stream(items)) => {
            let events = items.map(|item| {
                item.map(|event| sse_event(&event).into_bytes())
                    .map_err(Error::RustError)
            });
            let headers = Headers::new();
            headers.set("content-type", "text/event-stream")?;
            headers.set("cache-control", "no-cache")?;
            Ok(Response::from_stream(events)?.with_headers(headers))
        }
    }
}

fn worker_headers(headers: &http::HeaderMap) -> Result<Headers> {
    let out = Headers::new();
    for (name, value) in headers {
        let value = value
            .to_str()
            .map_err(|err| Error::RustError(err.to_string()))?;
        out.append(name.as_str(), value)?;
    }
    Ok(out)
}

/// Marks a stream `Send` for reflectapi, like `worker::send::SendWrapper` does for
/// values. Sound in a Worker, which runs on one thread.
pub struct SendStream<St>(SendWrapper<Pin<Box<St>>>);

impl<St> SendStream<St> {
    pub fn new(stream: St) -> Self {
        Self(SendWrapper::new(Box::pin(stream)))
    }
}

impl<St: Stream> Stream for SendStream<St> {
    type Item = St::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<St::Item>> {
        self.get_mut().0.as_mut().poll_next(cx)
    }
}
```

## Serve from a Worker or a Durable Object

A stateless API serves from the Worker's `fetch` handler. An API with state
per entity (a session, a job) serves from a Durable Object, and the Worker
forwards each request to the object it is for. Declare the routing key as a
request header, so the schema and the generated clients carry it:

```rust,ignore
#[derive(serde::Deserialize, reflectapi::Input)]
pub struct SessionHeaders {
    #[serde(rename = "x-session-id")]
    pub session_id: String,
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    match req.headers().get("x-session-id")? {
        Some(id) if !id.is_empty() => {
            env.durable_object("SESSION")?.get_by_name(&id)?.fetch_with_request(req).await
        }
        _ => Response::error("missing x-session-id", 400),
    }
}

#[durable_object]
pub struct Session {
    ctx: Ctx,
    dispatcher: std::result::Result<Dispatcher<Ctx>, String>,
}

impl DurableObject for Session {
    fn new(state: State, _env: Env) -> Self {
        Self {
            ctx: SendWrapper::new(Rc::new(SessionState::new(state))),
            dispatcher: api::<reflectapi::DispatchOnly>()
                .build()
                .map(Dispatcher::new)
                .map_err(|errors| errors.to_string()),
        }
    }

    async fn fetch(&self, req: Request) -> Result<Response> {
        match &self.dispatcher {
            Ok(dispatcher) => reflectapi_worker::serve(dispatcher, self.ctx.clone(), req).await,
            Err(errors) => Response::error(format!("invalid API definition: {errors}"), 500),
        }
    }
}
```

The `Dispatcher` holds no request state: the state is passed to each
`dispatch` call. A Durable Object builds it when it starts; to build it once
for every object in an isolate instead, keep it in a `thread_local!`.

## Generate the schema and clients at build time

The crate also builds natively, so a native test (or binary) can build the API
`WithSchema` and write the schema and the clients:

```rust,ignore
#[cfg(all(test, not(target_arch = "wasm32")))]
#[test]
fn generate_clients() -> Result<(), Box<dyn std::error::Error>> {
    let (schema, _) = api::<reflectapi::WithSchema>().build()?;
    std::fs::write("clients/reflectapi.json", serde_json::to_string_pretty(&schema)?)?;
    let rust = reflectapi::codegen::rust::generate(schema.clone(), &Default::default())?;
    std::fs::write("clients/rust/src/generated.rs", rust)?;
    for (file, content) in reflectapi::codegen::typescript::generate(schema, &Default::default())? {
        std::fs::write(format!("clients/typescript/{file}"), content)?;
    }
    Ok(())
}
```

Commit the output, and have CI regenerate it and fail on a difference, so an
API change cannot merge without its regenerated clients. The default codegen
configurations run no formatter, so the output is the same on every machine;
enabling formatting makes it depend on the formatter version installed.
